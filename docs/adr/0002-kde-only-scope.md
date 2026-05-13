# ADR-0002: Scope — KDE Plasma 6 / Wayland only

**Status:** Accepted
**Date:** 2026-05-13
**Deciders:** atassis

## Context

The original Anthropic Computer Use tool runs in an X11/Xvfb Docker container, deliberately avoiding native Wayland complexity. To target Linux Wayland, we have to pick a compositor stack to specialise in:

- **KDE Plasma 6 (KWin)** — exposes private D-Bus paths (`org.kde.KWin.EIS.RemoteDesktop`, `org.kde.KWin.ScreenShot2`, `org.kde.KWin.Scripting`) that allow no-portal-dialog access from a session-bus-connected client.
- **GNOME (Mutter)** — only via xdg-desktop-portal (RemoteDesktop / ScreenCast), which shows a consent dialog on first use; `restore_token` skips it later but the flow is markedly different.
- **wlroots-based** (sway, Hyprland, Wayfire) — virtual-pointer/virtual-keyboard protocols, no portal needed but no AT-SPI uniformity either.
- **X11** — xdotool/scrot work, but it's a legacy stack with degraded a11y.

The author (and primary user) is on KDE Plasma 6 / Wayland.

## Decision

MVP targets **KDE Plasma 6 on Wayland only**. The binary explicitly checks `XDG_CURRENT_DESKTOP` and `XDG_SESSION_TYPE` at startup; on any other configuration it exits with a clear error.

## Rationale

1. **Personal-use first.** This is the author's daily driver. There is no current need to support other compositors.
2. **KDE has the cleanest no-dialog path.** The `org.kde.KWin.EIS.RemoteDesktop` bypass is unique to KDE. GNOME/wlroots add either portal dialogs (annoying for an agent that needs to act unattended) or different code paths (large surface area for a marginal user gain).
3. **AT-SPI uniformity.** While AT-SPI works on every Linux desktop, KDE's apps (Qt-based) have the most consistent AT-SPI coverage.
4. **Avoid premature multi-compositor abstraction.** Building two backends costs ~2× the integration testing while paying for hypothetical users we don't have.
5. **kwin-mcp's lesson.** The closest prior art is also KDE-only, for these reasons.

## Consequences

- Cannot run on GNOME / Hyprland / Sway / wlroots. Not even with degraded functionality — the binary refuses to start.
- Cannot run on X11 (including X11-on-KDE if the user chose that login).
- The code never grows a "compositor abstraction layer" — backends are direct calls. This simplifies refactoring later if we ever add a second compositor.
- Distribution can target KDE-only assumptions (e.g., the `.desktop` file authorization is KDE-specific).
- Community adoption is narrower; that's acceptable for personal-use phase.

## Alternatives considered

- **Multi-compositor MVP.** Rejected: 2× testing surface, 2× failure modes, and the user only runs one compositor.
- **Wayland-portable via libei + xdg-desktop-portal RemoteDesktop only.** Works everywhere but every user pays a one-time consent dialog. Rejected — the no-dialog KDE path is worth too much.
- **KDE + xdg-desktop-portal fallback for other compositors.** Adds dialog flow + restore_token plumbing + GNOME-specific app_id handling. Rejected for MVP; explicitly listed as future work in §15 of the spec.

## When to revisit

- When the tool proves valuable enough that community use becomes a goal.
- When a second machine (work laptop, home server) running a different compositor becomes part of the use case.
- When `xdg-desktop-portal` RemoteDesktop with `restore_token` becomes stable enough across compositors to be a transparent fallback.
