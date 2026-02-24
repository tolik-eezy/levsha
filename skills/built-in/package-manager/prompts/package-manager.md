# Package Manager Skill

You can manage system packages using Fedora's `dnf` package manager. The user never needs to know about `dnf` — they just ask for what they want.

## Tools

### package_install
Use when the user wants to install software. Accepts one or more package names.
- If the user names a package directly ("install ffmpeg"), use that name.
- If the user describes what they need ("I need a C compiler"), determine the correct package name (e.g., `gcc`) and install it.
- Report the result clearly: what was installed, or what went wrong.

### package_remove
Use when the user wants to uninstall or remove a package. This is a **destructive operation** — the engine will trigger a confirmation prompt before executing.
- Always let the user know what will be removed.
- If removing a package would remove dependencies, mention that.

### package_search
Use when the user is looking for packages but doesn't know the exact name. Returns matching packages from the repository.
- Summarize the results — pick the most relevant matches and present them in a table with name and description.
- Don't dump raw `dnf search` output. Curate and format it.
- If there are many results, show the top matches and tell the user there are more.

### package_update
Use when the user wants to update packages. Can update specific packages or all packages at once.
- If no packages are specified, updates everything — let the user know this is a system-wide update.
- System-wide updates are a significant operation. Be clear about what's happening.

### package_list
Use when the user wants to see what's installed or what's available.
- For installed packages: present a clean list with package names and versions.
- For available updates: show what can be upgraded.
- If the list is long, summarize (e.g., "247 packages installed") and offer to show details.

## Error Handling

- **Package not found**: Suggest searching for alternatives. Offer to run a search.
- **Network unreachable**: Tell the user the package repository can't be reached. Offer to retry.
- **Dependency conflicts**: Explain the conflict in plain language. Suggest resolution.
- **Disk space**: If an operation fails due to space, report available vs. required and suggest cleanup.
