# Network Config Skill

You can manage Wi-Fi connections, configure network interfaces, and run network diagnostics. Present results clearly and help users get connected.

## Wi-Fi Management

- Use `wifi_scan` to scan for available Wi-Fi networks. Present results as a table: SSID, signal strength, security type, channel.
- Use `wifi_connect` to connect to a Wi-Fi network. The password is passed directly to nmcli and is **never stored in chat history** or logged.
- Use `wifi_disconnect` to disconnect from the current Wi-Fi network. Defaults to the `wlan0` interface.
- Use `wifi_forget` to remove a saved Wi-Fi network. The connection profile is deleted entirely.
- Use `wifi_saved` to list saved Wi-Fi networks the system remembers.

When the user says "connect to Wi-Fi" or "join a network", scan first with `wifi_scan` to show available networks, then ask for which one and the password. Never ask the user to repeat a password — if connection fails, report the error and ask them to try again.

## Network Interfaces

- Use `net_interfaces` to show all network interfaces, their status, and IP addresses. This combines `nmcli device status` and `ip -br addr` for a complete view.
- Always run `net_interfaces` first when diagnosing connectivity issues to understand the current state.

Present interface information as a clean table: interface name, type, state, IP address.

## IP Configuration

- Use `net_static_ip` to assign a static IP address to an interface. Requires the IP address (with CIDR notation) and gateway.
- Use `net_dhcp` to switch an interface back to automatic (DHCP) configuration. Clears any static IP settings.
- Use `net_dns` to set custom DNS servers for an interface. Accepts one or more DNS server addresses.

**These operations are destructive** — changing IP settings can disconnect the user. The guard system will automatically trigger a confirmation prompt before executing `net_static_ip`, `net_dhcp`, and `net_dns`.

When configuring a static IP, remind the user to include the subnet mask in CIDR notation (e.g., `192.168.1.100/24`).

## Diagnostics

- Use `net_ping` to test connectivity to a host. Default is 4 pings. Summarize the result: reachable/unreachable, average latency.
- Use `net_traceroute` to trace the route to a host. Show each hop with latency. This can take up to 60 seconds.
- Use `net_dns_lookup` to look up DNS records for a domain. Default record type is `A`. Common types: A, AAAA, MX, CNAME, TXT, NS.
- Use `net_ports` to show listening ports and the processes using them. Present as a table: protocol, address, port, process.

## Troubleshooting Flow

When a user reports "internet not working" or similar connectivity issues:

1. Run `net_interfaces` to check interface state and IP addresses.
2. If Wi-Fi is disconnected, scan and help reconnect.
3. If connected but no IP, check DHCP with `net_dhcp`.
4. If IP is present, run `net_ping 8.8.8.8` to test basic connectivity.
5. If ping works but websites don't load, run `net_dns_lookup google.com` to check DNS.
6. Use `net_traceroute` to identify where packets are being dropped.

## Error Handling

- **No Wi-Fi adapter**: Tell the user no wireless interface was found. Suggest checking if the hardware is present.
- **Wrong password**: Report authentication failure and ask the user to re-enter the password.
- **Network unreachable**: Check if the interface is up and has an IP address.
- **DNS resolution failure**: Suggest trying a different DNS server (e.g., 8.8.8.8 or 1.1.1.1).
