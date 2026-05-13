# kde-mcp vs Anthropic Computer Use — Are We Better?

A side-by-side analysis comparing this project's design against Anthropic's first-party Computer Use tool (current generation: `computer_20251124` + `bash_20250124` + `text_editor_20250728`).

**TL;DR:** Better for Linux/KDE workflows along almost every axis that matters for our use case. Worse on three axes: portability, training priors, and Anthropic's server-side prompt-injection classifier — and we don't care about those three for this project.

---

## Scoring rubric

For each dimension, one of:
- ✅ **kde-mcp is better** — measurably or structurally.
- ⚖️ **Tie / contextual** — neither dominates; depends on what you value.
- ❌ **Anthropic is better** — they have something we either can't have or chose not to chase.

---

## 1. Native Linux Wayland support

✅ kde-mcp.

Anthropic ships an X11/Xvfb session inside a Docker container. To use it against your Linux desktop, you either run the agent in the container (which doesn't see your real apps) or build your own bridge. There is no first-party Wayland integration.

kde-mcp runs against the user's real KWin (or a private virtual KWin) via D-Bus and libei — the modern Wayland-native input emulation stack. No Docker, no Xvfb, no xdotool.

## 2. KDE-native fast paths

✅ kde-mcp.

We use three KDE-private interfaces that are unique to KWin:

- `org.kde.KWin.EIS.RemoteDesktop` — hands us a libei fd directly, **skipping the xdg-desktop-portal consent dialog** that GNOME and wlroots make agents pay every session.
- `org.kde.KWin.ScreenShot2` — 30–70ms screenshot capture, far cheaper than the portal ScreenCast PipeWire route.
- `org.kde.KWin.Scripting.loadScript` — full window query and control via injectable JavaScript, things Wayland's public protocols don't expose.

Anthropic's reference impl doesn't have a Wayland-native fast path at all — it's X11 by construction.

## 3. Accessibility tree as primary surface

✅ kde-mcp.

Anthropic's tool surface is screenshot-and-coordinate-first. Every action triggers a screenshot after a 2-second delay; the model reasons about pixels and clicks at `(x, y)`. This has measurable problems: coordinate hallucination, stale state, token cost (500–1000 tokens per screenshot), failure under theme/scaling changes.

kde-mcp's primary action is `act_on_element(element_id, action)` — an AT-SPI `do_action()` invocation. For native Qt apps, GTK4 apps, well-implemented Firefox, and increasingly Chromium/Electron, this **bypasses input emulation entirely** and triggers the side effect directly via D-Bus. Token cost: ~10–50 tokens. Reliability: deterministic.

Pixels remain available as `screenshot` + `mouse`, but they are explicit fallbacks marked `vision_fallback: true` in the response.

## 4. Response shape carries decision-relevant info

✅ kde-mcp.

Anthropic's tool result for any computer action is a screenshot. The model decodes pixels to see what happened. Every action.

kde-mcp's action tools return structured diffs:

```jsonc
{ "ok": true, "diff": { "windows_added": [{"ref": "win#9", "title": "Save File"}], "focused_element": { "before": "el#a4", "after": "el#b1" } } }
```

No pixels needed to know "a new dialog appeared." The model continues directly. We documented this as ADR-0004 (Response-as-instruction).

## 5. Token efficiency per agent turn

✅ kde-mcp.

Order-of-magnitude estimates:

| Surface | Anthropic | kde-mcp |
|---|---|---|
| Tool definitions in `tools/list` (Anthropic native) | 735 (computer) + 245 (bash) + 700 (text_editor) = **1680 tokens** | ~15 tools × ~80 tokens each ≈ **1200 tokens** |
| Per-action response (after a click) | base64 PNG of full screen (1024×768) ≈ **~700 tokens** | structured diff (`windows_added`, `focused_element`) ≈ **~100 tokens** |
| Reading a button label | screenshot + vision parsing ≈ **~700 tokens** | `inspect` returns `name: "Save"` ≈ **~30 tokens** |
| 10-action sequence | 10 × screenshot + thinking ≈ **~8000 tokens** | 10 × diff + occasional inspect ≈ **~1500 tokens** |

The compounding savings on a long agent task are dramatic.

## 6. Coordinate hallucination

✅ kde-mcp.

Anthropic's tool requires the model to nominate pixel coordinates. Models miss small targets, drift on fractional-scaled displays, and lose accuracy when screenshots are auto-scaled to fit the 1568px / 1.15 Mpx API cap (the reference impl tries to handle scaling correctly; the docs note misses still happen).

kde-mcp's primary action takes opaque element refs (`el#a4f2`) that **must come from a recent observe/inspect**. Fabricated coordinates are not in the tool's schema. Pixel coordinates are accepted only by the explicit `mouse` tool, which marks every call `vision_fallback: true`.

We apply poka-yoke (ADR-0004): the wrong action is structurally impossible to express, not just "documented as bad."

## 7. Long-text input speed

✅ kde-mcp.

Anthropic's `type` action issues per-character keystrokes at ~12 ms each, chunked 50 chars per round-trip. Typing 500 characters: ~6 seconds + 10 round-trips.

kde-mcp's `keyboard.type` uses `use_clipboard: "auto"` — for text >32 chars or non-ASCII, it `wl-copy`s the text and sends Ctrl+V. Save/restore of the original clipboard happens transparently. 500 characters: ~50 ms + 1 round-trip.

## 8. Unicode / non-Latin / emoji input

✅ kde-mcp.

`xdotool type "café"` depends on the X11 keymap; failure mode is "missed characters" or "wrong characters." Anthropic's reference impl ships with this footgun.

`wl-copy` + paste preserves any Unicode codepoint exactly. The clipboard path solves the keymap problem entirely.

## 9. Window-level operations

✅ kde-mcp.

Anthropic's surface has no "list windows," no "focus window," no "get geometry." The model reasons about windows purely through screenshots, which means tasks like "find my open Firefox" require visual search.

kde-mcp exposes:
- `window_query {action: "list"|"find"|"get_geometry"}` — programmatic window listing
- `window_control {action: "focus"|"move"|"resize"|"minimize"|"maximize"|"close"|...}` — direct manipulation via KWin scripting

Hard tasks become trivial: "focus my Firefox window" is one tool call, not a screenshot-and-search loop.

## 10. Structured waits

✅ kde-mcp.

Anthropic offers a `wait` action that sleeps for N seconds. The reference impl additionally has a hard-coded 2-second delay after every action.

kde-mcp's `wait` takes structured conditions: `screen_change`, `window_appears`, `window_disappears`, `element_appears`, `element_disappears`, `element_state`. Each returns matched window/element data and elapsed_ms. The agent only waits as long as it has to.

## 11. Sandboxing / virtual session

✅ kde-mcp (with caveats).

Anthropic's reference impl is itself a sandboxed Docker container; that's the only mode. To run against your real desktop, you'd have to wire up your own X11 forwarding.

kde-mcp ships two modes: `--mode=live` (your real KDE session) and `--mode=virtual` (a private headless `kwin_wayland --virtual` with optional `wayvnc` for visual monitoring). One binary, mode chosen at startup.

Caveat: per-UID shared state (PipeWire audio, libsecret keyring, Firefox/Chrome profile locks) is shared across both modes. This is a Linux fact, not a design choice. ADR-0003 documents the tradeoff.

## 12. Fault isolation

✅ kde-mcp.

Anthropic's container model gives strong isolation but high overhead. kde-mcp's two-process model (live + virtual as separate MCP server entries) gives lightweight OS-process fault isolation: if virtual KWin dies, live MCP server is unaffected.

## 13. Audit trail

✅ kde-mcp.

Anthropic's reference impl logs to the Streamlit UI but not in a structured file. To audit a session you read the chat history.

kde-mcp writes JSONL audit log to `~/.local/state/kde-mcp/audit.jsonl` with one line per tool call (timestamp, tool, args, result summary, errors, duration). `tail -f` for live monitoring. Standard log rotation. Verbatim `bash` commands kept; large file content hashed.

## 14. Permission/safety integration with host

✅ kde-mcp.

Anthropic's tool is one type (`computer`) with one big action enum — the host can either allow all or none. No per-action permissioning.

kde-mcp splits tools along the safety axis: 8 read-only tools (auto-approve in Claude Code's default mode) + 7 destructive tools (prompt). The host's permission system works naturally; we don't build a parallel confirmation layer.

## 15. Prompt-injection trust marking

✅ kde-mcp.

Anthropic ships a server-side classifier (see "Anthropic is better, dimension 1" below). That handles ambient injection well.

kde-mcp adds a structural defense: every model-readable string from outside the agent's authorship (window content, AT-SPI labels, clipboard read, OCR text, bash stdout, file content read) is wrapped with `trust: "external"`. The model sees data marked as data, not as instruction. This is structurally enforced — the agent can't override it.

## 16. Extensibility

✅ kde-mcp.

Anthropic's tool schema is **baked into the model**. You cannot add an action without retraining or moving to your own tool definitions.

kde-mcp is a regular MCP server. Adding a new tool is a new module under `src/tools/`. The principles in §4 of the spec guide what shape the new tool should take.

## 17. Distribution / install

✅ kde-mcp.

Anthropic: pull a Docker image (~2 GB), expose ports 8080/8501/6080, mount volumes. Not friendly for local-real-desktop use.

kde-mcp: `cargo install --path .` produces a single ~30 MB static binary. One `.desktop` file install. Done.

---

## Where Anthropic is better

### ❌ 1. Server-side prompt-injection classifier

Anthropic runs a classifier on prompts containing computer-use tools and forces the model to ask for confirmation when it detects an injection attempt. This is a defense-in-depth layer we don't have. Our structural defense (trust wrappers, ADR-0004) helps but doesn't equal a classifier with training data on actual attacks.

Mitigation: rely on the host's permission mode + audit log + sensitive-path warnings. Plus the trust markers.

### ❌ 2. Training priors on the action vocabulary

Claude 3.5 Sonnet onward is **trained** on `computer_20251124`'s exact JSON shape and action names. Calling `{"action": "left_click", "coordinate": [x, y]}` is something the model has seen during training. The 735-token compact tool definition reflects this — Claude doesn't need a JSON Schema for `computer`, it knows the schema.

kde-mcp pays full JSON Schema tokens for every tool definition (~80 tokens × 15 tools ≈ 1200 tokens, less than Anthropic's 1680 but with less training assistance). For pure coordinate-clicking flows, Anthropic on Claude may still have an edge.

Mitigation: our a11y-first design avoids most coordinate-clicking, so we land in a regime where training priors on Anthropic's vocabulary matter less. We also pick action names (`mouse.click`, `keyboard.type`) that overlap with what Claude has seen in many MCP-server tools.

### ❌ 3. Portable across OSes (via container)

Anthropic's Docker container runs on macOS, Windows-with-WSL, and Linux equally. kde-mcp is KDE-Wayland-only, by design (ADR-0002).

This isn't a flaw in kde-mcp; it's the explicit scope tradeoff. For Linux/KDE users we are strictly better. For users on other platforms, Anthropic's Docker route is the only path (or use Cua, Playwright-MCP, etc.).

---

## Things that are honestly a tie

### ⚖️ 1. Bash and text_editor

Both projects implement essentially the same `bash` + `text_editor` semantics. We added a small blocklist (`rm -rf /`, etc.) for oh-shit prevention; Anthropic relies on the user's confirmation prompts. Neither is meaningfully better.

### ⚖️ 2. Multi-monitor handling

Both pipelines have to deal with monitor layouts, fractional scaling, and origin conventions. KWin's logical-pixel coordinate model is actually cleaner than X11's, but Anthropic's reference impl runs in a single Xvfb, so they sidestep the problem.

### ⚖️ 3. Cursor visibility in screenshots

Anthropic's reference impl uses `gnome-screenshot -p` (cursor) → fallback `scrot -p`. kde-mcp uses `org.kde.KWin.ScreenShot2` with `include-cursor: true` (default). Same outcome.

---

## Overall verdict

We are better for the use case we set out to serve. The dimensions where Anthropic wins (portability, training priors, server-side classifier) are exactly the dimensions we deliberately don't compete on.

The biggest single win is **#3 (accessibility-tree first)** combined with **#4 (response shape carries decisions)**. Together, these change the agent's behaviour from "screenshot every step, hallucinate coordinates" to "query the tree, invoke actions directly, react to deltas." On a 20-step task, that's the difference between burning ~20 screenshots × 700 tokens (~14,000 tokens) and burning ~2 screenshots + 20 diffs (~3000 tokens). 4–5× cheaper, more reliable, and the actions hit the intended targets every time.

The biggest risk we accept is **#2 (training priors)**: if a model's pretraining is heavily tilted toward the Anthropic `computer_20251124` schema and weakly toward a11y-tree-first invocations, our design pays for it. Mitigation: our tools use idiomatic MCP shapes the model has seen widely. Empirically this is fine; if it ever becomes a problem we can ship a thin "Anthropic-vocabulary compatibility" tool alongside.

## When to **not** use kde-mcp

- You run GNOME or any non-KDE compositor.
- You need to drive macOS or Windows.
- You need browser-only automation (use Playwright-MCP).
- You need sandboxed-VM agent execution (use Cua).
- You don't trust the agent at all and want Anthropic's classifier defense as a baseline. (Even then, you should layer kde-mcp's structural trust markers on top.)

For everything else on a KDE Plasma 6 Wayland desktop: yes, this design is better than what Anthropic ships.
