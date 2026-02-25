use ratatui::style::Color;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ── Default config text (written on first run) ────────────────────────────────
pub const DEFAULT_CONFIG: &str = r#"
# rkanban configuration file
# Keys can be single characters (a-z, A-Z, 0-9) or special names:
#   up, down, left, right, enter, esc, tab, backspace
# Colors can be named: black, red, green, yellow, blue, magenta, cyan, white,
#   darkgray, lightred, lightgreen, lightyellow, lightblue, lightmagenta, lightcyan, gray
# Or RGB hex: #rrggbb

[keys]
# Navigation
nav_left       = left
nav_right      = right
nav_up         = up
nav_down       = down

# Card actions
add_card       = a
edit_card      = e
view_card      = v
delete_card    = d
move_card_left = h
move_card_right= l
reorder_up     = K
reorder_down   = J

# Column management
add_column     = n
rename_column  = r
delete_column  = x
reorder_col_left  = H
reorder_col_right = L

# General
quit           = q
help           = ?

# Popup keys
popup_next_field = tab
popup_confirm    = enter
popup_cancel     = esc

[colors]
accent         = cyan
selected_bg    = cyan
selected_fg    = black
subtle         = darkgray
warning        = yellow
tag            = magenta
text           = white
background     = black
"#;

// ── Key representation ────────────────────────────────────────────────────────
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    Backspace,
}

impl Key {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "up"        => Some(Key::Up),
            "down"      => Some(Key::Down),
            "left"      => Some(Key::Left),
            "right"     => Some(Key::Right),
            "enter"     => Some(Key::Enter),
            "esc"       => Some(Key::Esc),
            "tab"       => Some(Key::Tab),
            "backspace" => Some(Key::Backspace),
            s if s.len() == 1 => s.chars().next().map(Key::Char),
            _ => None,
        }
    }

    pub fn matches(&self, code: &crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode as KC;
        match (self, code) {
            (Key::Char(a), KC::Char(b)) => a == b,
            (Key::Up,        KC::Up)        => true,
            (Key::Down,      KC::Down)      => true,
            (Key::Left,      KC::Left)      => true,
            (Key::Right,     KC::Right)     => true,
            (Key::Enter,     KC::Enter)     => true,
            (Key::Esc,       KC::Esc)       => true,
            (Key::Tab,       KC::Tab)       => true,
            (Key::Backspace, KC::Backspace) => true,
            _ => false,
        }
    }
}

// ── KeyMap ────────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct KeyMap {
    pub nav_left:          Key,
    pub nav_right:         Key,
    pub nav_up:            Key,
    pub nav_down:          Key,
    pub add_card:          Key,
    pub edit_card:         Key,
    pub view_card:         Key,
    pub delete_card:       Key,
    pub move_card_left:    Key,
    pub move_card_right:   Key,
    pub reorder_up:        Key,
    pub reorder_down:      Key,
    pub add_column:        Key,
    pub rename_column:     Key,
    pub delete_column:     Key,
    pub reorder_col_left:  Key,
    pub reorder_col_right: Key,
    pub quit:              Key,
    pub help:              Key,
    pub popup_next_field:  Key,
    pub popup_confirm:     Key,
    pub popup_cancel:      Key,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            nav_left:          Key::Left,
            nav_right:         Key::Right,
            nav_up:            Key::Up,
            nav_down:          Key::Down,
            add_card:          Key::Char('a'),
            edit_card:         Key::Char('e'),
            view_card:         Key::Char('v'),
            delete_card:       Key::Char('d'),
            move_card_left:    Key::Char('h'),
            move_card_right:   Key::Char('l'),
            reorder_up:        Key::Char('K'),
            reorder_down:      Key::Char('J'),
            add_column:        Key::Char('n'),
            rename_column:     Key::Char('r'),
            delete_column:     Key::Char('x'),
            reorder_col_left:  Key::Char('H'),
            reorder_col_right: Key::Char('L'),
            quit:              Key::Char('q'),
            help:              Key::Char('?'),
            popup_next_field:  Key::Tab,
            popup_confirm:     Key::Enter,
            popup_cancel:      Key::Esc,
        }
    }
}

// ── ColorScheme ───────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub struct ColorScheme {
    pub accent:      Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub subtle:      Color,
    pub warning:     Color,
    pub tag:         Color,
    pub text:        Color,
    pub background:  Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            accent:      Color::Cyan,
            selected_bg: Color::Cyan,
            selected_fg: Color::Black,
            subtle:      Color::DarkGray,
            warning:     Color::Yellow,
            tag:         Color::Magenta,
            text:        Color::White,
            background:  Color::Black,
        }
    }
}

// ── Full config ───────────────────────────────────────────────────────────────
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub keys:   KeyMap,
    pub colors: ColorScheme,
}

// ── Config file path ──────────────────────────────────────────────────────────
pub fn config_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(base)
        .join(".config")
        .join("rkanban")
        .join("config.conf")
}

// ── Load / parse ──────────────────────────────────────────────────────────────
pub fn load() -> Config {
    let path = config_path();

    // Write default config if it doesn't exist
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, DEFAULT_CONFIG.trim_start());
    }

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Config::default(),
    };

    let ini = parse_ini(&text);
    let mut config = Config::default();

    // ── keys section ──────────────────────────────────────────────────────────
    if let Some(keys) = ini.get("keys") {
        macro_rules! load_key {
            ($field:expr, $name:expr) => {
                if let Some(val) = keys.get($name) {
                    if let Some(k) = Key::from_str(val) {
                        $field = k;
                    }
                }
            };
        }
        load_key!(config.keys.nav_left,          "nav_left");
        load_key!(config.keys.nav_right,         "nav_right");
        load_key!(config.keys.nav_up,            "nav_up");
        load_key!(config.keys.nav_down,          "nav_down");
        load_key!(config.keys.add_card,          "add_card");
        load_key!(config.keys.edit_card,         "edit_card");
        load_key!(config.keys.view_card,         "view_card");
        load_key!(config.keys.delete_card,       "delete_card");
        load_key!(config.keys.move_card_left,    "move_card_left");
        load_key!(config.keys.move_card_right,   "move_card_right");
        load_key!(config.keys.reorder_up,        "reorder_up");
        load_key!(config.keys.reorder_down,      "reorder_down");
        load_key!(config.keys.add_column,        "add_column");
        load_key!(config.keys.rename_column,     "rename_column");
        load_key!(config.keys.delete_column,     "delete_column");
        load_key!(config.keys.reorder_col_left,  "reorder_col_left");
        load_key!(config.keys.reorder_col_right, "reorder_col_right");
        load_key!(config.keys.quit,              "quit");
        load_key!(config.keys.help,              "help");
        load_key!(config.keys.popup_next_field,  "popup_next_field");
        load_key!(config.keys.popup_confirm,     "popup_confirm");
        load_key!(config.keys.popup_cancel,      "popup_cancel");
    }

    // ── colors section ────────────────────────────────────────────────────────
    if let Some(colors) = ini.get("colors") {
        macro_rules! load_color {
            ($field:expr, $name:expr) => {
                if let Some(val) = colors.get($name) {
                    if let Some(c) = parse_color(val) {
                        $field = c;
                    }
                }
            };
        }
        load_color!(config.colors.accent,      "accent");
        load_color!(config.colors.selected_bg, "selected_bg");
        load_color!(config.colors.selected_fg, "selected_fg");
        load_color!(config.colors.subtle,      "subtle");
        load_color!(config.colors.warning,     "warning");
        load_color!(config.colors.tag,         "tag");
        load_color!(config.colors.text,        "text");
        load_color!(config.colors.background,  "background");
    }

    config
}

// ── INI parser ────────────────────────────────────────────────────────────────
fn parse_ini(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut section = String::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_lowercase();
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let val = val.trim().to_string();
            // Strip inline comments
            let val = val.split('#').next().unwrap_or("").trim().to_string();
            map.entry(section.clone()).or_default().insert(key, val);
        }
    }
    map
}

// ── Color parser ──────────────────────────────────────────────────────────────
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    if s.starts_with('#') && s.len() == 7 {
        let r = u8::from_str_radix(&s[1..3], 16).ok()?;
        let g = u8::from_str_radix(&s[3..5], 16).ok()?;
        let b = u8::from_str_radix(&s[5..7], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    match s.as_str() {
        "black"        => Some(Color::Black),
        "red"          => Some(Color::Red),
        "green"        => Some(Color::Green),
        "yellow"       => Some(Color::Yellow),
        "blue"         => Some(Color::Blue),
        "magenta"      => Some(Color::Magenta),
        "cyan"         => Some(Color::Cyan),
        "white"        => Some(Color::White),
        "darkgray"     => Some(Color::DarkGray),
        "lightred"     => Some(Color::LightRed),
        "lightgreen"   => Some(Color::LightGreen),
        "lightyellow"  => Some(Color::LightYellow),
        "lightblue"    => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan"    => Some(Color::LightCyan),
        "gray"         => Some(Color::Gray),
        _              => None,
    }
}
