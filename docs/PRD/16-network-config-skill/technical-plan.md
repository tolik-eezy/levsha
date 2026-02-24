# 16 — Network Configuration Skill: Technical Plan

**Module:** Network Configuration Skill
**Language:** Rust (engine tools) + YAML/JSON (skill definition)
**Phase:** 2

---

## 1. Skill Structure

```
skills/
  built-in/
    network-config/
      skill.yaml
      prompts/
        network-config.md
      tools/
        wifi_scan.json
        wifi_connect.json
        wifi_disconnect.json
        wifi_forget.json
        wifi_saved.json
        net_interfaces.json
        net_set_static.json
        net_set_dhcp.json
        net_dns.json
        net_ping.json
        net_traceroute.json
        net_dns_lookup.json
        net_ports.json
```

---

## 2. Tool Definitions

### wifi_scan

```json
{
  "name": "wifi_scan",
  "description": "Scan for available Wi-Fi networks. Returns SSID, signal strength, security type, and channel.",
  "input_schema": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

**Command:** `nmcli -t -f SSID,SIGNAL,SECURITY,CHAN device wifi list --rescan yes`

### wifi_connect

```json
{
  "name": "wifi_connect",
  "description": "Connect to a Wi-Fi network. Password is passed securely and not stored in chat history.",
  "input_schema": {
    "type": "object",
    "properties": {
      "ssid": {
        "type": "string",
        "description": "Network name (SSID)"
      },
      "password": {
        "type": "string",
        "description": "Network password (optional for open networks)"
      }
    },
    "required": ["ssid"]
  }
}
```

**Command:** `nmcli device wifi connect '{{ssid}}' password '{{password}}'`

### net_set_static

```json
{
  "name": "net_set_static",
  "description": "Configure a static IP address for a network interface. Requires confirmation as it may disrupt connectivity.",
  "input_schema": {
    "type": "object",
    "properties": {
      "interface": {
        "type": "string",
        "description": "Network interface name (e.g., eth0, wlan0)"
      },
      "ip": {
        "type": "string",
        "description": "IP address with CIDR (e.g., 192.168.1.100/24)"
      },
      "gateway": {
        "type": "string",
        "description": "Gateway IP address"
      },
      "dns": {
        "type": "string",
        "description": "DNS servers (comma-separated)"
      }
    },
    "required": ["interface", "ip", "gateway"]
  }
}
```

**Commands:**
```bash
nmcli connection modify '{{connection}}' ipv4.method manual ipv4.addresses '{{ip}}' ipv4.gateway '{{gateway}}'
nmcli connection modify '{{connection}}' ipv4.dns '{{dns}}'
nmcli connection up '{{connection}}'
```

### net_ping

```json
{
  "name": "net_ping",
  "description": "Ping a host to test connectivity and measure latency.",
  "input_schema": {
    "type": "object",
    "properties": {
      "host": {
        "type": "string",
        "description": "Hostname or IP to ping"
      },
      "count": {
        "type": "integer",
        "description": "Number of pings (default: 4)"
      }
    },
    "required": ["host"]
  }
}
```

**Command:** `ping -c {{count}} {{host}}`

---

## 3. Engine-Side Tool Handlers

Network tools need custom handlers for password security and confirmation.

```rust
// In engine/src/tools/network.rs

pub fn handle_wifi_connect(input: &serde_json::Value) -> Result<ToolOutput> {
    let ssid = input["ssid"].as_str().ok_or("missing ssid")?;
    let password = input.get("password").and_then(|p| p.as_str());

    // Password handling: request secure input from chat shell if not provided
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            return Ok(ToolOutput::secure_input(SecureInputRequest {
                prompt: format!("{} requires a password.", ssid),
                field_name: "password".into(),
                tool_name: "wifi_connect".into(),
                tool_input: input.clone(),
            }));
        }
    };

    let output = std::process::Command::new("nmcli")
        .args(["device", "wifi", "connect", ssid, "password", &password])
        .output()?;

    // Never include password in output
    if output.status.success() {
        // Get the assigned IP
        let ip = get_interface_ip("wlan0").unwrap_or_default();
        Ok(ToolOutput::text(format!(
            "Connected to {}. IP: {}", ssid, ip
        )))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(ToolOutput::error(format!(
            "Failed to connect to {}: {}", ssid, stderr
        )))
    }
}

pub fn handle_net_set_static(input: &serde_json::Value) -> Result<ToolOutput> {
    let interface = input["interface"].as_str().ok_or("missing interface")?;
    let ip = input["ip"].as_str().ok_or("missing ip")?;
    let gateway = input["gateway"].as_str().ok_or("missing gateway")?;
    let dns = input.get("dns").and_then(|d| d.as_str()).unwrap_or("8.8.8.8");

    // Get current config for confirmation display
    let current = get_interface_config(interface)?;

    // This is a network change — require confirmation
    Ok(ToolOutput::confirm(ConfirmRequest {
        command: format!(
            "Set {} to static IP {} (gateway: {}, DNS: {})",
            interface, ip, gateway, dns
        ),
        reason: format!(
            "Changing network settings may disconnect you.\n\
             Current: {} ({}) → New: {} (Static)",
            current.ip, current.method, ip
        ),
        risk: "destructive".into(),
    }))
}

fn get_interface_ip(interface: &str) -> Result<String> {
    let output = std::process::Command::new("nmcli")
        .args(["-t", "-f", "IP4.ADDRESS", "device", "show", interface])
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.split(':').nth(1).unwrap_or("unknown").trim().to_string())
}

fn parse_wifi_scan(output: &str) -> Vec<WifiNetwork> {
    output.lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split(':').collect();
            WifiNetwork {
                ssid: parts.get(0).unwrap_or(&"").to_string(),
                signal: parts.get(1).unwrap_or(&"0").parse().unwrap_or(0),
                security: parts.get(2).unwrap_or(&"").to_string(),
                channel: parts.get(3).unwrap_or(&"0").parse().unwrap_or(0),
            }
        })
        .collect()
}
```

---

## 4. Secure Input Protocol

Wi-Fi passwords require a secure input channel that bypasses conversation history.

```rust
// IPC extension for secure input
#[derive(Serialize, Deserialize)]
pub struct SecureInputRequest {
    pub prompt: String,
    pub field_name: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct SecureInputResponse {
    pub field_name: String,
    pub value: String,  // Cleared from memory after use
    pub tool_name: String,
    pub tool_input: serde_json::Value,
}
```

The Chat Shell renders a masked input widget (reusing the API key input pattern) and sends the value directly to the engine without adding it to the message history.

---

## 5. Implementation Stages

**Stage 1 — Tool Definitions (0.5 day)**
1. Create all 13 tool JSON schemas.
2. Create skill.yaml manifest.
3. Write prompt fragment.

**Stage 2 — Wi-Fi Tools (1 day)**
1. wifi_scan with nmcli output parsing.
2. wifi_connect with secure password handling.
3. wifi_disconnect, wifi_forget, wifi_saved.
4. Secure input IPC protocol.

**Stage 3 — IP Configuration (1 day)**
1. net_interfaces with status parsing.
2. net_set_static with confirmation dialog.
3. net_set_dhcp.
4. net_dns.

**Stage 4 — Diagnostics (0.5 day)**
1. net_ping, net_traceroute, net_dns_lookup, net_ports.
2. Output formatting.

**Stage 5 — Testing (0.5 day)**
1. Wi-Fi scan parsing tests.
2. Confirmation flow for static IP.
3. Password security tests (not in history).

---

## 6. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| Wi-Fi scan parser | nmcli output parsing, various formats |
| IP config parser | Interface status parsing |
| DNS lookup parser | dig output parsing |

### Integration Tests

| Test | Method |
|------|--------|
| Wi-Fi scan | Mock nmcli output, verify formatted result |
| Connect with password | Mock nmcli, verify password not in tool output |
| Static IP confirmation | Run net_set_static, verify confirmation triggered |
| Ping formatting | Mock ping output, verify formatted display |
| DNS lookup | Mock dig output, verify record parsing |

### Security Tests

| Test | Assertion |
|------|-----------|
| Password not in history | After wifi_connect, search history for password string |
| Password not in logs | Check engine logs for password leakage |
| Static IP requires confirmation | net_set_static always triggers ConfirmRequest |
