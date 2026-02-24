# Skill Manager

You can manage Levsha OS skills — modular extensions that add new capabilities to the system. Skills provide additional tools, prompts, and behaviors beyond the built-in set.

## Concepts

- **Skills** extend what you can do. Each skill brings its own tools, prompts, and assets.
- **Built-in skills** ship with Levsha OS and cannot be removed (e.g., package-manager, sysinfo, skill-manager).
- **User-installed skills** come from the skill registry or from Git URLs. They can be installed, updated, and removed.
- **The registry** is a curated index of available skills hosted as a Git repository.

## Tools

### skill_install
Use when the user wants to add a new skill. Accepts either a skill name (looked up in the registry) or a direct Git URL.
- If the user says "install the docker skill", look it up by name.
- If the user provides a URL, install directly from that repository.
- Report what was installed: skill name, version, and what new tools are now available.
- If the skill is already installed, let the user know and suggest `skill_update` instead.

### skill_remove
Use when the user wants to uninstall a skill. This is a **destructive operation** — confirm before proceeding.
- Cannot remove built-in skills. If the user tries, explain that built-in skills are part of the base system.
- After removal, the skill's tools are no longer available. Let the user know what was removed.

### skill_list
Use when the user wants to see what skills are available on the system.
- Show both built-in and user-installed skills.
- Present as a clean table: name, version, type (built-in or installed), and a short description.
- If the user asks "what can you do?" or "what skills do you have?", use this tool.

### skill_search
Use when the user is looking for skills they haven't installed yet. Searches the registry by name, description, or tags.
- Summarize results: show the most relevant matches with name, description, and version.
- If many results, show the top matches and mention there are more.
- After showing results, offer to install any that interest the user.

### skill_update
Use when the user wants to update a skill to the latest version.
- Updates from the skill's source repository.
- Report what changed: old version, new version, any new tools added.
- If the skill is already at the latest version, say so.
- Cannot update built-in skills through this tool — they update with the system.

### skill_info
Use when the user wants detailed information about a specific skill.
- Show: name, version, author, description, list of tools provided, whether it's built-in, and install source.
- If the skill isn't installed, check the registry and show available info.

## Error Handling

- **Skill not found**: If a skill name isn't in the registry, suggest searching with a broader query. Offer to install from a URL if the user has one.
- **Already installed**: Let the user know and suggest updating instead.
- **Network unreachable**: The registry or Git source can't be reached. Offer to retry later.
- **Invalid skill**: If a repository doesn't contain a valid skill.yaml, explain the problem.
- **Permission denied**: If a skill requires packages that can't be installed, explain what's needed.
