# 23 — GPU-Accelerated Inference: Technical Plan

**Module:** GPU-Accelerated Inference (L1 + L2)
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    gpu/
      mod.rs              # GPU subsystem entry point
      detector.rs         # GPU auto-detection (nvidia-smi, rocm-smi, vulkaninfo)
      capability.rs       # GpuCapability struct, vendor abstraction
      configurator.rs     # Auto-configuration (VRAM → layers + quantization)
      monitor.rs          # VRAM monitoring (periodic polling)
      backend.rs          # llama.cpp backend selection (CUDA/ROCm/Vulkan)
    local/
      server.rs           # Modified: GPU flags for llama-server startup
      models.rs           # Modified: quantization variant management

skills/
  built-in/
    sysinfo/
      tools/
        gpu_info.yaml     # New: sys_gpu tool definition
```

### New Dependencies

```toml
# No new Rust crates needed — GPU detection uses command-line tools
# parsed via std::process::Command + serde_json for structured output
```

---

## 2. GPU Detection Module

```rust
use std::process::Command;

/// Detected GPU information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    pub vendor: GpuVendor,
    pub model: String,
    pub vram_mb: u64,
    pub vram_used_mb: u64,
    pub driver_version: String,
    pub compute_capability: Option<String>,  // CUDA compute capability (NVIDIA only)
    pub temperature_c: Option<u32>,
    pub utilization_pct: Option<u32>,
    pub power_watts: Option<u32>,
    pub power_limit_watts: Option<u32>,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

pub struct GpuDetector;

impl GpuDetector {
    /// Detect all available GPUs. Tries NVIDIA first, then AMD, then Intel.
    pub fn detect_all() -> Vec<GpuCapability> {
        let mut gpus = Vec::new();
        gpus.extend(Self::detect_nvidia());
        gpus.extend(Self::detect_amd());
        gpus.extend(Self::detect_intel());
        gpus
    }

    fn detect_nvidia() -> Vec<GpuCapability> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=index,name,memory.total,memory.used,driver_version,temperature.gpu,utilization.gpu,power.draw,power.limit",
                "--format=csv,noheader,nounits"
            ])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if fields.len() < 9 { return None; }

            Some(GpuCapability {
                vendor: GpuVendor::Nvidia,
                index: fields[0].parse().unwrap_or(0),
                model: fields[1].to_string(),
                vram_mb: fields[2].parse().unwrap_or(0),
                vram_used_mb: fields[3].parse().unwrap_or(0),
                driver_version: fields[4].to_string(),
                compute_capability: Self::get_cuda_compute(fields[0]),
                temperature_c: fields[5].parse().ok(),
                utilization_pct: fields[6].parse().ok(),
                power_watts: fields[7].parse::<f64>().ok().map(|w| w as u32),
                power_limit_watts: fields[8].parse::<f64>().ok().map(|w| w as u32),
            })
        }).collect()
    }

    fn detect_amd() -> Vec<GpuCapability> {
        let output = Command::new("rocm-smi")
            .args(["--showid", "--showmeminfo", "vram", "--showtemp", "--showuse", "--json"])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };

        // Parse rocm-smi JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(json) => Self::parse_rocm_json(&json),
            Err(_) => Vec::new(),
        }
    }

    fn detect_intel() -> Vec<GpuCapability> {
        let output = Command::new("vulkaninfo")
            .args(["--json"])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(json) => Self::parse_vulkan_json(&json),
            Err(_) => Vec::new(),
        }
    }

    fn get_cuda_compute(gpu_index: &str) -> Option<String> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=compute_cap",
                "--format=csv,noheader",
                &format!("--id={}", gpu_index),
            ])
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    fn parse_rocm_json(json: &serde_json::Value) -> Vec<GpuCapability> {
        // Parse ROCm JSON format — structure varies by rocm-smi version.
        // Extracts: model name, VRAM total/used, temperature, utilization.
        Vec::new() // Implementation depends on rocm-smi JSON schema
    }

    fn parse_vulkan_json(json: &serde_json::Value) -> Vec<GpuCapability> {
        // Parse vulkaninfo JSON — extracts device name, memory heaps.
        // Only returns Intel devices (filter by vendorID 0x8086).
        Vec::new() // Implementation depends on vulkaninfo JSON schema
    }
}
```

---

## 3. Auto-Configuration Algorithm

```rust
/// Calculates optimal GPU configuration based on available VRAM and model.
pub struct GpuConfigurator {
    vram_reserve_mb: u64,
}

pub struct GpuConfig {
    pub gpu_layers: u32,
    pub quantization: Quantization,
    pub backend: GpuBackend,
    pub main_gpu: u32,
    pub tensor_split: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Copy)]
pub enum Quantization {
    Q4KM,
    Q5KM,
    Q8_0,
}

#[derive(Debug, Clone, Copy)]
pub enum GpuBackend {
    Cuda,
    Rocm,
    Vulkan,
    Cpu,
}

/// Known model layer counts and per-layer VRAM requirements.
struct ModelProfile {
    total_layers: u32,
    vram_per_layer_mb: HashMap<Quantization, u64>,
    vram_overhead_mb: u64,  // KV cache, scratch buffers
}

impl GpuConfigurator {
    pub fn new(vram_reserve_mb: u64) -> Self {
        Self { vram_reserve_mb }
    }

    /// Calculate optimal config for the given GPUs and model.
    pub fn configure(
        &self,
        gpus: &[GpuCapability],
        model_name: &str,
    ) -> GpuConfig {
        if gpus.is_empty() {
            return GpuConfig {
                gpu_layers: 0,
                quantization: Quantization::Q4KM,
                backend: GpuBackend::Cpu,
                main_gpu: 0,
                tensor_split: None,
            };
        }

        let backend = match gpus[0].vendor {
            GpuVendor::Nvidia => GpuBackend::Cuda,
            GpuVendor::Amd => GpuBackend::Rocm,
            GpuVendor::Intel => GpuBackend::Vulkan,
            GpuVendor::Unknown => GpuBackend::Cpu,
        };

        let profile = self.model_profile(model_name);
        let available_vram = self.total_available_vram(gpus);

        // Try quantizations from highest quality to lowest
        for quant in &[Quantization::Q8_0, Quantization::Q5KM, Quantization::Q4KM] {
            let per_layer = profile.vram_per_layer_mb
                .get(quant)
                .copied()
                .unwrap_or(200);
            let total_model_vram = per_layer * profile.total_layers as u64
                + profile.vram_overhead_mb;

            if total_model_vram <= available_vram {
                // Full model fits on GPU
                let config = GpuConfig {
                    gpu_layers: profile.total_layers,
                    quantization: *quant,
                    backend,
                    main_gpu: 0,
                    tensor_split: self.compute_tensor_split(gpus),
                };
                return config;
            }

            // Partial offload — calculate how many layers fit
            let layers_that_fit = ((available_vram - profile.vram_overhead_mb) / per_layer)
                .min(profile.total_layers as u64) as u32;

            if layers_that_fit >= profile.total_layers / 2 {
                return GpuConfig {
                    gpu_layers: layers_that_fit,
                    quantization: *quant,
                    backend,
                    main_gpu: 0,
                    tensor_split: self.compute_tensor_split(gpus),
                };
            }
        }

        // Not enough VRAM for meaningful offload — use CPU
        GpuConfig {
            gpu_layers: 0,
            quantization: Quantization::Q4KM,
            backend: GpuBackend::Cpu,
            main_gpu: 0,
            tensor_split: None,
        }
    }

    fn total_available_vram(&self, gpus: &[GpuCapability]) -> u64 {
        gpus.iter()
            .map(|g| g.vram_mb.saturating_sub(g.vram_used_mb))
            .sum::<u64>()
            .saturating_sub(self.vram_reserve_mb)
    }

    fn compute_tensor_split(&self, gpus: &[GpuCapability]) -> Option<Vec<f32>> {
        if gpus.len() <= 1 {
            return None;
        }
        let total: f64 = gpus.iter().map(|g| g.vram_mb as f64).sum();
        Some(gpus.iter().map(|g| (g.vram_mb as f64 / total) as f32).collect())
    }

    fn model_profile(&self, model_name: &str) -> ModelProfile {
        // Return known profile for common models.
        // Fallback: estimate from model name (parameter count heuristic).
        match model_name {
            n if n.contains("8b") || n.contains("7b") => ModelProfile {
                total_layers: 33,
                vram_per_layer_mb: [
                    (Quantization::Q4KM, 135),
                    (Quantization::Q5KM, 165),
                    (Quantization::Q8_0, 258),
                ].into(),
                vram_overhead_mb: 300,
            },
            n if n.contains("13b") => ModelProfile {
                total_layers: 40,
                vram_per_layer_mb: [
                    (Quantization::Q4KM, 185),
                    (Quantization::Q5KM, 225),
                    (Quantization::Q8_0, 350),
                ].into(),
                vram_overhead_mb: 400,
            },
            n if n.contains("70b") => ModelProfile {
                total_layers: 80,
                vram_per_layer_mb: [
                    (Quantization::Q4KM, 480),
                    (Quantization::Q5KM, 585),
                    (Quantization::Q8_0, 910),
                ].into(),
                vram_overhead_mb: 800,
            },
            _ => ModelProfile {
                total_layers: 33,
                vram_per_layer_mb: [
                    (Quantization::Q4KM, 150),
                    (Quantization::Q5KM, 180),
                    (Quantization::Q8_0, 280),
                ].into(),
                vram_overhead_mb: 300,
            },
        }
    }
}
```

---

## 4. llama-server GPU Flags

Modified `LlamaServer::start()` from module 14 to include GPU configuration:

```rust
impl LlamaServer {
    pub async fn start(&mut self, gpu_config: &GpuConfig) -> Result<()> {
        if self.is_running().await {
            return Ok(());
        }

        // Select the correct binary based on GPU backend
        let binary = self.backend_binary(gpu_config.backend);

        let mut cmd = tokio::process::Command::new(&binary);
        cmd.arg("--model").arg(&self.model_path)
            .arg("--ctx-size").arg(self.context_size.to_string())
            .arg("--threads").arg(num_cpus::get().to_string());

        // GPU-specific flags
        if gpu_config.gpu_layers > 0 {
            cmd.arg("--n-gpu-layers").arg(gpu_config.gpu_layers.to_string());
        }

        if gpu_config.main_gpu > 0 {
            cmd.arg("--main-gpu").arg(gpu_config.main_gpu.to_string());
        }

        if let Some(ref split) = gpu_config.tensor_split {
            let split_str: Vec<String> = split.iter().map(|s| format!("{:.2}", s)).collect();
            cmd.arg("--tensor-split").arg(split_str.join(","));
        }

        // Transport config (from module 14)
        match &self.transport {
            LocalTransport::Tcp { port } => {
                cmd.arg("--port").arg(port.to_string());
            }
            LocalTransport::UnixSocket { path } => {
                cmd.arg("--host").arg(path.as_os_str());
            }
        }

        let child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        self.process = Some(child);
        self.wait_for_ready(Duration::from_secs(60)).await?; // GPU init may take longer
        Ok(())
    }

    /// Select the correct llama-server binary for the GPU backend.
    fn backend_binary(&self, backend: GpuBackend) -> PathBuf {
        match backend {
            GpuBackend::Cuda => PathBuf::from("/usr/bin/llama-server-cuda"),
            GpuBackend::Rocm => PathBuf::from("/usr/bin/llama-server-rocm"),
            GpuBackend::Vulkan => PathBuf::from("/usr/bin/llama-server-vulkan"),
            GpuBackend::Cpu => self.binary_path.clone(),
        }
    }
}
```

---

## 5. VRAM Monitor

```rust
use tokio::time;

/// Periodic VRAM usage monitor.
pub struct VramMonitor {
    interval: Duration,
    warn_threshold: f64,
    gpus: Vec<GpuCapability>,
    warning_sent: bool,
}

pub enum VramEvent {
    /// VRAM usage updated (for status display).
    Updated(Vec<VramUsage>),
    /// VRAM usage exceeds threshold.
    Warning { gpu_index: u32, usage_pct: f64 },
    /// llama-server likely OOM — suggest reducing layers.
    Critical { gpu_index: u32 },
}

pub struct VramUsage {
    pub gpu_index: u32,
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub usage_pct: f64,
}

impl VramMonitor {
    pub fn new(interval: Duration, warn_threshold: f64) -> Self {
        Self {
            interval,
            warn_threshold,
            gpus: Vec::new(),
            warning_sent: false,
        }
    }

    /// Start monitoring loop. Returns a receiver for VRAM events.
    pub fn start(
        &self,
        event_tx: tokio::sync::mpsc::Sender<VramEvent>,
    ) -> tokio::task::JoinHandle<()> {
        let interval = self.interval;
        let warn_threshold = self.warn_threshold;

        tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            let mut warning_sent = false;

            loop {
                ticker.tick().await;

                let gpus = GpuDetector::detect_all();
                let usages: Vec<VramUsage> = gpus.iter().map(|g| {
                    let usage_pct = if g.vram_mb > 0 {
                        g.vram_used_mb as f64 / g.vram_mb as f64
                    } else {
                        0.0
                    };
                    VramUsage {
                        gpu_index: g.index,
                        total_mb: g.vram_mb,
                        used_mb: g.vram_used_mb,
                        free_mb: g.vram_mb.saturating_sub(g.vram_used_mb),
                        usage_pct,
                    }
                }).collect();

                let _ = event_tx.send(VramEvent::Updated(usages.clone())).await;

                for usage in &usages {
                    if usage.usage_pct >= warn_threshold && !warning_sent {
                        let _ = event_tx.send(VramEvent::Warning {
                            gpu_index: usage.gpu_index,
                            usage_pct: usage.usage_pct,
                        }).await;
                        warning_sent = true;
                    }
                    if usage.usage_pct >= 0.98 {
                        let _ = event_tx.send(VramEvent::Critical {
                            gpu_index: usage.gpu_index,
                        }).await;
                    }
                }

                // Reset warning if usage drops below threshold
                if usages.iter().all(|u| u.usage_pct < warn_threshold) {
                    warning_sent = false;
                }
            }
        })
    }
}
```

---

## 6. GPU Info Tool for Sysinfo Skill

New tool added to the sysinfo skill manifest:

```yaml
# skills/built-in/sysinfo/tools/gpu_info.yaml
name: sys_gpu
description: Get GPU information including model, VRAM, driver, temperature, and utilization.
parameters:
  type: object
  properties:
    verbose:
      type: boolean
      description: Include detailed info (power, compute capability, all GPUs).
      default: false
  required: []
```

Tool implementation returns structured GPU data:

```rust
pub fn handle_sys_gpu(params: &serde_json::Value) -> Result<ToolResult> {
    let verbose = params.get("verbose")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let gpus = GpuDetector::detect_all();

    if gpus.is_empty() {
        return Ok(ToolResult::text("No GPU detected. Running in CPU-only mode."));
    }

    let mut output = String::new();
    for gpu in &gpus {
        output.push_str(&format!("GPU {}: {} ({})\n", gpu.index, gpu.model, gpu.vendor_name()));
        output.push_str(&format!(
            "  VRAM: {} MB total, {} MB used, {} MB free\n",
            gpu.vram_mb, gpu.vram_used_mb, gpu.vram_mb - gpu.vram_used_mb
        ));
        output.push_str(&format!("  Driver: {}\n", gpu.driver_version));

        if let Some(temp) = gpu.temperature_c {
            output.push_str(&format!("  Temperature: {}°C\n", temp));
        }
        if let Some(util) = gpu.utilization_pct {
            output.push_str(&format!("  Utilization: {}%\n", util));
        }
        if verbose {
            if let Some(ref cc) = gpu.compute_capability {
                output.push_str(&format!("  Compute capability: {}\n", cc));
            }
            if let Some(power) = gpu.power_watts {
                let limit = gpu.power_limit_watts.unwrap_or(0);
                output.push_str(&format!("  Power: {}W / {}W\n", power, limit));
            }
        }
    }

    Ok(ToolResult::text(&output))
}
```

---

## 7. llama.cpp Build Variants

The ISO includes multiple llama-server binaries, one per GPU backend:

```
/usr/bin/
  llama-server           # CPU-only (baseline, always works)
  llama-server-cuda      # NVIDIA CUDA backend
  llama-server-rocm      # AMD ROCm/HIP backend
  llama-server-vulkan    # Vulkan backend (Intel, fallback)
```

### Build Configuration

```bash
# CPU build (default)
cmake -B build-cpu -DGGML_NATIVE=OFF
cmake --build build-cpu --target llama-server

# CUDA build (NVIDIA)
cmake -B build-cuda -DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES="60;70;75;80;86;89;90"
cmake --build build-cuda --target llama-server

# ROCm build (AMD)
cmake -B build-rocm -DGGML_HIP=ON -DAMDGPU_TARGETS="gfx900;gfx906;gfx908;gfx90a;gfx1030;gfx1100"
cmake --build build-rocm --target llama-server

# Vulkan build (Intel, fallback)
cmake -B build-vulkan -DGGML_VULKAN=ON
cmake --build build-vulkan --target llama-server
```

The engine selects the correct binary at runtime based on `GpuConfig.backend`.

---

## 8. Kickstart / ISO Changes

Additions to the Fedora kickstart for GPU support:

```
# GPU detection tools
vulkan-tools              # vulkaninfo for GPU detection
mesa-vulkan-drivers       # Intel/AMD Vulkan drivers

# NVIDIA (not bundled — installed on demand)
# User runs: "install nvidia drivers" → triggers:
#   dnf install akmod-nvidia xorg-x11-drv-nvidia-cuda

# ROCm (not bundled — installed on demand)
# User runs: "install ROCm" → triggers:
#   dnf install rocm-hip rocm-smi
```

Driver packages are large and vendor-specific, so they are installed on demand via the package manager skill rather than bundled in the base ISO.

---

## 9. Implementation Stages

**Stage 1 -- GPU Detection (1-2 days)**
1. `GpuDetector` with nvidia-smi, rocm-smi, vulkaninfo parsing.
2. `GpuCapability` struct.
3. Detection at engine startup, cache results.
4. Test: verify detection on hardware with/without GPUs.

**Stage 2 -- Auto-Configuration (1-2 days)**
1. `GpuConfigurator` with VRAM-based layer/quantization selection.
2. Model profiles for common models (8B, 13B, 70B).
3. Multi-GPU tensor split calculation.
4. Test: verify correct configs for various VRAM sizes.

**Stage 3 -- llama-server GPU Integration (2-3 days)**
1. Multi-backend binary selection.
2. GPU flags in `LlamaServer::start()`.
3. Increased startup timeout for GPU initialization.
4. GPU failure fallback to CPU.
5. Test: start llama-server with GPU, verify acceleration, verify fallback.

**Stage 4 -- VRAM Monitoring and UI (1-2 days)**
1. `VramMonitor` with periodic polling.
2. VRAM warning toast in Chat Shell.
3. GPU/CPU indicator in status bar.
4. Performance metrics (tokens/s) after local responses.
5. Test: verify warnings at threshold, verify status bar updates.

**Stage 5 -- Sysinfo and Model Management (1 day)**
1. `sys_gpu` tool implementation and skill manifest.
2. GPU info card rendering.
3. Model recommendation based on VRAM.
4. Quantization variant download support.
5. Test: verify GPU info output, verify recommendations match VRAM.

---

## 10. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `detector.rs` | nvidia-smi output parsing, rocm-smi JSON parsing, vulkaninfo parsing |
| `capability.rs` | GpuCapability serialization, vendor detection |
| `configurator.rs` | Layer calculation for various VRAM sizes, quantization selection |
| `monitor.rs` | Warning threshold detection, event generation |
| `backend.rs` | Binary path selection, flag generation |

### Integration Tests

| Test | Method |
|------|--------|
| GPU detection | Run on real hardware, verify correct vendor/model/VRAM |
| Auto-config accuracy | Mock VRAM values, verify optimal layers selected |
| llama-server GPU start | Start with GPU flags, verify health check passes |
| GPU → CPU fallback | Force GPU failure (invalid layers), verify CPU fallback |
| VRAM warning | Load model near VRAM limit, verify warning fires |
| Multi-GPU split | Mock 2 GPUs, verify tensor split ratio |
| Performance metrics | Run inference, verify tokens/s reported |
| No-GPU graceful | Remove GPU tools, verify CPU-only startup |

### VM GPU Passthrough Testing

For CI/CD or developer testing with real GPU hardware:

```bash
# QEMU with GPU passthrough (NVIDIA example)
qemu-system-x86_64 \
    -machine q35 \
    -cpu host \
    -smp 4 \
    -m 8G \
    -device vfio-pci,host=01:00.0 \
    -cdrom levsha-os.iso \
    -boot d

# Verify inside VM:
# 1. nvidia-smi shows GPU
# 2. llama-server starts with --n-gpu-layers
# 3. tokens/s > 20 (vs ~5 on CPU)
```
