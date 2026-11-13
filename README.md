# Command Code GUI

A desktop client for [Command Code](https://commandcode.ai), built in Rust with
[egui](https://github.com/emilk/egui) and the
[Agent Client Protocol](https://agentclientprotocol.com/) (ACP).

The app spawns `commandcode acp` and speaks ACP over stdio, so it reuses the
existing CLI login and the same sessions as the terminal client.

## Features

- Streaming chat with markdown and syntax highlighting
- Tool call cards, execution status, raw input/output, and inline diffs
- Permission dialogs backed by `session/request_permission`
- Session history from `~/.commandcode/projects`, grouped by project
- Model, reasoning effort, and mode switching from the session bar
- Context usage indicator
- Command palette for ACP-advertised slash commands
- Dark and light themes, configurable font scale
- Responsive layout:
  - sidebar collapses below 620 px
  - tool details become an overlay below 1080 px
  - minimum window size is 420×360, so tiling WMs can split 50/50 evenly

## Requirements

- Command Code CLI in `PATH` and logged in (`commandcode login`)
- Rust 1.95+ for source builds
- NixOS/Nix is supported directly through the included flake

## Run

With Nix:

```bash
nix run
```

For development:

```bash
nix develop -c cargo run
```

Or, with the graphics libraries and a Rust toolchain available:

```bash
cargo run
```

The default agent command is:

```text
commandcode --no-auto-update acp
```

`--no-auto-update` avoids writes to the Nix store. Change the binary or
arguments in Settings, then use “Reconnect agent”.

## Keyboard

| Key | Action |
| --- | --- |
| `Enter` | Send |
| `Shift+Enter` | New line |
| `Ctrl+N` | New session |
| `Ctrl+K` | Command palette |
| `Ctrl+,` | Settings |
| `Esc` | Stop the current turn / close a dialog |

The composer is focused automatically when a session becomes ready.

## Configuration

Settings are stored at:

```text
~/.config/commandcode-gui/settings.json
```

Notable options:

- `agent_binary` / `agent_args`
- `default_cwd`
- `theme`: `system`, `dark`, or `light`
- `font_scale`
- `show_thoughts`
- `permission_policy`: `ask`, `auto_allow_reads`, or `yolo`

The CJK font is loaded from `COMMANDCODE_GUI_CJK_FONT` when set, otherwise via
`fc-match "Noto Sans CJK JP"`. The Nix package and devShell set this
automatically.

## Development

```bash
nix develop -c cargo fmt --check
nix develop -c cargo clippy --all-targets -- -D warnings
nix develop -c cargo test
```

End-to-end ACP smoke test (creates a real session and asks for a reply):

```bash
nix develop -c cargo run --example acp_smoke
```
