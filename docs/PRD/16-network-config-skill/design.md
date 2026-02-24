# 16 — Network Configuration Skill: Design Specification

**Module:** Network Configuration Skill
**Phase:** 2

---

## 1. Interaction Design

Network configuration is managed entirely through conversation. The skill formats network information as styled cards in the chat.

### Wi-Fi Network Scan

```
User: scan for wifi networks

Levsha: I found 6 available networks:

  ┌─ Wi-Fi Networks ─────────────────────────────────────┐
  │                                                       │
  │  Signal  Name                  Security    Channel    │
  │  ─────────────────────────────────────────────────── │
  │  ████    HomeNetwork           WPA2        6          │
  │  ███░    CoffeeShop-5G         WPA2        36         │
  │  ██░░    Neighbor-WiFi         WPA2        11         │
  │  ██░░    OpenGuest             Open        1          │
  │  █░░░    FarAway-Net           WPA3        44         │
  │  █░░░    IoT-Network           WPA2        6          │
  │                                                       │
  │  Say "connect to <name>" to join a network.           │
  │                                                       │
  └───────────────────────────────────────────────────────┘
```

### Connect to Network

```
User: connect to CoffeeShop-5G

Levsha: CoffeeShop-5G requires a password.
        Enter the Wi-Fi password below:

        ┌────────────────────────────────────────────┐
        │  Password: ________________________________│
        └────────────────────────────────────────────┘
```

After entering the password:

```
Levsha: ✓ Connected to CoffeeShop-5G
        IP: 192.168.1.42
        Signal: Good (███░)
```

### Network Status

```
User: what's my network status?

Levsha:
  ┌─ Network Status ──────────────────────────────────┐
  │                                                     │
  │  Interface    Status     IP Address                 │
  │  ─────────────────────────────────────────────────  │
  │  📶 wlan0     Connected  192.168.1.42               │
  │               CoffeeShop-5G (WPA2)                  │
  │                                                     │
  │  🔌 eth0      Connected  10.0.2.15                  │
  │               DHCP                                   │
  │                                                     │
  │  DNS: 8.8.8.8, 8.8.4.4                             │
  │  Gateway: 192.168.1.1                               │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

---

## 2. Network Diagnostics Display

### Ping Results

```
User: ping google.com

Levsha:
  ┌─ Ping: google.com ────────────────────────────────┐
  │                                                     │
  │  PING google.com (142.250.80.46)                   │
  │                                                     │
  │  Reply 1:  12.3 ms                                 │
  │  Reply 2:  11.8 ms                                 │
  │  Reply 3:  12.1 ms                                 │
  │  Reply 4:  13.0 ms                                 │
  │                                                     │
  │  ─────────────────────────────────────────────────  │
  │  4 packets sent, 4 received, 0% loss               │
  │  min/avg/max: 11.8/12.3/13.0 ms                   │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

### DNS Lookup

```
User: lookup the DNS for github.com

Levsha:
  ┌─ DNS: github.com ─────────────────────────────────┐
  │                                                     │
  │  A      140.82.121.4                                │
  │  AAAA   2606:50c0:8000::154                        │
  │  MX     alt1.aspmx.l.google.com (5)               │
  │  NS     dns1.p08.nsone.net                         │
  │                                                     │
  │  Query time: 23 ms                                  │
  │  Server: 8.8.8.8                                    │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

---

## 3. Static IP Configuration

Configuration changes require confirmation before applying.

```
User: set eth0 to static IP 10.0.1.100

Levsha:
  ┌─ ⚠ Network Configuration Change ──────────────────┐
  │                                                      │
  │  Interface: eth0                                     │
  │                                                      │
  │  Current:                                            │
  │    IP: 10.0.2.15 (DHCP)                             │
  │    Gateway: 10.0.2.2                                 │
  │    DNS: 10.0.2.3                                     │
  │                                                      │
  │  New:                                                │
  │    IP: 10.0.1.100/24 (Static)                       │
  │    Gateway: 10.0.1.1                                 │
  │    DNS: 8.8.8.8                                      │
  │                                                      │
  │  ⚠ Changing network settings may disconnect you.    │
  │                                                      │
  │  ┌──────────┐  ┌──────────┐                         │
  │  │  Apply   │  │  Cancel  │                         │
  │  └──────────┘  └──────────┘                         │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

---

## 4. Styling Reference

### Wi-Fi Card

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Signal bars (full) | `$accent-green` (#62B37B) |
| Signal bars (empty) | `$bg-secondary` (#F5F1EA) |
| Network name | IBM Plex Sans 14px, weight 500, `$text-primary` |
| Security type | IBM Plex Mono 12px, `$text-tertiary` |
| Channel | IBM Plex Mono 12px, `$text-tertiary` |
| Table header | IBM Plex Sans 12px, weight 600, `$text-tertiary`, uppercase |
| Table separator | 1px `$border-primary` |
| Open network indicator | `$accent-gold` (#D4A853) text "Open" |

### Password Input

| Element | Style |
|---------|-------|
| Container | `$bg-surface`, 1px `$border-primary`, cornerRadius 8 |
| Label | IBM Plex Sans 13px, weight 500, `$text-secondary` |
| Input | IBM Plex Mono 14px, `$text-primary`, masked dots |
| Focus border | 2px `$accent-copper` |

### Network Status Card

| Element | Style |
|---------|-------|
| Interface icon (📶) | Lucide `wifi`, `$accent-green` (connected) or `$text-tertiary` (disconnected) |
| Interface icon (🔌) | Lucide `cable`, `$accent-green` (connected) or `$text-tertiary` |
| Interface name | IBM Plex Mono 13px, weight 600, `$text-primary` |
| Status | IBM Plex Sans 13px, `$accent-green` (Connected) or `$accent-copper` (Disconnected) |
| IP address | IBM Plex Mono 13px, `$text-primary` |
| Details | IBM Plex Sans 12px, `$text-secondary` |

### Diagnostic Results Card

| Element | Style |
|---------|-------|
| Code font | IBM Plex Mono 13px, `$text-primary` |
| Stats line | IBM Plex Mono 12px, weight 500, `$text-secondary` |
| Timing values | IBM Plex Mono 12px, `$accent-green` (good < 50ms) or `$accent-gold` (moderate < 200ms) or `$accent-copper` (poor) |
| Packet loss 0% | `$accent-green` |
| Packet loss > 0% | `$accent-copper` |

### Configuration Change Card

| Element | Style |
|---------|-------|
| Container | `$danger-bg` (#FFF5F0), 2px `$accent-copper` border |
| Warning icon | Lucide `alert-triangle`, `$accent-gold` |
| Current values | IBM Plex Mono 13px, `$text-secondary` |
| New values | IBM Plex Mono 13px, weight 600, `$text-primary` |
| Warning text | IBM Plex Sans 13px, `$accent-copper` |

---

## 5. Wi-Fi Password Security

- Wi-Fi passwords use a dedicated masked input widget (like the API key input).
- Password is never stored in conversation history.
- Password is passed directly to `nmcli` and stored only in NetworkManager's connection files.
- Chat shows "✓ Connected to [SSID]" without revealing the password.

---

## 6. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in network messages |
| Lucide `wifi` | Wi-Fi interface icon |
| Lucide `cable` | Ethernet interface icon |
| Lucide `wifi-off` | Disconnected Wi-Fi |
| Lucide `signal` | Signal strength |
| Lucide `globe` | DNS/internet |
| Lucide `activity` | Ping/traceroute |
| Lucide `alert-triangle` | Configuration change warning |
| Lucide `lock` | WPA2/WPA3 security indicator |
| Lucide `unlock` | Open network indicator |
