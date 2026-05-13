# ADR-0003: Mode per process; no runtime swap

**Status:** Accepted
**Date:** 2026-05-13
**Deciders:** atassis

## Context

The server supports two operating modes:
- **`live`** — controls the user's real KDE session.
- **`virtual`** — spawns a headless `kwin_wayland --virtual` session for isolated automation.

An obvious-sounding design is "one process, two backends, a `session.switch` tool to flip between them at runtime." This was the initial proposal. After deeper research, the design changed to "one mode per process; configure two MCP server entries if you want both available."

## Decision

The mode is fixed at process start via `--mode=live | --mode=virtual` and is immutable for the life of the process. Users who want both modes available in one MCP host configure two server entries.

## Rationale

Research surfaced several hard problems with the dual-context-in-one-process model:

1. **Cross-session identifier invalidation.** AT-SPI element IDs are D-Bus-bus-unique. KWin window UUIDs are KWin-instance-unique. A handle obtained in live mode is meaningless in virtual mode. Every tool would need to validate which session a handle belongs to and reject mismatches — silent misrouting bugs would be the result if we got it wrong.

2. **Per-UID shared state is not solvable by `dbus-run-session`.** PipeWire audio, libsecret/kwallet, Firefox/Chrome/Electron profile locks, SSH agent, and IBus/fcitx are per-user-not-per-bus. These hazards exist for any dual-session architecture; the runtime-swap design does not improve them, only adds internal complexity on top.

3. **The pattern is untested in the wild.** kwin-mcp exposes both `session_start` (virtual) and `session_connect` (live) but no bug report, blog post, or community thread describes operating both concurrently. The dual-mode-in-one-process flow inherits any future bug in either backend.

4. **OS-process boundary gives free fault isolation.** Two processes is two crash domains. If virtual KWin dies, the live MCP server is unaffected.

5. **The supposed UX benefit is mostly hypothetical.** "Research in live, reproduce in virtual within one conversation" is a workflow nobody has been documented requesting. Per-task / per-config selection is the dominant pattern.

## Consequences

- Two MCP server entries to configure if both modes are desired:
  ```jsonc
  {
    "kde-live":    { "command": "kde-mcp", "args": ["--mode=live"] },
    "kde-virtual": { "command": "kde-mcp", "args": ["--mode=virtual", "--vnc=5910"] }
  }
  ```
- No `session` tool. No namespaced IDs. No `session_epoch`. No active-context plumbing.
- Tool implementations take a single backend reference — no `&Context` parameter threading. Less surface area for subtle bugs.
- Audit log columns are simpler: process = mode. No `session_mode` column needed.
- Switching modes requires asking the host to call the other server (Cline/Claude Code do this naturally) — no in-process migration.

## Alternatives considered

- **Option 1: Runtime-swappable inside ONE MCP process.** Initial proposal. Rejected for the reasons in §Rationale.
- **Option 3: One mode per process, restart to change.** Simpler than Option 2 but loses the "both modes available without restart" property. Rejected in favour of Option 2 since the cost over Option 3 is ~40 MB extra RAM (one idle Rust binary).

## Research artifacts

The deeper research is summarised in [docs/research/wayland-kde-primitives.md](../research/wayland-kde-primitives.md) and the conversation history is captured in the design doc's §5.1.
