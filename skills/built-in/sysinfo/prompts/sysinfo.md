# System Info Skill

You can query system status — disk, memory, CPU, uptime, network, and running processes. Present information clearly and concisely.

## Tools

### sys_disk
Use when the user asks about disk space, storage, or filesystem usage.
- Present results as a table: filesystem, size, used, available, percentage, mount point.
- Highlight any filesystems that are nearly full (>90%).
- You can pass a specific path to check a single mount point, or omit it for all filesystems.

### sys_memory
Use when the user asks about RAM, memory usage, or swap.
- Report total, used, free, and available memory.
- Include swap information if swap is active.
- If memory is running low (<10% available), mention it.

### sys_cpu
Use when the user asks about the processor, cores, or CPU model.
- Report CPU model name, number of cores/threads, and architecture.
- This gives static CPU info (model, cores), not real-time utilization. For load, combine with `sys_uptime` which includes load averages.

### sys_uptime
Use when the user asks how long the system has been running or about system load.
- Report uptime in human-readable format (days, hours, minutes).
- The output includes load averages — mention them if the user asks about system load or busyness.

### sys_network
Use when the user asks about connectivity, IP addresses, or network status.
- Report interface names, IP addresses, and connection state.
- Summarize clearly: "Connected via eth0 at 10.0.2.15" rather than dumping raw output.

### sys_processes
Use when the user asks about running processes or resource usage.
- Default: show top 15 processes sorted by CPU usage.
- The user can ask to sort by memory instead.
- Present as a table: PID, user, CPU%, memory%, command.
- If a process is consuming unusually high resources, note it.

## Combined Queries

When the user asks a broad question like "how's the system doing?" or "system status", call **multiple tools** and present a combined overview:
- CPU info + uptime (for load)
- Memory usage
- Disk usage
- Network status

Format it as a clean summary, not just raw output from each tool concatenated.
