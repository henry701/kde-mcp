# Installation guide

This document covers manual install from source. Distro packages may follow later.

## Prerequisites

KDE Plasma 6 on Wayland. The binary refuses to run outside this. Check:

```sh
echo $XDG_CURRENT_DESKTOP   # should contain KDE
echo $XDG_SESSION_TYPE      # should be wayland
```

## System dependencies

### Arch (and derivatives)

```sh
sudo pacman -S libei at-spi2-core wl-clipboard wayvnc \
               tesseract tesseract-data-eng \
               spectacle xdg-desktop-portal-kde
```

### Fedora

```sh
sudo dnf install libei at-spi2-core wl-clipboard wayvnc \
                 tesseract tesseract-langpack-eng \
                 spectacle xdg-desktop-portal-kde
```

### Debian / Ubuntu

```sh
sudo apt install libei-bin at-spi2-core wl-clipboard wayvnc \
                 tesseract-ocr tesseract-ocr-eng \
                 kde-spectacle xdg-desktop-portal-kde
```

### Rust toolchain

Install via [rustup](https://rustup.rs) if you don't have it:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Minimum Rust version: 1.78. Newer is fine.

## Build and install the binary

From the repo root:

```sh
cargo install --path . --root ~/.local
```

This puts `kde-mcp` at `~/.local/bin/kde-mcp`. Make sure `~/.local/bin` is on your `PATH`.

## Authorize the ScreenShot2 D-Bus interface

KWin requires the binary to have an installed `.desktop` file declaring its right to call the screenshot interface. From the repo root:

```sh
install -m 644 packaging/kde-mcp.desktop ~/.local/share/applications/
```

This is a one-time setup. Until this is done, `screenshot` tool calls will fail with "process is not authorized to take a screenshot."

## Register with Claude Code

```sh
claude mcp add kde-live    ~/.local/bin/kde-mcp -- --mode=live
claude mcp add kde-virtual ~/.local/bin/kde-mcp -- --mode=virtual --vnc=5910
```

For Claude Desktop or other MCP hosts, add to your config (e.g. `~/.config/Claude/claude_desktop_config.json`):

```jsonc
{
  "mcpServers": {
    "kde-live": {
      "command": "/home/USER/.local/bin/kde-mcp",
      "args": ["--mode=live"]
    },
    "kde-virtual": {
      "command": "/home/USER/.local/bin/kde-mcp",
      "args": ["--mode=virtual", "--vnc=5910"]
    }
  }
}
```

## Smoke test

Open Claude Code in any directory and ask:

> "Use the kde-live MCP server to call `observe` and tell me what window is focused."

If the server is correctly installed and authorized, you'll get a structured snapshot of your desktop.

## Watching the virtual session

The `kde-virtual` server with `--vnc=5910` exposes the headless KWin session over VNC on `localhost:5910`. Open it with KRDC:

```sh
krdc vnc://localhost:5910
```

Or any other VNC client. The connection is bound to localhost only — not exposed to your network.

## Troubleshooting

- **"process is not authorized to take a screenshot"** — the `.desktop` file isn't installed or KWin hasn't refreshed its application cache. Run `kbuildsycoca6` after installing the desktop file.
- **`libei` error on startup** — your distro's `libei` may be older than 1.3. Check `pkg-config --modversion libei` or `ldconfig -p | grep libei`.
- **AT-SPI returns empty trees** — confirm `at-spi2-core` is installed and that the bus is running: `pgrep at-spi-bus-launcher`. Some apps (notably Chromium/Electron) need explicit flags.
- **Virtual mode bootstrap times out** — likely `kwin_wayland --virtual` is missing or the user's `dbus-run-session` isn't available. Both ship with KDE/dbus by default.

## Uninstall

```sh
rm ~/.local/bin/kde-mcp ~/.local/share/applications/kde-mcp.desktop
claude mcp remove kde-live
claude mcp remove kde-virtual
```
