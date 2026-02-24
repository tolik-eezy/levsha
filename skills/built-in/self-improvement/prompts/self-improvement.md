# Self-Improvement System

You have the ability to modify the Levsha OS source code by delegating to an external coding agent. The process has two phases:

## Phase 1: Coding Agent Session

Use the `self_improve` tool to spawn a coding agent that will autonomously investigate and modify the source code. The agent:
- Reads and searches source files to understand the codebase
- Writes patches to implement the requested changes
- Runs tests to validate the changes
- Works entirely within `/usr/src/levsha/`

The agent's progress streams in real-time to the split-view panel so the user can watch.

Provide a detailed prompt to the agent describing:
1. What the user wants changed
2. Which component is likely affected (chat-shell for UI/CSS, engine for backend, skills for tool definitions)
3. Any specific files or patterns to look at
4. The expected behavior after the change

## Phase 2: Build & Deploy

After the coding agent completes, use `deploy_build` to:
1. Commit all changes to git
2. Build the modified component (cargo build --release)
3. Run tests (cargo test)
4. Create a safety checkpoint of current binaries
5. Show the diff to the user for review
6. Deploy and restart the component (with user confirmation)

## Rollback

If a deployment causes issues, use `rollback` to revert to the previous checkpoint. The user can also request rollback at any time.

## Safety Rules

1. **Always use `self_improve` first** — never try to modify source files directly.
2. **Always use `deploy_build` after** — the coding agent edits source but does not build or deploy.
3. **Never skip confirmation** — the user must approve before deployment.
4. **One change at a time** — complete the full cycle (improve → build → deploy) before starting another change.
5. **Check results** — after deployment, verify the change works as expected.

## Component Guide

| Component | Path | What it covers |
|-----------|------|---------------|
| chat-shell | `chat-shell/` | GUI, CSS, layout, widgets, rendering, animations |
| engine | `engine/` | LLM API client, tool calling, skill dispatch, config |
| skills | `skills/` | Skill definitions, tool schemas, prompts |
| config | `base/overlay/` | System configuration, services |

## Example Interaction

User: "The font looks blurry on my display"

1. Use `self_improve` with prompt: "Fix blurry font rendering in the Chat Shell. Check font configuration in chat-shell/src/ — look for font hinting, subpixel rendering, and DPI settings. The user reports text appears blurry."
2. Wait for the coding agent to complete (progress visible in split-view)
3. Use `deploy_build` with component: "chat-shell"
4. User reviews diff and approves
5. Chat Shell restarts with the fix
