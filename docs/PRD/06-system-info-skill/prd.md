# 06 — System Info Skill: Product Requirements

**Module:** System Info Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The system info skill is one of two built-in skills that ship with Levsha OS MVP. It enables users to query system status — disk usage, memory, CPU, uptime, network, and running processes — through natural language conversation. The skill reads data from standard Linux sources (`/proc`, `/sys`, standard utilities) and formats it for display in the chat.

This skill is always active. It cannot be disabled or removed.

---

## 2. Requirements

| ID | Requirement | Priority | Acceptance Criteria |
|---|---|---|---|
| SY-01 | Report disk usage | P0 | User asks "how much disk space do I have?" and receives formatted disk usage for all mounted filesystems showing used/total/percentage. |
| SY-02 | Report memory usage | P0 | User asks "how much RAM is free?" and receives used/total/available memory and swap usage. |
| SY-03 | Report CPU info and load | P0 | User asks "what CPU is this?" and receives CPU model, core count, and current load average. User asks "is the system busy?" and receives current CPU utilization. |
| SY-04 | Report uptime | P0 | User asks "how long has the system been running?" and receives uptime in human-readable format (days, hours, minutes). |
| SY-05 | Report network status | P0 | User asks "am I connected?" and receives connection status, IP address(es), and interface info. |
| SY-06 | List running processes | P0 | User asks "what's running right now?" and receives a table of top processes sorted by resource usage (CPU or memory). |

---

## 3. Natural Language Examples

**Disk usage:**
- "how much disk space do I have?"
- "is the disk full?"
- "show disk usage"
- "how much storage is left?"

**Memory:**
- "how much RAM is free?"
- "memory usage?"
- "am I running low on memory?"
- "how much swap is being used?"

**CPU:**
- "what CPU is this?"
- "how many cores?"
- "is the system busy?"
- "show me CPU usage"

**Uptime:**
- "how long has the system been running?"
- "uptime"
- "when did the system start?"

**Network:**
- "am I connected?"
- "what's my IP?"
- "show network status"
- "is the internet working?"

**Processes:**
- "what's running right now?"
- "what processes are using the most memory?"
- "top processes by CPU"
- "show me what's eating resources"

**Combined:**
- "how's the system doing?" — returns a combined status overview (CPU, memory, disk, uptime, network).

---

## 4. Combined Status Overview

When the user asks a general question like "how's the system doing?", the skill returns a combined status box with all key metrics:

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

---

## 5. Acceptance Criteria

1. User asks about disk space — formatted disk usage is displayed for all mounted filesystems.
2. User asks about memory — used, total, and available RAM are displayed. Swap included if active.
3. User asks about CPU — model name, core count, and current load are displayed.
4. User asks about uptime — human-readable uptime is displayed.
5. User asks about network — connection status and IP address are displayed.
6. User asks about processes — top processes table is displayed, sorted by requested metric.
7. General system status query — combined overview box is rendered.
8. All reported values are accurate (100% accuracy target per success metrics).

---

## 6. Out of Scope (MVP)

- Hardware inventory (GPU, USB devices, PCI devices)
- Temperature/thermal monitoring
- Battery status (VM only in MVP)
- Historical data or trends
- Alerting or thresholds
- Service management (start/stop/restart services)
- Log viewing
