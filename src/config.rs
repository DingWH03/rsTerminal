use serde::{Deserialize, Serialize};

/// Virtual keyboard layout mode (persisted on terminal profiles).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum KeyboardMode {
    /// Function / arrow keys only.
    Special,
    /// Full alphanumeric keyboard.
    #[default]
    Full,
}

/// Bash `PROMPT_COMMAND` that emits OSC 7 (`file://host/path`) for cwd tracking over SSH.
///
/// Sent via SSH `set_env` (no PTY echo). Requires the server to accept the variable
/// (`AcceptEnv PROMPT_COMMAND` / `SetEnv`, etc.); otherwise it is ignored.
pub const SSH_OSC7_PROMPT_COMMAND: &str = r#"printf "\033]7;file://%s%s\033\\" "$HOSTNAME" "$PWD""#;

/// Terminal cursor appearance (configurable in settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CursorStyle {
    /// Thin vertical bar at the left of the cell.
    #[default]
    Bar,
    /// Inverted full cell (classic block cursor).
    Block,
    /// Horizontal line at the bottom of the cell.
    Underline,
    /// Blinking vertical bar.
    BarBlink,
    /// Blinking block cursor.
    BlockBlink,
    /// Blinking underline.
    UnderlineBlink,
}

impl CursorStyle {
    pub const ALL: [Self; 6] = [
        Self::Bar,
        Self::Block,
        Self::Underline,
        Self::BarBlink,
        Self::BlockBlink,
        Self::UnderlineBlink,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Bar => rust_i18n::t!("cursor_bar").into_owned(),
            Self::Block => rust_i18n::t!("cursor_block").into_owned(),
            Self::Underline => rust_i18n::t!("cursor_underline").into_owned(),
            Self::BarBlink => rust_i18n::t!("cursor_bar_blink").into_owned(),
            Self::BlockBlink => rust_i18n::t!("cursor_block_blink").into_owned(),
            Self::UnderlineBlink => rust_i18n::t!("cursor_underline_blink").into_owned(),
        }
    }
}

/// Terminal bell / alert behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum BellStyle {
    /// No bell.
    Off,
    /// Visual flash only.
    #[default]
    Visual,
    /// System beep only.
    Audible,
    /// Both flash and beep.
    Both,
}

impl BellStyle {
    pub const ALL: [Self; 4] = [Self::Off, Self::Visual, Self::Audible, Self::Both];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Visual => "Visual",
            Self::Audible => "Audible",
            Self::Both => "Visual + Audible",
        }
    }
}

/// Terminal type reported via $TERM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TerminalType {
    #[default]
    Xterm256,
    Xterm,
    Screen256,
    Screen,
    Tmux256,
    Tmux,
}

impl TerminalType {
    pub const ALL: [Self; 6] = [
        Self::Xterm256,
        Self::Xterm,
        Self::Screen256,
        Self::Screen,
        Self::Tmux256,
        Self::Tmux,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Xterm256 => "xterm-256color",
            Self::Xterm => "xterm",
            Self::Screen256 => "screen-256color",
            Self::Screen => "screen",
            Self::Tmux256 => "tmux-256color",
            Self::Tmux => "tmux",
        }
    }
}

/// Neutral RGBA color for themes (serde-compatible with egui `Color32` `[u8;4]` arrays).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Serialize for Rgba {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_array().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let arr = <[u8; 4]>::deserialize(deserializer)?;
        Ok(Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub bg: Rgba,
    pub fg: Rgba,
    pub cursor: Rgba,
    pub selection: Rgba,
    /// Scrollback scrollbar thumb.
    #[serde(default = "default_theme_scrollbar_thumb")]
    pub scrollbar_thumb: Rgba,
    /// Scrollback scrollbar thumb while hovered or dragged.
    #[serde(default = "default_theme_scrollbar_thumb_hover")]
    pub scrollbar_thumb_hover: Rgba,
    pub black: Rgba,
    pub red: Rgba,
    pub green: Rgba,
    pub yellow: Rgba,
    pub blue: Rgba,
    pub magenta: Rgba,
    pub cyan: Rgba,
    pub white: Rgba,
    pub bright_black: Rgba,
    pub bright_red: Rgba,
    pub bright_green: Rgba,
    pub bright_yellow: Rgba,
    pub bright_blue: Rgba,
    pub bright_magenta: Rgba,
    pub bright_cyan: Rgba,
    pub bright_white: Rgba,
}

fn default_theme_scrollbar_thumb() -> Rgba {
    Rgba::from_rgba_premultiplied(180, 180, 180, 190)
}

fn default_theme_scrollbar_thumb_hover() -> Rgba {
    Rgba::from_rgb(59, 142, 234)
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            bg: Rgba::from_rgb(30, 30, 30),
            fg: Rgba::from_rgb(220, 220, 220),
            cursor: Rgba::from_rgb(255, 255, 255),
            selection: Rgba::from_rgba_premultiplied(100, 100, 255, 128),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(180, 180, 180, 190),
            scrollbar_thumb_hover: Rgba::from_rgb(59, 142, 234),
            black: Rgba::from_rgb(0, 0, 0),
            red: Rgba::from_rgb(205, 49, 49),
            green: Rgba::from_rgb(13, 188, 121),
            yellow: Rgba::from_rgb(229, 229, 16),
            blue: Rgba::from_rgb(36, 114, 200),
            magenta: Rgba::from_rgb(188, 63, 188),
            cyan: Rgba::from_rgb(17, 168, 205),
            white: Rgba::from_rgb(220, 220, 220),
            bright_black: Rgba::from_rgb(102, 102, 102),
            bright_red: Rgba::from_rgb(241, 76, 76),
            bright_green: Rgba::from_rgb(35, 209, 139),
            bright_yellow: Rgba::from_rgb(245, 245, 67),
            bright_blue: Rgba::from_rgb(59, 142, 234),
            bright_magenta: Rgba::from_rgb(214, 112, 214),
            bright_cyan: Rgba::from_rgb(41, 184, 219),
            bright_white: Rgba::from_rgb(255, 255, 255),
        }
    }
}

impl TerminalTheme {
    pub fn ansi_color(&self, idx: u8) -> Rgba {
        match idx {
            0 => self.black,
            1 => self.red,
            2 => self.green,
            3 => self.yellow,
            4 => self.blue,
            5 => self.magenta,
            6 => self.cyan,
            7 => self.white,
            8 => self.bright_black,
            9 => self.bright_red,
            10 => self.bright_green,
            11 => self.bright_yellow,
            12 => self.bright_blue,
            13 => self.bright_magenta,
            14 => self.bright_cyan,
            15 => self.bright_white,
            _ => self.indexed_color(idx),
        }
    }

    /// xterm 256-color palette (16–255). Used by zsh autosuggest / completion grays.
    pub fn indexed_color(&self, idx: u8) -> Rgba {
        match idx {
            0..=15 => self.ansi_color(idx),
            16..=231 => {
                let i = idx - 16;
                let r = (i / 36) % 6;
                let g = (i / 6) % 6;
                let b = i % 6;
                let level = |c: u8| -> u8 { if c == 0 { 0 } else { 55 + (c - 1) * 40 } };
                Rgba::from_rgb(level(r), level(g), level(b))
            }
            232..=255 => {
                let level = 8 + (idx - 232) * 10;
                Rgba::from_rgb(level, level, level)
            }
        }
    }

    // ---- Built-in theme presets ----

    pub fn dracula() -> Self {
        Self {
            bg: Rgba::from_rgb(40, 42, 54),
            fg: Rgba::from_rgb(248, 248, 242),
            cursor: Rgba::from_rgb(248, 248, 242),
            selection: Rgba::from_rgba_premultiplied(68, 71, 90, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(139, 233, 253, 175),
            scrollbar_thumb_hover: Rgba::from_rgb(139, 233, 253),
            black: Rgba::from_rgb(33, 34, 44),
            red: Rgba::from_rgb(255, 85, 85),
            green: Rgba::from_rgb(80, 250, 123),
            yellow: Rgba::from_rgb(241, 250, 140),
            blue: Rgba::from_rgb(98, 114, 254),
            magenta: Rgba::from_rgb(255, 121, 198),
            cyan: Rgba::from_rgb(139, 233, 253),
            white: Rgba::from_rgb(248, 248, 242),
            bright_black: Rgba::from_rgb(98, 114, 164),
            bright_red: Rgba::from_rgb(255, 110, 110),
            bright_green: Rgba::from_rgb(105, 255, 140),
            bright_yellow: Rgba::from_rgb(255, 255, 170),
            bright_blue: Rgba::from_rgb(130, 150, 255),
            bright_magenta: Rgba::from_rgb(255, 140, 210),
            bright_cyan: Rgba::from_rgb(160, 245, 255),
            bright_white: Rgba::from_rgb(255, 255, 255),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            bg: Rgba::from_rgb(0, 43, 54),
            fg: Rgba::from_rgb(131, 148, 150),
            cursor: Rgba::from_rgb(131, 148, 150),
            selection: Rgba::from_rgba_premultiplied(7, 54, 66, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(131, 148, 150, 200),
            scrollbar_thumb_hover: Rgba::from_rgb(38, 139, 210),
            black: Rgba::from_rgb(7, 54, 66),
            red: Rgba::from_rgb(220, 50, 47),
            green: Rgba::from_rgb(133, 153, 0),
            yellow: Rgba::from_rgb(181, 137, 0),
            blue: Rgba::from_rgb(38, 139, 210),
            magenta: Rgba::from_rgb(211, 54, 130),
            cyan: Rgba::from_rgb(42, 161, 152),
            white: Rgba::from_rgb(238, 232, 213),
            bright_black: Rgba::from_rgb(0, 43, 54),
            bright_red: Rgba::from_rgb(203, 75, 22),
            bright_green: Rgba::from_rgb(88, 110, 117),
            bright_yellow: Rgba::from_rgb(101, 123, 131),
            bright_blue: Rgba::from_rgb(131, 148, 150),
            bright_magenta: Rgba::from_rgb(108, 113, 196),
            bright_cyan: Rgba::from_rgb(147, 161, 161),
            bright_white: Rgba::from_rgb(253, 246, 227),
        }
    }

    pub fn monokai() -> Self {
        Self {
            bg: Rgba::from_rgb(39, 40, 34),
            fg: Rgba::from_rgb(248, 248, 242),
            cursor: Rgba::from_rgb(248, 248, 240),
            selection: Rgba::from_rgba_premultiplied(73, 72, 62, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(166, 226, 46, 175),
            scrollbar_thumb_hover: Rgba::from_rgb(166, 226, 46),
            black: Rgba::from_rgb(39, 40, 34),
            red: Rgba::from_rgb(249, 38, 114),
            green: Rgba::from_rgb(166, 226, 46),
            yellow: Rgba::from_rgb(230, 219, 116),
            blue: Rgba::from_rgb(102, 217, 239),
            magenta: Rgba::from_rgb(174, 129, 255),
            cyan: Rgba::from_rgb(161, 239, 228),
            white: Rgba::from_rgb(248, 248, 242),
            bright_black: Rgba::from_rgb(117, 113, 94),
            bright_red: Rgba::from_rgb(249, 38, 114),
            bright_green: Rgba::from_rgb(166, 226, 46),
            bright_yellow: Rgba::from_rgb(230, 219, 116),
            bright_blue: Rgba::from_rgb(102, 217, 239),
            bright_magenta: Rgba::from_rgb(174, 129, 255),
            bright_cyan: Rgba::from_rgb(161, 239, 228),
            bright_white: Rgba::from_rgb(249, 248, 245),
        }
    }

    pub fn nord() -> Self {
        Self {
            bg: Rgba::from_rgb(46, 52, 64),
            fg: Rgba::from_rgb(216, 222, 233),
            cursor: Rgba::from_rgb(216, 222, 233),
            selection: Rgba::from_rgba_premultiplied(67, 76, 94, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(136, 192, 208, 180),
            scrollbar_thumb_hover: Rgba::from_rgb(136, 192, 208),
            black: Rgba::from_rgb(59, 66, 82),
            red: Rgba::from_rgb(191, 97, 106),
            green: Rgba::from_rgb(163, 190, 140),
            yellow: Rgba::from_rgb(235, 203, 139),
            blue: Rgba::from_rgb(129, 161, 193),
            magenta: Rgba::from_rgb(180, 142, 173),
            cyan: Rgba::from_rgb(136, 192, 208),
            white: Rgba::from_rgb(229, 233, 240),
            bright_black: Rgba::from_rgb(76, 86, 106),
            bright_red: Rgba::from_rgb(191, 97, 106),
            bright_green: Rgba::from_rgb(163, 190, 140),
            bright_yellow: Rgba::from_rgb(235, 203, 139),
            bright_blue: Rgba::from_rgb(129, 161, 193),
            bright_magenta: Rgba::from_rgb(180, 142, 173),
            bright_cyan: Rgba::from_rgb(136, 192, 208),
            bright_white: Rgba::from_rgb(236, 239, 244),
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            bg: Rgba::from_rgb(26, 27, 38),
            fg: Rgba::from_rgb(169, 177, 214),
            cursor: Rgba::from_rgb(169, 177, 214),
            selection: Rgba::from_rgba_premultiplied(54, 57, 79, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(122, 162, 247, 180),
            scrollbar_thumb_hover: Rgba::from_rgb(122, 162, 247),
            black: Rgba::from_rgb(26, 27, 38),
            red: Rgba::from_rgb(247, 118, 142),
            green: Rgba::from_rgb(158, 206, 106),
            yellow: Rgba::from_rgb(224, 175, 104),
            blue: Rgba::from_rgb(122, 162, 247),
            magenta: Rgba::from_rgb(187, 154, 247),
            cyan: Rgba::from_rgb(42, 195, 222),
            white: Rgba::from_rgb(169, 177, 214),
            bright_black: Rgba::from_rgb(65, 72, 104),
            bright_red: Rgba::from_rgb(247, 118, 142),
            bright_green: Rgba::from_rgb(158, 206, 106),
            bright_yellow: Rgba::from_rgb(224, 175, 104),
            bright_blue: Rgba::from_rgb(122, 162, 247),
            bright_magenta: Rgba::from_rgb(187, 154, 247),
            bright_cyan: Rgba::from_rgb(42, 195, 222),
            bright_white: Rgba::from_rgb(197, 202, 229),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            bg: Rgba::from_rgb(40, 40, 40),
            fg: Rgba::from_rgb(235, 219, 178),
            cursor: Rgba::from_rgb(235, 219, 178),
            selection: Rgba::from_rgba_premultiplied(60, 56, 54, 160),
            scrollbar_thumb: Rgba::from_rgba_premultiplied(215, 153, 33, 185),
            scrollbar_thumb_hover: Rgba::from_rgb(215, 153, 33),
            black: Rgba::from_rgb(40, 40, 40),
            red: Rgba::from_rgb(204, 36, 29),
            green: Rgba::from_rgb(152, 151, 26),
            yellow: Rgba::from_rgb(215, 153, 33),
            blue: Rgba::from_rgb(69, 133, 136),
            magenta: Rgba::from_rgb(177, 98, 134),
            cyan: Rgba::from_rgb(104, 157, 106),
            white: Rgba::from_rgb(168, 153, 132),
            bright_black: Rgba::from_rgb(146, 131, 116),
            bright_red: Rgba::from_rgb(251, 73, 52),
            bright_green: Rgba::from_rgb(184, 187, 38),
            bright_yellow: Rgba::from_rgb(250, 189, 47),
            bright_blue: Rgba::from_rgb(131, 165, 152),
            bright_magenta: Rgba::from_rgb(211, 134, 155),
            bright_cyan: Rgba::from_rgb(142, 192, 124),
            bright_white: Rgba::from_rgb(235, 219, 178),
        }
    }

    /// List of all built-in presets with their names.
    pub fn presets() -> [ThemePreset; 7] {
        [
            ("Default", Self::default),
            ("Dracula", Self::dracula),
            ("Solarized Dark", Self::solarized_dark),
            ("Monokai", Self::monokai),
            ("Nord", Self::nord),
            ("Tokyo Night", Self::tokyo_night),
            ("Gruvbox Dark", Self::gruvbox_dark),
        ]
    }
}

/// Built-in theme preset: display name + constructor.
pub type ThemePreset = (&'static str, fn() -> TerminalTheme);
