# ADR-0001: Use Rust

**Status:** Accepted
**Date:** 2026-05-13
**Deciders:** atassis

## Context

The project needs to talk to:
- D-Bus (KWin Scripting, ScreenShot2, EIS)
- libei (input emulation, via a file descriptor handed back from KWin's EIS interface)
- AT-SPI (accessibility tree)
- A PTY for persistent bash
- The MCP protocol over stdio

Prior art uses Python ([kwin-mcp](https://github.com/isac322/kwin-mcp)) or Node ([tanob/mcp-desktop-automation](https://github.com/tanob/mcp-desktop-automation)). The MCP spec has SDKs in Python, TypeScript, Rust, Go, Java, Kotlin, Swift.

## Decision

Implement in Rust.

## Rationale

| Need | Rust crate | Status (May 2026) |
|---|---|---|
| MCP server SDK | `rmcp` (official `modelcontextprotocol/rust-sdk`) | Stable; supports tools + image content blocks |
| D-Bus | `zbus` v4 | Excellent — async, codegen, type-safe |
| libei | `reis` | Maintained; works against libei 1.x |
| AT-SPI | `atspi` | Pure Rust, async, used by Odilia screen reader; arguably better than the Python `pyatspi` bindings |
| ARGB → PNG | `image` | Stable |
| OCR | shell out to `tesseract` binary | Equivalent across languages |
| Persistent PTY | `portable-pty` | Stable |
| Portal fallback (not needed in MVP) | `ashpd` | Stable |

`zbus` and `atspi` are *nicer* than the Python equivalents. Concrete benefits:
- Single static binary → simpler distribution (no Python/pip/venv at install time)
- Lower memory footprint (~20–40 MB vs Python's ~80–120 MB for an idle MCP server)
- Better perf on the libei event hot path (event-loop driven, no GIL)
- Strong typing catches the entire class of "wrong field name in D-Bus call" bugs

Costs:
- ~1.5–2× lines of code vs Python
- Longer iteration cycle (compile time, type system friction)
- Smaller community of MCP-server-in-Rust examples to learn from

For a personal tool the user will run daily, the binary's properties (cold start, footprint, distribution) matter more than dev velocity.

## Consequences

- Distribution: `cargo install` (or pre-built binary) — no runtime deps for the language itself.
- CI eventually: GitHub Actions matrix; Linux runners only.
- Contribution friction: Rust has steeper onramp than Python. Acceptable given the personal-use first stance.
- We commit to the `rmcp` SDK's design as it evolves; if it breaks badly we may need to vendor or fork.

## Alternatives considered

- **Python.** Fastest iteration, most prior art (kwin-mcp). Rejected for distribution awkwardness and footprint.
- **TypeScript / Node.** Most active MCP SDK ecosystem (microsoft/playwright-mcp). Rejected because the D-Bus / libei / AT-SPI bindings in Node are weaker than Rust's, and we'd be shelling out to libraries anyway.
- **Go.** Reasonable D-Bus support; MCP SDK exists but less mature. Rejected for weaker libei and AT-SPI bindings.
- **C / C++.** Best library availability but worst dev velocity and memory safety. Not considered seriously.
