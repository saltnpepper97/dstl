<h1 align="center">dstl</h1>

<p align="center"><b>Dustin's Simple TUI Launcher</b> - A fast, keyboard-driven application launcher for the terminal with fuzzy search and extensive theming support.</p>

## Features

- 🚀 **Fast fuzzy search** - Quickly find applications as you type
- 🎨 **Highly customizable** - Extensive theming with hex color support
- 📱 **Dual view modes** - Switch between single-pane and dual-pane (category + apps) layouts
- ⌨️ **Vim-style navigation** - hjkl movement plus arrow keys
- 📋 **Recent apps tracking** - Quick access to frequently used applications
- 🎯 **Smart cursor** - Full cursor control with blinking support
- 🔧 **Flexible configuration** - Uses `.rune` config format with import/gather support

## Installation

### AUR

```sh
paru -s dstl
```

or

```sh
yay -s dstl
```

### From Source

```bash
git clone https://github.com/saltnpepper97/dstl
cd dstl
cargo build --release
sudo cp target/release/dstl /usr/local/bin/
```

## Configuration

dstl looks for configuration in the following locations (in order):
1. `~/.config/dstl/dstl.rune`
2. `/usr/share/doc/dstl/dstl.rune`

### Basic Configuration Example

```rune
dstl:
    # Display mode
    dmenu = false
    startup_mode = "dual"  # or "single"
    search_position = "top"  # or "bottom"
    focus_search_on_switch = true
    
    # Terminal emulator for terminal apps
    terminal = "foot"
    
    # Recent apps settings
    max_recent_apps = 15
    recent_first = false
    
    # Theme configuration
    theme:
        border = "#ffffff"
        focus = "#00ff00"
        highlight = "#0000ff"
        cursor_color = "#00ff00"  # defaults to focus color
        cursor_shape = "block"  # "block", "underline", or "pipe"
        cursor_blink_interval = 500  # milliseconds, 0 to disable
        border_style = "rounded"  # "plain", "rounded", "thick", "double"
        highlight_type = "background"  # or "foreground"
    end
end
```

### Theme System with Gather

dstl supports importing themes using the `gather` statement:

```rune
# Import a theme file
gather "~/.config/dstl/themes/dracula.rune" as theme

dstl:
    terminal = "alacritty"
    # Theme colors will be loaded from the gathered file
    theme:
      cursor_shape = "block"
      cursor_blink_interval = 500
      border_style = "plain"
      highlight_type = "background"
    end
end
```

**Theme Priority:**
1. Aliased gather imports (e.g., `gather "theme.rune" as mytheme`)
2. Top-level theme in main config or non-aliased gather
3. Document named "theme"
4. Built-in defaults

### Color Format

Colors support multiple hex formats:
- `#RGB` - 3-digit hex (e.g., `#fff`)
- `#RRGGBB` - 6-digit hex (e.g., `#ffffff`)

## Usage

### Launching

```bash
# Launch directly
dstl
```

# Launch from config (hyprland example)
```
bind = $mainMod, R, exec, kitty --class dstl -e dstl
```

### Keyboard Shortcuts

#### Navigation
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `h` / `←` - Move left (categories in dual-pane, or prev app in single-pane)
- `l` / `→` - Move right (apps in dual-pane, or next app in single-pane)
- `Tab` - Cycle focus (Search → Categories → Apps → Search)

#### Search
- `Type` - Search for applications (fuzzy matching)
- `Backspace` - Delete character before cursor
- `Delete` - Delete character at cursor
- `←` / `→` - Move cursor within search query
- `Home` - Jump to start of search query
- `End` - Jump to end of search query

#### Actions
- `Enter` - Launch selected application
- `m` - Toggle between single-pane and dual-pane mode
- `q` - Quit (when not in search bar)
- `Esc` - Quit

## View Modes

### Single-Pane Mode
Shows all applications in one list with fuzzy search filtering across all categories.

### Dual-Pane Mode
- **Left pane**: Categories with app count
- **Right pane**: Applications in selected category
- Search filters both panes simultaneously
- Special "Recent" category shows recently launched apps

## Advanced Configuration

### Key Settings Explained

- **`dmenu`**: Enable dmenu-like behavior (boolean)
- **`search_position`**: Place search bar at `"top"` or `"bottom"`
- **`startup_mode`**: Start in `"single"` or `"dual"` pane mode
- **`focus_search_on_switch`**: Auto-focus search when switching modes
- **`timeout`**: Auto-close timeout in milliseconds (0 to disable)
- **`max_recent_apps`**: Maximum number of recent apps to track
- **`recent_first`**: Show recent apps category first

### Cursor Customization

- **`cursor_shape`**: Visual style of the cursor
  - `"block"` - Solid block (█)
  - `"underline"` - Underscore (_)
  - `"pipe"` - Vertical bar (|)
- **`cursor_blink_interval`**: Blink speed in milliseconds (0 = no blinking)
- **`cursor_color`**: Hex color for cursor (defaults to focus color)

### Border Styles

- `"plain"` - Simple lines
- `"rounded"` - Rounded corners
- `"thick"` - Bold lines
- `"double"` - Double-line borders

### Highlight Types

- `"background"` - Highlight with background color (selected text is black)
- `"foreground"` - Highlight with foreground color only

## Desktop Entry Detection

dstl automatically scans for `.desktop` files in standard XDG directories to populate the application list. Categories are extracted from desktop entries.

## Tips

- Use fuzzy search to quickly find apps by typing partial names
- The search algorithm scores matches, showing best matches first
- Recent apps are persistent across sessions
- Cursor stays visible and solid while typing or moving
- Navigate between search and lists seamlessly with arrow keys

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
