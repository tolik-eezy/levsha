# 06 — System Info Skill: Technical Plan

**Module:** System Info Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document specifies the implementation details for the system info skill: Linux command and API mappings, tool JSON schemas, output parsing, the skill manifest, and the testing plan.

---

## 2. Linux Command / API Mapping

| Tool | Command / Source | Output Format |
|---|---|---|
| `disk_usage` | `df -B1 --output=target,size,used,avail,pcent` | One row per mounted filesystem. Sizes in bytes for parsing, converted to human-readable by the engine. |
| `memory_info` | Read `/proc/meminfo` | Key-value pairs in kB. Parse MemTotal, MemFree, MemAvailable, Buffers, Cached, SwapTotal, SwapFree. |
| `cpu_info` | Read `/proc/cpuinfo` + `/proc/loadavg` | CPU model from first "model name" line. Core count from "processor" count. Load averages from loadavg. |
| `uptime` | Read `/proc/uptime` | First field = uptime in seconds (float). Second field = idle time. |
| `network_status` | `ip -j addr show` | JSON output from iproute2. Parse interface name, state, and inet/inet6 addresses. |
| `process_list` | `ps -eo pid,comm,rss,pcpu --sort=-rss` or `--sort=-pcpu` | One row per process. RSS in kB, CPU as percentage. |

All reads are non-destructive. No system state is modified.

---

## 3. Tool JSON Schemas

### 3.1 disk_usage

```json
{
  "name": "disk_usage",
  "description": "Report disk usage for mounted filesystems.",
  "parameters": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Specific mount point to check. Omit for all filesystems."
      }
    },
    "required": []
  }
}
```

### 3.2 memory_info

```json
{
  "name": "memory_info",
  "description": "Report system memory and swap usage.",
  "parameters": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

### 3.3 cpu_info

```json
{
  "name": "cpu_info",
  "description": "Report CPU model, core count, and current load averages.",
  "parameters": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

### 3.4 uptime

```json
{
  "name": "uptime",
  "description": "Report how long the system has been running.",
  "parameters": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

### 3.5 network_status

```json
{
  "name": "network_status",
  "description": "Report network interface status, IP addresses, and internet connectivity.",
  "parameters": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

### 3.6 process_list

```json
{
  "name": "process_list",
  "description": "List top running processes sorted by resource usage.",
  "parameters": {
    "type": "object",
    "properties": {
      "sort_by": {
        "type": "string",
        "enum": ["cpu", "memory"],
        "description": "Sort processes by CPU or memory usage. Default: memory."
      },
      "limit": {
        "type": "integer",
        "description": "Maximum number of processes to return. Default: 10."
      }
    },
    "required": []
  }
}
```

---

## 4. Output Parsing

### 4.1 disk_usage

Parse `df` output line by line. Skip the header row. For each row, extract mount point, total size, used, available, and percentage. Convert byte values to human-readable (MB/GB) with one decimal place. Filter out pseudo-filesystems (tmpfs, devtmpfs) unless they are `/tmp`.

### 4.2 memory_info

Parse `/proc/meminfo` as key-value pairs. Compute:
- **Used** = MemTotal - MemAvailable
- **Cache** = Buffers + Cached
- **Swap used** = SwapTotal - SwapFree

Convert all values from kB to MB or GB as appropriate.

### 4.3 cpu_info

Parse `/proc/cpuinfo` for the first `model name` field. Count `processor` entries for core count. Read `/proc/loadavg` and split on whitespace for 1/5/15 minute averages.

### 4.4 uptime

Read `/proc/uptime`, parse the first float. Convert seconds to days, hours, and minutes.

### 4.5 network_status

Parse JSON from `ip -j addr show`. For each interface, extract `ifname`, `operstate`, and `addr_info[].local` for IPv4/IPv6 addresses. Skip loopback unless explicitly requested. Check internet connectivity with a DNS lookup for a well-known hostname.

### 4.6 process_list

Parse `ps` output line by line. Skip the header. For each row, extract PID, command name, RSS (convert kB to MB), and CPU percentage. Apply sort and limit.

---

## 5. Skill Manifest

```yaml
# skills/built-in/system-info/skill.yaml
name: system-info
version: 1.0.0
description: "Query disk, memory, CPU, uptime, network status, and running processes."
author: levsha-os
built_in: true
removable: false

requires:
  packages:
    - procps-ng
    - iproute

prompt: prompts/system-info.md

tools:
  - tools/disk_usage.json
  - tools/memory_info.json
  - tools/cpu_info.json
  - tools/uptime.json
  - tools/network_status.json
  - tools/process_list.json
```

---

## 6. File Structure

```
skills/built-in/system-info/
  skill.yaml
  prompts/
    system-info.md              # System prompt fragment
  tools/
    disk_usage.json             # Tool schema
    memory_info.json
    cpu_info.json
    uptime.json
    network_status.json
    process_list.json
```

---

## 7. Testing Plan

### 7.1 Unit Tests

| Test | Description |
|---|---|
| Parse df output | Verify filesystem extraction from `df` with various mount points. |
| Parse /proc/meminfo | Verify memory calculation (used, available, cache, swap). |
| Parse /proc/cpuinfo | Verify model name and core count extraction. |
| Parse /proc/uptime | Verify conversion from seconds to human-readable format. |
| Parse ip addr JSON | Verify interface name, state, and IP extraction. |
| Parse ps output | Verify PID, name, RSS, CPU extraction with sort and limit. |
| Human-readable formatting | Verify kB/MB/GB conversion and decimal precision. |

### 7.2 Integration Tests (VM)

| Test | Description |
|---|---|
| Disk usage query | Ask "how much disk space do I have?" — verify output matches `df -h`. |
| Memory query | Ask "how much RAM is free?" — verify output matches `free -h`. |
| CPU query | Ask "what CPU is this?" — verify model and core count match `lscpu`. |
| Uptime query | Ask "how long has the system been running?" — verify against `uptime`. |
| Network query | Ask "what's my IP?" — verify IP matches `ip addr`. |
| Process query | Ask "what's using the most memory?" — verify top process matches `ps aux --sort=-rss`. |
| Combined status | Ask "how's the system doing?" — verify all metrics appear in status box. |
| Accuracy | All reported values must match actual system values (100% accuracy target). |

### 7.3 Acceptance Tests

| Test | Maps to |
|---|---|
| Disk usage formatted correctly | SY-01 |
| Memory usage reported with used/total/available | SY-02 |
| CPU model, cores, and load displayed | SY-03 |
| Human-readable uptime shown | SY-04 |
| Network status with IP and connectivity | SY-05 |
| Process table sorted by requested metric | SY-06 |
