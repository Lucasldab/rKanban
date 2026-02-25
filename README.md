# rkanban

A fast, keyboard-driven Kanban board for the terminal. Built with [Ratatui](https://github.com/ratatui-org/ratatui) and Rust.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- Multiple columns with cards, descriptions, and tags
- Full keyboard navigation — no mouse required
- Cursor movement and multiline descriptions in edit popups
- Reorder cards and columns in place
- Comma-separated tags displayed inline
- Auto-saves to JSON on every change
- Accepts a custom board file as a CLI argument
- Fully configurable keybindings and color scheme via INI config file

## Installation

```bash
git clone https://github.com/youruser/rkanban
cd rkanban
cargo build --release
```

The binary will be at `target/release/rkanban`. You can move it anywhere on your `$PATH`:

```bash
cp target/release/rkanban ~/.local/bin/
```

## Usage

```bash
# Open the default board (board.json in the current directory)
rkanban

# Open a specific board file
rkanban ~/boards/work.json
rkanban ~/boards/personal.json
```

Board files are created automatically if they don't exist.

## Keybindings

All bindings are remappable — see [Configuration](#configuration).

### Navigation

| Key | Action |
|-----|--------|
| `←` / `→` | Move between columns |
| `↑` / `↓` | Move between cards |

### Card Actions

| Key | Action |
|-----|--------|
| `a` | Add new card |
| `e` | Edit selected card |
| `v` / `Enter` | View card details |
| `d` | Delete selected card |
| `h` / `l` | Move card to previous / next column |
| `J` / `K` | Reorder card down / up within column |

### Column Management

| Key | Action |
|-----|--------|
| `n` | Add new column |
| `r` | Rename selected column |
| `x` | Delete selected column (requires confirmation) |
| `H` / `L` | Reorder column left / right |

### General

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `q` | Quit (auto-saves) |

### Popup / Edit Mode

| Key | Action |
|-----|--------|
| `Tab` | Cycle fields: Title → Description → Tags |
| `←` / `→` | Move cursor within field |
| `Enter` | Insert newline (in description) / confirm card |
| `Esc` | Cancel and close popup |

## Card Fields

Each card has three fields editable from the same popup:

- **Title** — required, single line
- **Description** — optional, multiline (press `Enter` to insert newlines)
- **Tags** — optional, comma-separated (e.g. `bug, urgent, backend`)

Tags are displayed inline to the right of the card title in the board view and at the bottom of the card detail view.

## Configuration

On first run, rkanban creates a config file at:

```
~/.config/rkanban/config.conf
```

### Keybindings

Keys can be single characters (`a`–`z`, `A`–`Z`, `0`–`9`) or special names: `up`, `down`, `left`, `right`, `enter`, `esc`, `tab`, `backspace`.

```ini
[keys]
# Navigation
nav_left        = left
nav_right       = right
nav_up          = up
nav_down        = down

# Card actions
add_card        = a
edit_card       = e
view_card       = v
delete_card     = d
move_card_left  = h
move_card_right = l
reorder_up      = K
reorder_down    = J

# Column management
add_column        = n
rename_column     = r
delete_column     = x
reorder_col_left  = H
reorder_col_right = L

# General
quit = q
help = ?

# Popup keys
popup_next_field = tab
popup_confirm    = enter
popup_cancel     = esc
```

### Colors

Colors can be named or specified as hex RGB:

```ini
[colors]
accent      = cyan
selected_bg = cyan
selected_fg = black
subtle      = darkgray
warning     = yellow
tag         = magenta
text        = white
background  = black
```

**Named colors:** `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `gray`, `darkgray`, `lightred`, `lightgreen`, `lightyellow`, `lightblue`, `lightmagenta`, `lightcyan`

**Hex RGB:** `#rrggbb` — e.g. `accent = #89b4fa` for a Catppuccin blue

### Example: Catppuccin Mocha theme

```ini
[colors]
accent      = #89b4fa
selected_bg = #89b4fa
selected_fg = #1e1e2e
subtle      = #585b70
warning     = #f9e2af
tag         = #cba6f7
text        = #cdd6f4
background  = #1e1e2e
```

## Board File Format

Boards are saved as pretty-printed JSON. The format is stable and human-readable, so you can edit files directly or generate them from scripts.

```json
{
  "columns": [
    {
      "name": "Todo",
      "selected": 0,
      "cards": [
        {
          "title": "My task",
          "description": "Some notes here",
          "tags": ["urgent", "backend"]
        }
      ]
    }
  ],
  "selected_column": 0
}
```

## Project Structure

```
src/
├── main.rs        # Entry point, event loop, input handlers
├── config.rs      # INI config loader, KeyMap, ColorScheme
├── app/
│   └── mod.rs     # App state, Card/Column types, board logic
└── ui/
    └── mod.rs     # Ratatui rendering
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [ratatui](https://crates.io/crates/ratatui) | TUI framework |
| [crossterm](https://crates.io/crates/crossterm) | Terminal backend |
| [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) | Board file serialization |

No external config parsing library — the INI parser is built in.

## License

MIT
