# 16 — Network Configuration Skill: Product Requirements

**Module:** Network Configuration Skill
**Phase:** 2 (Separate Project)
**Status:** Draft

---

## 1. Overview

Phase 1 assumes auto-configured ethernet via DHCP. The network configuration skill enables Wi-Fi setup, static IP configuration, DNS management, and network diagnostics — all through chat.

---

## 2. Functional Requirements

### 2.1 Wi-Fi Management (NC-01)

| Field | Value |
|-------|-------|
| **ID** | NC-01 |
| **Priority** | P0 |
| **Requirement** | Users can scan, connect to, and manage Wi-Fi networks via chat. |

**Tools:**

| Tool | Description | Command |
|------|-------------|---------|
| `wifi_scan` | Scan for available networks | `nmcli device wifi list` |
| `wifi_connect` | Connect to a network | `nmcli device wifi connect '{{ssid}}' password '{{password}}'` |
| `wifi_disconnect` | Disconnect from current network | `nmcli device disconnect {{interface}}` |
| `wifi_forget` | Remove saved network | `nmcli connection delete '{{ssid}}'` |
| `wifi_saved` | List saved networks | `nmcli connection show` |

**Security note:** Wi-Fi passwords are passed to `nmcli` and stored in NetworkManager's connection files. They are not stored in conversation history.

**Acceptance Criteria:**

- [ ] User can scan for Wi-Fi networks.
- [ ] User can connect with a password.
- [ ] Connection status is reported.
- [ ] Saved networks can be listed and removed.
- [ ] Passwords are not stored in chat history.

### 2.2 IP Configuration (NC-02)

| Field | Value |
|-------|-------|
| **ID** | NC-02 |
| **Priority** | P1 |
| **Requirement** | Users can configure static IP, DNS, and gateway settings. |

**Tools:**

| Tool | Description |
|------|-------------|
| `net_interfaces` | List network interfaces and their status. |
| `net_set_static` | Set static IP, gateway, DNS for an interface. |
| `net_set_dhcp` | Switch an interface back to DHCP. |
| `net_dns` | View/set DNS servers. |

**Acceptance Criteria:**

- [ ] User can set static IP configuration.
- [ ] User can revert to DHCP.
- [ ] DNS servers can be configured.
- [ ] Configuration changes are confirmed before applying.

### 2.3 Network Diagnostics (NC-03)

| Field | Value |
|-------|-------|
| **ID** | NC-03 |
| **Priority** | P0 |
| **Requirement** | Users can diagnose network issues via chat. |

**Tools:**

| Tool | Description | Command |
|------|-------------|---------|
| `net_ping` | Ping a host | `ping -c {{count}} {{host}}` |
| `net_traceroute` | Trace route to host | `traceroute {{host}}` |
| `net_dns_lookup` | DNS lookup | `dig {{domain}}` |
| `net_ports` | Show listening ports | `ss -tlnp` |

**Acceptance Criteria:**

- [ ] Basic network diagnostics work via chat.
- [ ] Results are formatted clearly.
- [ ] Failures produce helpful troubleshooting suggestions (via LLM).

---

## 3. Skill Manifest

```yaml
name: network-config
version: 0.1.0
description: "Configure Wi-Fi, IP settings, DNS, and diagnose network issues"
author: levsha
builtin: false

prompt: prompts/network-config.md
tools:
  - tools/wifi_scan.json
  - tools/wifi_connect.json
  - tools/wifi_disconnect.json
  - tools/wifi_forget.json
  - tools/wifi_saved.json
  - tools/net_interfaces.json
  - tools/net_set_static.json
  - tools/net_set_dhcp.json
  - tools/net_dns.json
  - tools/net_ping.json
  - tools/net_traceroute.json
  - tools/net_dns_lookup.json
  - tools/net_ports.json

requires:
  packages:
    - NetworkManager
    - bind-utils
    - traceroute
```

---

## 4. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skills System (04) | Upstream | Uses the skill framework. |
| Base System (01) | Upstream | NetworkManager must be installed and running. |
