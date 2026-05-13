# kde-mcp

> An MCP server that gives any AI agent the ability to observe and control a KDE Plasma 6 / Wayland desktop — through programmatic primitives (AT-SPI, KWin Scripting, libei via KWin's private EIS interface) first, screenshots only as a fallback.

**Status:** design / specification phase. No code yet.
**Target:** KDE Plasma 6 on Wayland, KWin. Single-user, personal use first.
**Language:** Rust.
**License:** MIT.

## What this is

A computer-use tool in the same family as [Anthropic's Computer Use](https://docs.claude.com/en/docs/build-with-claude/computer-use), but:

- **Native to KDE/Wayland.** Anthropic's reference impl is X11/Xvfb inside Docker. This binary runs against your real (or a virtual) KDE session and uses KDE's first-party APIs.
- **Accessibility-tree first.** Pixels are an explicit fallback. Most actions are deterministic AT-SPI invocations, not coordinate clicks. Inspired by the Playwright-MCP philosophy.
- **Response-shaped behavior.** Tool responses carry the structured deltas that drive the agent's next decision; prompt-engineering instructions are minimised.
- **Two-process model.** `kde-mcp --mode=live` controls your real desktop. `kde-mcp --mode=virtual` spawns a headless `kwin_wayland --virtual` session (optionally with `wayvnc` so you can watch over VNC) for isolated automation. No runtime swap, no single-process dual-context fragility.

## Why not just use Anthropic's reference / kwin-mcp?

| Concern | Anthropic ref | kwin-mcp | kde-mcp (this project) |
|---|---|---|---|
| Display server | X11/Xvfb in Docker | Wayland (KDE) | Wayland (KDE) |
| Input mechanism | xdotool | libei (KDE EIS) | libei (KDE EIS) |
| Primary surface | screenshots | mixed | a11y tree first; pixels fallback |
| Distribution | Docker container | Python package | single Rust binary |
| MCP integration | none (direct API) | yes | yes |
| Response shape | text + screenshot per action | mostly screenshots | structured diffs, screenshots opt-in |
| Tool count | 3 (computer, bash, text_editor) | 30 | 15 |
| Mode model | one container = one X session | both modes in one process | one mode per process |

Full design rationale in `docs/specs/2026-05-13-kde-computer-use-mcp-design.md`.

## Quick orientation

- `docs/specs/` — the design specification (the load-bearing document).
- `docs/research/` — collated research that informed the design (Anthropic spec deep-dive, Wayland/KDE primitive landscape, prior-art survey, LLM tool-design principles).
- `docs/adr/` — Architecture Decision Records: one file per non-obvious choice with the reasoning frozen at the moment it was made.
- `docs/comparison-with-anthropic-computer-use.md` — side-by-side analysis.
- `packaging/` — `.desktop` file for KWin ScreenShot2 D-Bus authorization, install notes.
- `src/` — Rust source (empty until implementation phase begins).

## Hard scope limits

- Not multi-compositor. KDE only. (GNOME/wlroots are tractable later but not in this design.)
- Not multi-platform. Linux only.
- Not a browser-automation tool — use Playwright-MCP for browsers.
- Not a sandboxed-VM agent — use Cua for that.
- Not headed for community use *yet*. Personal-use MVP first; community polish later if the tool proves useful.

## Status of the work

This repository currently contains design artifacts only. Implementation will begin after the spec is reviewed.
