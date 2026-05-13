# Linux Wayland / KDE Primitives — Research Notes

The landscape of input injection, screen capture, window queries, and accessibility on Linux desktops as of 2025–2026, focused on what works for KDE Plasma 6 on Wayland.

---

## Headline

For a KDE Plasma 6 Wayland MCP server, the working stack (no portal dialogs, low latency, KDE-native) is:

| Need | API | Implementation hint |
|---|---|---|
| Input injection | `org.kde.KWin.EIS.RemoteDesktop` D-Bus → libei fd | KDE-private interface; bypasses xdg-desktop-portal |
| Screenshots | `org.kde.KWin.ScreenShot2` D-Bus | Authorized via `.desktop` file with `X-KDE-DBUS-Restricted-Interfaces` |
| Window queries / control | `org.kde.KWin.Scripting.loadScript` | Eject script, load, run, capture output, unload |
| Accessibility tree | AT-SPI2 D-Bus (`org.a11y.atspi.*`) | Same on every Linux desktop |
| Clipboard | `wl-clipboard` (`wl-copy`/`wl-paste`) | Klipper D-Bus as monitoring fallback |

---

## Input injection — protocol landscape

| Protocol | KWin (Plasma 6) | Mutter (GNOME) | wlroots |
|---|---|---|---|
| `zwlr_virtual_pointer_v1` | **No** | No | Yes |
| `zwlr_virtual_keyboard_v1` | **No** | No | Yes |
| `zwp_virtual_keyboard_v1` (unstable) | **Yes** (since Plasma 5.7) | Yes | Yes |
| `zwp_input_method_v2` | Yes | Yes | Yes |
| `org_kde_kwin_fake_input` (KDE-private) | Yes | No | No |
| `xdg-desktop-portal` RemoteDesktop + libei | Yes (via xdp-kde) | Yes (Mutter 45+) | Partial |
| Direct D-Bus `org.kde.KWin.EIS.RemoteDesktop` | **Yes** (private, no portal dialog) | — | — |
| `/dev/uinput` (kernel, bypasses Wayland) | Always works | Always works | Always works |

KWin deliberately did not implement wlr-* virtual input protocols — it has its own `kde-fake-input` and a libeis backend ([KWin MR !5496](https://invent.kde.org/plasma/kwin/-/merge_requests/5496)).

### Tool details

**`ydotool`** ([ReimuNotMoe/ydotool](https://github.com/ReimuNotMoe/ydotool)): uses `/dev/uinput` directly. Requires daemon (`ydotoold` since v1.0.0) and either root or `input` group + udev rule. Layout-agnostic — sends raw evdev codes which the compositor maps through its xkb. Latency ~5–15ms after daemon warmup.

**`wtype`** ([atx/wtype](https://github.com/atx/wtype)): uses `zwp_virtual_keyboard_v1`. Works on KWin for typing (KWin implements that protocol). Does **not** support mouse on KWin. Generates transient keymap so Unicode/emoji typing works.

**`kdotool`** ([jinliu/kdotool](https://github.com/jinliu/kdotool)): KDE-only, **does not inject input**. Defers to ydotool/wtype. It does window queries via KWin Scripting (loadScript pattern).

**libei / libeis**: the modern, security-aware emulated-input stack. Three pieces: `libei` (client), `libeis` (server, integrated into compositor), `liboeffis` (portal D-Bus glue). Sub-millisecond latency. Adopted by:
- Mutter 45+ (Sep 2023)
- KWin Plasma 6.0+ via [MR !5496](https://invent.kde.org/plasma/kwin/-/merge_requests/5496) — exposes `org.kde.KWin.EIS.RemoteDesktop` D-Bus
- wlroots: out-of-tree, in progress

Rust bindings: [`reis`](https://docs.rs/reis/) crate. Python: [`snegg`](https://libinput.pages.freedesktop.org/snegg/).

**KWin's `org.kde.KWin.EIS.RemoteDesktop`** is the killer interface for KDE: D-Bus name `org.kde.KWin`, object `/EIS`, interface `org.kde.KWin.EIS.RemoteDesktop`. Calling it returns a libei-compatible **file descriptor**. Pass the fd to `reis::Context` (Rust) or `ei_setup_backend_fd()` (C). The entire `xdg-desktop-portal` dialog chain is **skipped** because you talk to KWin directly on the session bus. This is the **only D-Bus interface in any Linux compositor that provides input emulation without an interactive portal dialog.**

**XTEST under Xwayland** reaches only other X11 clients on the same Xwayland server. It does NOT reach native Wayland clients. This is the architectural reason xdotool stopped working on Wayland.

---

## Screen capture

| Tool | Compositor | Dialog | Cursor capture | Notes |
|---|---|---|---|---|
| `grim` (+ `slurp`) | wlroots only | No | Optional `-c` | Doesn't work on KWin or Mutter |
| `spectacle --background --nonotify -f` | KDE | No | Yes flag | Uses KWin ScreenShot2 internally |
| `gnome-screenshot` | GNOME | No on classic, yes flatpak | Yes | Portal under Wayland |
| `xdg-desktop-portal.ScreenShot` | All | First time, can be remembered | Configurable | Universal but interactive |
| `xdg-desktop-portal.ScreenCast` → PipeWire | All | Yes (RestoreToken skips) | Configurable | What OBS/Discord/browsers use; continuous stream |
| `org.kde.KWin.ScreenShot2` D-Bus | KDE only | No (authorized `.desktop` required) | Yes (option) | Fast ~30–70ms, raw ARGB on pipe |
| `wf-recorder` | wlroots | No | Yes | Video |

### KWin ScreenShot2 details

Introduced in [KWin MR !759](https://invent.kde.org/plasma/kwin/-/merge_requests/759); [MR !1594](https://invent.kde.org/plasma/kwin/-/merge_requests/1594) added `CaptureActiveScreen` in Plasma 6.0.

Methods (return raw ARGB on a Unix pipe fd with metadata):
- `CaptureWorkspace(options, pipe_fd)` — full virtual desktop, all monitors (Plasma 6.0+)
- `CaptureActiveScreen(options, pipe_fd)` — monitor with the pointer
- `CaptureScreen(name, options, pipe_fd)` — by output name
- `CaptureWindow(uuid, options, pipe_fd)`
- `CaptureArea(x, y, w, h, options, pipe_fd)`

`options` is `a{sv}`; relevant keys: `include-cursor: bool`, `include-decoration: bool`, `native-resolution: bool` (bypass logical-pixel scaling, returns physical pixels).

**Authorization:** KWin checks `KApplicationTrader` for a `.desktop` file containing `X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2`. Without it: `"process is not authorized to take a screenshot"`. Ship a `.desktop` file alongside the binary.

Latency: ~30–70ms at 1080p. ARGB → PNG via Pillow/`image` crate adds 5–20ms.

---

## Window queries

Wayland's design: **a client doesn't know its absolute screen position**. The compositor knows. All queries go through it.

| Operation | KDE | wlroots | GNOME |
|---|---|---|---|
| List windows | `org.kde.plasma.window_management` protocol; `workspace.windowList()` in scripts | `wlr-foreign-toplevel-management-unstable-v1` | GNOME Shell Eval D-Bus (locked) or gdbus calls |
| Active window | Scripting `workspace.activeWindow` | wlr-foreign-toplevel "activated" state | GNOME Shell only |
| Geometry | Scripting `window.frameGeometry` / `window.clientGeometry` | Not exposed | Shell extension required |
| Raise / activate | `xdg_activation_v1` token, or `workspace.activeWindow = w` in script | xdg_activation_v1 | xdg_activation_v1 |
| Move / resize | Scripting | Not exposed | Not exposed |

KDE explicitly closed [bug 502647](https://bugs.kde.org/show_bug.cgi?id=502647) refusing to implement `wlr-foreign-toplevel-management` as INTENTIONAL.

### KWin Scripting via D-Bus

Endpoint:
- Service `org.kde.KWin`, object `/Scripting`, interface `org.kde.kwin.Scripting`
- `loadScript(filename, pluginName) → scriptId` (filename, not source)
- `run()` / `stop()` on `/Scripting/Script{id}`
- `unloadScript(scriptName)`

Inside the script: globals `workspace` (active window, window list, current desktop, output info) and `options`.

```javascript
const w = workspace.activeWindow;
print(JSON.stringify({
  uuid: w.internalId,
  caption: w.caption,
  resourceClass: w.resourceClass,
  pid: w.pid,
  x: w.frameGeometry.x, y: w.frameGeometry.y,
  w: w.frameGeometry.width, h: w.frameGeometry.height,
  screen: w.output.name
}));
```

Coordinates: **logical pixels** (post fractional scaling). Multiply by `w.output.scale` for device pixels.

[`kdotool`](https://github.com/jinliu/kdotool) is the canonical reference for this pattern — writes temp JS, loads, runs, reads via signal or temp file, unloads. ~50–150ms per call.

---

## AT-SPI accessibility tree

AT-SPI2 is a D-Bus interface (separate `--bus accessibility` bus, exposed as `org.a11y.Bus`).

### Coverage by app family

- **Qt apps:** Generally good. Qt 5/6 implement AT-SPI directly. Best under KDE.
- **GTK 4:** Implements AT-SPI through built-in `GtkAccessible` (ATK removed). Reasonable.
- **GTK 3:** Through ATK → AT-SPI. Less complete.
- **Firefox:** Known regressions in late 2025 on Wayland AT-SPI; partially fixed by April 2026.
- **Chromium/Electron:** Needs `--force-renderer-accessibility` or specific flags. Spotty.
- **Java:** Generally not exposed.

### What you get

Per element: role (button, text-field, menu-item, ...), name, description, value, states (enabled, focused, visible, ...), absolute screen coordinates, action list with names ("press", "activate", "click"). You can invoke `Action.do_action(index)` — sometimes bypasses input emulation entirely.

Lag: 100–500ms behind UI state. Subscribe to events or poll-with-delay.

Rust bindings: [`atspi`](https://crates.io/crates/atspi) crate, pure Rust, async. Better than the Python `pyatspi` bindings in modernity.

### Bootstrap in private D-Bus session

Crucial gotcha for virtual mode: `at-spi-bus-launcher` needs explicit bootstrap inside `dbus-run-session`. The user's live session already has it via login session. See [kwin-mcp issue #14](https://github.com/isac322/kwin-mcp/issues/14) — fix is to parse `/usr/share/dbus-1/services/org.a11y.Bus.service` and `/usr/share/dbus-1/accessibility-services/org.a11y.atspi.Registry.service` to find launcher binaries (Fedora `/usr/libexec` vs Debian `/usr/lib`).

---

## Clipboard

**`wl-clipboard`** ([bugaevc/wl-clipboard](https://github.com/bugaevc/wl-clipboard)):
- `wl-copy < file`, `wl-paste > file`
- MIME negotiation: `wl-copy --type image/png < img.png`; `wl-paste --list-types`
- Uses `wlr-data-control-unstable-v1` for `--watch`. **Plasma 6.5 regression: `wl-paste --watch` is broken.** Plain `wl-copy`/`wl-paste` still work.

**Klipper D-Bus** (Plasma 6.5 watch-bug workaround):
- `qdbus org.kde.klipper /klipper getClipboardContents` → string
- `qdbus org.kde.klipper /klipper setClipboardContents "text"` → set
- Signal: `dbus-monitor "type='signal',interface='org.kde.klipper.klipper',member='clipboardHistoryUpdated'"`
- Requires Klipper widget enabled (default).
- Text-only. Binary/image only via wl-clipboard.

---

## Permission / sandboxing

| Approach | Needs | First-run friction |
|---|---|---|
| `/dev/uinput` direct / ydotool | `input` group + udev rule | One-time sudo + relogin |
| KWin private `org.kde.KWin.EIS` D-Bus | Session-bus access only | None |
| KWin `org.kde.KWin.ScreenShot2` | Authorized `.desktop` file | Ship the desktop file |
| `xdg-desktop-portal.RemoteDesktop` | None | Interactive dialog (RestoreToken removes it after) |
| `xdg-desktop-portal.ScreenCast` | None | Interactive dialog (RestoreToken removes it after) |
| AT-SPI | `accessibility` enabled (default on KDE/GNOME) | None |
| KWin Scripting D-Bus | None | None |

### systemd-user pitfall

A systemd user service does NOT inherit `DBUS_SESSION_BUS_ADDRESS`/`WAYLAND_DISPLAY` automatically. For an MCP server run as `.service`:

```ini
[Service]
PassEnvironment=DBUS_SESSION_BUS_ADDRESS WAYLAND_DISPLAY XDG_RUNTIME_DIR XDG_SESSION_TYPE DISPLAY
```

Or `systemctl --user import-environment WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS` at session startup.

### KDE 6.3+ portal pre-authorization

KDE introduced a `kde-authorized` permission table:

```
flatpak permission-set kde-authorized remote-desktop org.example.mcp-computer-use yes
flatpak permission-set kde-authorized screen-cast    org.example.mcp-computer-use yes
```

Skips all dialogs as long as `app_id` matches. Caveat: host (non-sandboxed) apps can impersonate app_ids; usability rather than security. GNOME has no equivalent.

Plasma 6.5 added a Settings GUI for per-app management.

---

## Wayland/KDE-specific reliability gotchas

- **Focus-stealing prevention** (Plasma 6 with "Extreme" setting): KWin only activates if there's a valid `xdg-activation` token. From inside a KWin Script, `workspace.activeWindow = w` bypasses the check. From outside, mint a token via `xdg_activation_v1.get_activation_token`.
- **uinput layout mismatch**: ydotool sends evdev keycodes. Compositor maps via current xkb. `ydotool key 30` ("a" on US) types "q" on AZERTY. Workaround: `wtype -k <unicode>` mints a fresh keymap.
- **Unicode/emoji input**: clipboard-paste is faster and more reliable than per-character typing. `wl-copy` + Ctrl+V beats `ydotool type "café"`.
- **Modifier sticking**: process crash between `keydown Ctrl` and `keyup Ctrl` leaves modifier held. uinput: virtual device disappearing on exit releases keys. libei: EIS connection close releases them.
- **Idle inhibit and screen lock**: continuous input prevents screensaver. KDE screen lock blocks all input emulation (including libei) — no bypass.
- **Multi-monitor**: Wayland has per-output coordinate spaces. KWin ScreenShot2 / Scripting absolute coords are virtual-desktop-space in logical pixels. Query layout with `kscreen-doctor -o` or `workspace.screens`.
- **Fractional scaling** (Plasma 6 default): logical-pixel coordinates from AT-SPI/Scripting/screenshots are post-scaled. Clicks at those coordinates work directly. With `native-resolution: true` you get physical pixels — must divide by output scale.
- **`org.kde.KWin` vs `org.kde.kwin` naming inconsistency**: capitalisation matters. New screen capture and EIS use `KWin.*` (capitalised). Older Screenshot v1 and Scripting use `kwin.*`. Triple-check bindings.

---

## Existing "computer-use for Linux" projects (ranked by relevance)

| Project | Approach | Verdict |
|---|---|---|
| [isac322/kwin-mcp](https://github.com/isac322/kwin-mcp) | KWin EIS + AT-SPI + ScreenShot2 + virtual KWin sessions; Python; 30 tools | **Closest match.** Read this codebase first. |
| [kurojs/wayland-mcp](https://github.com/kurojs/wayland-mcp) | `evemu-event` + `grim/slurp` + VLM; multi-compositor | Coordinate-only, no AT-SPI |
| [someaka/wayland-mcp](https://github.com/someaka/wayland-mcp) | `evemu-event` + OpenRouter VLM | Archived April 2026 |
| [AB498/computer-control-mcp](https://github.com/AB498/computer-control-mcp) | PyAutoGUI + RapidOCR | X11-only (fails on native Wayland clients) |
| [spencerkinney/computer-use-mcp](https://github.com/spencerkinney/computer-use-mcp) | Wrapper around Anthropic's tool schema | mac-leaning |
| [charettep/ubuntu-desktop-control-mcp](https://github.com/charettep/ubuntu-desktop-control-mcp) | xdotool / wmctrl | X11 only |
| [anthropics/anthropic-quickstarts/computer-use-demo](https://github.com/anthropics/anthropic-quickstarts) | Docker Xfce + Xvfb + xdotool + scrot | Reference for tool schema. Bypasses Wayland via X11 in container. |
| [nut.js / RobotJS](https://nutjs.dev/) | X11 / XTEST | No Wayland. |
| Open Interpreter OS mode | screen capture + LLM coords + PyAutoGUI | X11 on Linux; broken on Wayland |
| [microsoft/OmniParser](https://github.com/microsoft/OmniParser) | UI element vision model | Pure vision, no input |
