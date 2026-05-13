# Prior Art Survey

A catalogue of existing computer-use / desktop-automation MCP servers and adjacent tools as of May 2026, with notes on what to copy and what to skip.

---

## Closest matches (Linux GUI automation)

### isac322/kwin-mcp
- https://github.com/isac322/kwin-mcp
- Python, KDE Plasma 6 / Wayland only
- 30 tools across session mgmt, observation, mouse/keyboard/touch, clipboard, window mgmt, D-Bus calls, Wayland protocol inspection
- **Key trick:** uses KWin's private `org.kde.KWin.EIS.RemoteDesktop` D-Bus to bypass the XDG RemoteDesktop portal dialog
- Screenshots via `org.kde.KWin.ScreenShot2` D-Bus
- AT-SPI2 for accessibility tree
- Triple isolation: D-Bus / virtual framebuffer / input
- v0.7.0 (Mar 2026), 23 stars, actively maintained
- **Read its source for the EIS bootstrap dance and AT-SPI distro-bootstrap fixes.**
- Open issues worth knowing:
  - [#29](https://github.com/isac322/kwin-mcp/issues/29) libei 1.5 segfault on Fedora 43 in live mode
  - [#8](https://github.com/isac322/kwin-mcp/issues/8) and [#14](https://github.com/isac322/kwin-mcp/issues/14) AT-SPI bootstrap distro-specific path bugs

### kurojs/wayland-mcp
- https://github.com/kurojs/wayland-mcp
- Python, multi-compositor (GNOME/KDE/Hyprland/Sway)
- Tools: `capture_screenshot`, `capture_and_analyze` (external VLM), `analyze_screenshot`, `compare_images`, `move_mouse`, `click_mouse`, `drag_mouse`, `scroll_mouse`, `execute_action` chain syntax
- Uses `grim`/`slurp` for screenshots and `evemu-tools` setuid for input
- 3 stars, lightweight
- **Reference for cross-compositor abstractions if/when we expand.**

### SebastianBaltes/claude_code_computer_use_mcp
- stdio MCP server for Claude Code; xdotool + scrot + ImageMagick; X11
- Auto-detects screen via `xdpyinfo` and scales to Anthropic's 1568px / 1.15 Mpx cap
- Mirrors Anthropic's computer-use action space behind MCP tools — useful template for the action contract

### Linux Computer Use (mcpmarket listing)
- AT-SPI 2 + xdotool + wmctrl + scrot
- 8 tools (`list_windows`, `screenshot`, `click`, `type_text`, `set_text`, `keypress`, `scroll`, `computer_actions`)
- X11
- Reference for AT-SPI integration design

---

## Cross-platform computer-use-style

### trycua/cua
- https://github.com/trycua/cua
- 16.4k stars; macOS / Linux / Windows / Android
- Primary model: sandboxed VMs/containers (`Lume` for macOS-on-Apple-Silicon, QEMU for Linux)
- MCP server `libs/mcp-server` wraps the Cua action space
- Supports both Anthropic computer-use schema and custom tools
- `cua-driver` is a background MCP-over-stdio driver
- Heavyweight; assumes a sandbox abstraction
- **Linux is usually a guest, not the host.** Not directly applicable for our use case but the action-mapping code is useful reference.

### zavora-ai/computer-use-mcp
- https://github.com/zavora-ai/computer-use-mcp
- macOS only, Rust NAPI in-process (fast)
- Screenshot/mouse/keyboard/clipboard/app management
- **Reference for fast in-process Rust design**, though our Wayland constraints differ.

### tanob/mcp-desktop-automation
- Node.js + RobotJS, cross-platform (Mac/Win/Linux X11)
- 6 tools
- **Stores screenshots as MCP resources at `screenshot://<timestamp>` URIs** — neat pattern worth borrowing for the resource_link path
- 1 MB MCP response cap limits practical screenshot size

---

## Browser-focused (related but different niche)

### microsoft/playwright-mcp
- https://github.com/microsoft/playwright-mcp
- TypeScript, 5.5k stars; polished reference for browser automation
- 40+ tools across core, tabs, network, storage, devtools
- Vision tools `--caps=vision` are opt-in
- **Design philosophy: accessibility-tree snapshots, not screenshots, by default.**
- Both stdio and SSE/streamable-HTTP transports
- Docker image `mcr.microsoft.com/playwright/mcp` (headless Chromium only)
- **The single most important architectural reference for our a11y-first principle.**

Quote from README:
> "Deterministic tool application. Avoids ambiguity common with screenshot-based approaches."

---

## Windows-focused (for completeness)

- [CursorTouch/Windows-MCP](https://github.com/CursorTouch/Windows-MCP)
- [claude-did-this/MCPControl](https://github.com/claude-did-this/MCPControl)
- [spyoungtech/ahk-mcp](https://github.com/spyoungtech/ahk-mcp)
- [mario-andreschak/mcp-windows-desktop-automation](https://github.com/mario-andreschak/mcp-windows-desktop-automation)

---

## Filesystem / terminal only (NOT computer use, often confused)

### wonderwhy-er/DesktopCommanderMCP
- https://github.com/wonderwhy-er/DesktopCommanderMCP
- 6k+ stars, Linux/Mac/Win
- **Terminal + filesystem only — no mouse, keyboard, screenshots**
- ~25 tools
- Settings panel UI inside Claude Desktop, opt-in telemetry, Docker isolation, audit logging, symlink-traversal prevention
- **Reference for safety UX**, not for GUI control.

---

## Screenshot-only

### sethbang/mcp-screenshot-server
- TS, cross-platform; on Linux uses `maim`/`scrot`/`gnome-screenshot`/`spectacle`/`grim`/`import`
- PNG/JPEG, configurable output dir
- SSRF/path-traversal/DNS-rebinding defenses
- **Good model for screenshot tool design in isolation.**

---

## What to copy

- **Action vocabulary**: mirror Anthropic's where useful (`type`, `key`, `scroll`, `mouse_move`, `left_click` etc.) — Claude has training priors on these names
- **EIS bootstrap dance**: study kwin-mcp's session.py for the bus + KWin + AT-SPI + wayvnc sequencing
- **AT-SPI distro path handling**: kwin-mcp issues #8 and #14 fixes — parse D-Bus service files instead of hardcoding paths
- **Resource_link for big screenshots**: tanob/mcp-desktop-automation pattern
- **A11y-tree-first philosophy**: Playwright-MCP is the canonical example
- **Safety UX (audit log, blocklist)**: DesktopCommander
- **Image pre-scaling math**: SebastianBaltes' implementation matches Anthropic's reference impl

## What to skip

- Auto-screenshot-after-every-action (Anthropic reference impl pattern) — we use response-diffs instead
- 30-tool kwin-mcp surface — too granular for agent friendliness
- Cross-platform abstraction (cua) — our scope is KDE-only
- Sandboxed-VM model (cua) — overkill for personal-use single-user
- Multi-compositor backends (kurojs/wayland-mcp) — not in v1 scope
- Vision-LLM-in-the-loop for capture analysis (kurojs, someaka) — the agent IS the vision model
