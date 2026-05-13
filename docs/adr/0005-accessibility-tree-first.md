# ADR-0005: Accessibility-tree first; pixels as fallback

**Status:** Accepted
**Date:** 2026-05-13
**Deciders:** atassis

## Context

There are two ways to reason about and act on a desktop UI:
1. **Pixel-based** — capture screenshots, ask the model to identify regions visually, click at `(x, y)` coordinates.
2. **Structured-tree-based** — query the platform's accessibility API (AT-SPI on Linux), get every widget's role/name/value/state/bbox + the actions it supports, invoke actions by element ID.

Anthropic's Computer Use takes path #1 by default (it screenshots after every action). Microsoft's Playwright-MCP takes path #2 by default, with pixels behind an opt-in `--caps=vision`.

## Decision

`kde-mcp` takes path #2. AT-SPI is the primary surface; screenshots are an explicit, opt-in fallback for cases the tree can't express.

The killer primitive is `act_on_element(element_id, action)` — invokes AT-SPI `do_action()` directly. No coordinate hallucination, no input emulation race, deterministic.

## Rationale

- **Coordinate hallucination is real.** Models systematically miss small UI targets when asked to "click on the Save button at...". Screenshots after-the-fact also tend to be 1–2s stale relative to the model's last-seen frame.
- **Token efficiency: 5–30×.** A screenshot is 500–1000 tokens. An AT-SPI tree of the focused window is 50–300 tokens.
- **Resilience to visual change.** Theme changes, fractional scaling, monitor switching all break pixel-based agents. The a11y tree is invariant.
- **`do_action()` can bypass input emulation entirely.** For native Qt and well-implemented GTK apps, invoking the action via AT-SPI doesn't even move the cursor or emit key events. Pure D-Bus call → desired side effect.
- **Linux's a11y story is good enough.** Qt apps: excellent. GTK4: reasonable. Firefox: reasonable (2026 state). Electron/Chromium: spotty but improving. The spotty cases trigger the fallback path naturally.

## Concrete consequences

- Tool surface is structured around `act_on_element` as primary; `mouse` is the fallback.
- `observe` and `inspect` return AT-SPI tree info, not screenshots. `screenshot` exists but the agent should reach for `observe` first.
- Element refs are opaque IDs (validated server-side); the model **cannot** pass coordinates to `act_on_element` — only the explicit `mouse` tool accepts coordinates.
- When AT-SPI is empty for an app (canvas, video player, raw OpenGL, broken Electron), `inspect` returns `warnings: ["AT-SPI tree empty — vision fallback recommended"]` and the model naturally switches to `screenshot` + `mouse`.
- The `mouse` tool's response includes `vision_fallback: true` so the audit log can count uses and we can identify apps that need attention.

## Consequences for build effort

- AT-SPI integration is more work than xdotool wrapping. We pay this cost up front.
- We become dependent on `at-spi2-core` being installed and the user's session having an a11y registry.
- Some apps will need user intervention to enable a11y (Chromium `--force-renderer-accessibility`, etc.). Documented in the install guide.

## Alternatives considered

- **Pixels-first like Anthropic.** Easier to implement, broader app coverage, but pays the coordinate-hallucination + token-cost tax on every call. Rejected as primary surface.
- **Both equal weight.** The model would have to pick per call; better to set the default reflex.
- **Vision-LLM-assisted parsing (OmniParser-style) over screenshots.** Adds a second model in the loop. Rejected as too heavyweight for an MVP MCP server; the agent IS the vision model.

## Source

Playwright-MCP's README:
> "Uses Playwright's accessibility tree, not pixel-based input. ... Deterministic tool application. Avoids ambiguity common with screenshot-based approaches."

isac322/kwin-mcp implements both surfaces with `accessibility_tree` and `screenshot` tools at parity; we lift this further by making the tree the default.
