# 06 — System Info Skill: Design

> **Visual styling follows [theme.design.md](../theme.design.md).** All colors, typography, spacing, and component styles are defined in the central theme document. This module must not define its own color palette or override theme tokens.

**Module:** System Info Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document defines the tool interface, system prompt fragment, response formatting, and data sources for the system info skill. The skill provides six tools that query system metrics from standard Linux sources and format them for display in the chat.

---

## 2. Tool Definitions

### 2.1 `disk_usage`

Reports filesystem disk usage.

- **Parameters:** `path` (string, optional) — specific mount point. Defaults to all mounted filesystems.
- **Behavior:** Reads disk usage data. Returns per-filesystem: mount point, total size, used, available, and percentage used.
- **Data source:** `df -h` or `statvfs` syscall.

### 2.2 `memory_info`

Reports memory and swap usage.

- **Parameters:** None.
- **Behavior:** Returns total, used, available, and cached memory. Includes swap if active.
- **Data source:** `/proc/meminfo`.

### 2.3 `cpu_info`

Reports CPU model, core count, and current load.

- **Parameters:** None.
- **Behavior:** Returns CPU model name, architecture, core count, and current load averages (1/5/15 min).
- **Data source:** `/proc/cpuinfo`, `/proc/loadavg`.

### 2.4 `uptime`

Reports system uptime.

- **Parameters:** None.
- **Behavior:** Returns uptime in seconds. The LLM formats it into human-readable text (e.g., "2 days, 3 hours, 17 minutes").
- **Data source:** `/proc/uptime`.

### 2.5 `network_status`

Reports network connectivity and interface info.

- **Parameters:** None.
- **Behavior:** Returns per-interface: name, status (up/down), IP address(es), and MAC address. Indicates whether the system has internet connectivity.
- **Data source:** `ip addr`, connectivity check via DNS or HTTP probe.

### 2.6 `process_list`

Lists running processes.

- **Parameters:** `sort_by` (string, optional) — "cpu" or "memory". Defaults to "memory". `limit` (integer, optional) — max results. Defaults to 10.
- **Behavior:** Returns top processes sorted by the specified metric. Each entry: PID, name, memory usage, CPU percentage.
- **Data source:** `/proc/[pid]/stat`, `/proc/[pid]/status`, or `ps aux`.

---

## 3. System Prompt Fragment

```
You have access to system monitoring tools. You can report disk usage, memory, CPU info, uptime, network status, and running processes.

When the user asks a general question like "how's the system doing?", provide a combined overview of CPU, memory, disk, uptime, and network in a single formatted status box.

Format all metrics in human-readable units (MB, GB, percentages). Use tables for process lists and multi-filesystem disk reports.

When reporting on system health, add a brief assessment: "Everything looks healthy" or flag concerns like low disk space or high memory usage.

Never suggest the user run commands directly. You are the interface.
```

---

## 4. Response Formatting

All response formatting uses theme tokens from `theme.design.md`. Refer to the theme for exact hex values.

**Color token reference for this module:**

| Symbol / Element | Theme Token | Color | Usage |
|---|---|---|---|
| Table borders | `bg-tertiary` | `#EBE6DC` gentle tan | Box-drawing characters |
| Table header text | `text-primary` | `#3A3228` warm brown | Section titles (e.g., "System Status") |
| Table body text | `text-primary` | `#3A3228` warm brown | Metric values |
| Labels (CPU, Memory...) | `text-secondary` | `#6B5D4F` warm mid-brown | Metric labels |
| Healthy indicator | `success` | `#62B37B` green | Values within normal range |
| Warning indicator | `warning` | `#D4A853` gold | Values approaching threshold |
| Critical indicator | `error` | `#C67A52` copper | Values exceeding threshold |
| Info indicator | `info` | `#8A7F72` warm brown | Informational notes |
| Status box background | `bg-surface` | `#FDFBF7` lightest parchment | Container background |
| Status box border | `bg-tertiary` | `#EBE6DC` gentle tan | Container border |

### 4.1 Combined Status Box

Used when the user asks a general question about system health. Container uses `bg-surface` background, `bg-tertiary` border. Labels in `text-secondary`, values in `text-primary`.

```
  ┌─── System Status ────────────────────────────────┐
  │                                                   │
  │  CPU      2 vCPUs (x86_64) — 12% load            │
  │  Memory   847 MB / 2048 MB (41%)                  │
  │  Disk     2.1 GB / 8.0 GB used (26%)             │
  │  Uptime   1 hour, 23 minutes                      │
  │  Network  connected — 10.0.2.15 (NAT)            │
  │                                                   │
  └───────────────────────────────────────────────────┘
```

### 4.2 Process Table

```
  ┌─── Top Processes by Memory ──────────────────────┐
  │  PID   NAME              MEM     CPU              │
  │  1142  levsha-chat       312 MB  3.2%             │
  │  892   systemd-journald   48 MB  0.1%             │
  │  1089  pipewire           23 MB  0.0%             │
  │  1     systemd            12 MB  0.0%             │
  └──────────────────────────────────────────────────┘
```

### 4.3 Disk Usage Table

```
  ┌─── Disk Usage ───────────────────────────────────┐
  │  Mount   Size    Used    Free    Use%             │
  │  /       8.0 GB  2.1 GB  5.9 GB  26%             │
  └──────────────────────────────────────────────────┘
```

### 4.4 Network Status

```
  ┌─── Network ─────────────────────────────────────┐
  │  Interface  Status  IP Address                    │
  │  eth0       up      10.0.2.15                     │
  │  lo         up      127.0.0.1                     │
  └──────────────────────────────────────────────────┘

  Internet: connected                ← "connected" in success (#62B37B)
                                     ← "disconnected" in error (#C67A52)
```

---

## 5. Data Sources

| Metric | Primary Source | Fallback |
|---|---|---|
| Disk usage | `statvfs` syscall / `df` | — |
| Memory | `/proc/meminfo` | `free` command |
| CPU model | `/proc/cpuinfo` | `lscpu` |
| CPU load | `/proc/loadavg` | `uptime` command |
| Uptime | `/proc/uptime` | `uptime` command |
| Network interfaces | `ip addr` (via netlink or command) | `ifconfig` |
| Internet connectivity | DNS lookup or HTTP HEAD to known host | — |
| Processes | `/proc/[pid]/stat` + `/proc/[pid]/status` | `ps aux --sort` |

All data is read-only. The system info skill never modifies system state.

---

## 6. UX Flow: System Status Query (Scenario 10.2)

```
User: "how's the system doing?"

1. LLM interprets as a general system status query.
2. LLM calls cpu_info(), memory_info(), disk_usage(), uptime(), network_status().
3. Engine executes all five tools (can be parallelized).
4. LLM formats results into the combined status box.
5. LLM adds a brief health assessment.

User: "what processes are using the most memory?"

6. LLM calls process_list(sort_by="memory", limit=10).
7. Engine reads /proc data and returns top 10 by memory.
8. LLM formats as a process table.
```

---

## 7. Health Assessment Logic

The LLM provides a brief health comment based on thresholds. Health status indicators use theme semantic colors:

| Status | Theme Token | Color | Indicator |
|---|---|---|---|
| Healthy | `success` | `#62B37B` green | `✓` checkmark |
| Warning | `warning` | `#D4A853` gold | `⚠` warning triangle |
| Critical | `error` | `#C67A52` copper | `✗` failure X |

**Threshold table:**

| Metric | Healthy | Warning | Critical |
|---|---|---|---|
| Disk usage | < 70% | 70–90% | > 90% |
| Memory usage | < 70% | 70–90% | > 90% |
| CPU load (1 min avg) | < core count | 1–2x core count | > 2x core count |
| Network | connected | — | disconnected |

The LLM uses these thresholds to decide tone and color: "Everything looks healthy" (with `✓` in `success` green), "Disk space is getting low" (with `⚠` in `warning` gold), or "Memory is critically low — consider closing some processes" (with `✗` in `error` copper).
