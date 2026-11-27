//! One binding list for the terminal and the window.
//!
//! The window adapter parses these tokens. The terminal already toggles
//! Settings on F1 and F2; that path stays in the TUI so its key tests hold.
//! The chords still live here.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    Settings,
    /// Opens the same sheet. No chords, so it is not a second key.
    Theme,
    Quit,
}

/// One key chord, without a frontend's string spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord<'a> {
    pub key: &'a str,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

impl<'a> Chord<'a> {
    pub const fn plain(key: &'a str) -> Self {
        Self {
            key,
            control: false,
            alt: false,
            shift: false,
        }
    }

    pub const fn ctrl(key: &'a str) -> Self {
        Self {
            key,
            control: true,
            alt: false,
            shift: false,
        }
    }

    /// Parse a window token (`f1`, `ctrl-/`, `ctrl-,`, `ctrl-q`).
    pub fn parse(token: &'a str) -> Option<Self> {
        let token = token.trim();
        if token.is_empty() {
            return None;
        }
        let mut parts: Vec<&str> = token.split('-').collect();
        let key = parts.pop()?;
        if key.is_empty() {
            return None;
        }
        let mut control = false;
        let mut alt = false;
        let mut shift = false;
        for part in parts {
            match part {
                "ctrl" => control = true,
                "alt" => alt = true,
                "shift" => shift = true,
                _ => return None,
            }
        }
        Some(Self {
            key,
            control,
            alt,
            shift,
        })
    }
}

/// Window token for a chord.
pub fn chord_token(chord: Chord<'_>) -> String {
    let mut parts = Vec::new();
    if chord.control {
        parts.push("ctrl");
    }
    if chord.shift {
        parts.push("shift");
    }
    if chord.alt {
        parts.push("alt");
    }
    parts.push(chord.key);
    parts.join("-")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Primary,
    Alias,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub command: Command,
    pub title: &'static str,
    pub detail: &'static str,
    pub kbd: &'static str,
    pub chords: &'static [Chord<'static>],
    pub kind: BindingKind,
    pub in_palette: bool,
}

const COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        command: Command::Settings,
        title: "Settings",
        detail: "Themes, window, and shortcuts",
        kbd: "F1",
        chords: &[
            Chord::plain("f1"),
            Chord::ctrl("/"),
            Chord::plain("f2"),
            Chord::ctrl(","),
        ],
        kind: BindingKind::Primary,
        in_palette: true,
    },
    CommandSpec {
        command: Command::Theme,
        title: "Theme",
        detail: "Settings",
        kbd: "",
        chords: &[],
        kind: BindingKind::Primary,
        in_palette: false,
    },
    CommandSpec {
        command: Command::Quit,
        title: "Quit",
        detail: "Leave the app",
        kbd: "Ctrl Q",
        chords: &[Chord::ctrl("q")],
        kind: BindingKind::Primary,
        in_palette: false,
    },
];

pub fn registry() -> &'static [CommandSpec] {
    COMMANDS
}

pub fn spec(command: Command) -> &'static CommandSpec {
    registry()
        .iter()
        .find(|spec| spec.command == command)
        .expect("command is registered")
}

pub fn command_for_chord(chord: Chord<'_>) -> Option<Command> {
    registry().iter().find_map(|spec| {
        spec.chords
            .iter()
            .any(|bound| bound.key == chord.key && same_mods(*bound, chord))
            .then_some(spec.command)
    })
}

fn same_mods(bound: Chord<'_>, chord: Chord<'_>) -> bool {
    bound.control == chord.control && bound.alt == chord.alt && bound.shift == chord.shift
}

/// One shortcut-sheet row. The first column is the window key. The second is the terminal key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SheetRow {
    pub section: &'static str,
    pub action: &'static str,
    pub gui: &'static str,
    pub tui: &'static str,
}

pub fn shortcut_sheet() -> &'static [SheetRow] {
    SHEET
}

const SHEET: &[SheetRow] = &[
    SheetRow {
        section: "Global",
        action: "Settings",
        gui: "F1",
        tui: "F1",
    },
    SheetRow {
        section: "Global",
        action: "Quit",
        gui: "Ctrl Q",
        tui: "Ctrl Q",
    },
    SheetRow {
        section: "Global",
        action: "Contrast",
        gui: "1",
        tui: "1",
    },
    SheetRow {
        section: "Global",
        action: "Palette",
        gui: "2",
        tui: "2",
    },
    SheetRow {
        section: "Global",
        action: "Next field",
        gui: "Tab",
        tui: "Tab",
    },
    SheetRow {
        section: "Global",
        action: "Previous field",
        gui: "Shift Tab",
        tui: "Shift Tab",
    },
    SheetRow {
        section: "Global",
        action: "Paste",
        gui: "Ctrl V",
        tui: "Ctrl V",
    },
    SheetRow {
        section: "Global",
        action: "Fix",
        gui: "Ctrl F",
        tui: "Ctrl F",
    },
    SheetRow {
        section: "Global",
        action: "Browser preview",
        gui: "Ctrl O",
        tui: "Ctrl O",
    },
    SheetRow {
        section: "Global",
        action: "Generate palette",
        gui: "Ctrl G",
        tui: "Ctrl G",
    },
    SheetRow {
        section: "Global",
        action: "Blur or close",
        gui: "Esc",
        tui: "Esc",
    },
    SheetRow {
        section: "Contrast",
        action: "Cycle style",
        gui: "Ctrl S",
        tui: "Ctrl S",
    },
    SheetRow {
        section: "Contrast",
        action: "Copy hex",
        gui: "Ctrl C",
        tui: "Ctrl C",
    },
    SheetRow {
        section: "Contrast",
        action: "Bold preset",
        gui: "Ctrl B",
        tui: "Ctrl B",
    },
    SheetRow {
        section: "Contrast",
        action: "Next font",
        gui: "Ctrl T",
        tui: "Ctrl T",
    },
    SheetRow {
        section: "Contrast",
        action: "Step size or weight",
        gui: "Up Down",
        tui: "Up Down",
    },
    SheetRow {
        section: "Contrast",
        action: "Lightness",
        gui: "[ ]",
        tui: "[ ]",
    },
    SheetRow {
        section: "Contrast",
        action: "Hue",
        gui: "{ }",
        tui: "{ }",
    },
    SheetRow {
        section: "Contrast",
        action: "Swap colors",
        gui: "Space",
        tui: "Space",
    },
    SheetRow {
        section: "Palette",
        action: "Save SCSS and tokens",
        gui: "Ctrl S",
        tui: "Ctrl S",
    },
    SheetRow {
        section: "Palette",
        action: "Copy SCSS",
        gui: "Ctrl C",
        tui: "Ctrl C",
    },
    SheetRow {
        section: "Palette",
        action: "Edit role",
        gui: "Enter",
        tui: "Enter",
    },
    SheetRow {
        section: "Palette",
        action: "Open Fix on pair",
        gui: "Enter",
        tui: "Enter",
    },
    SheetRow {
        section: "Fix",
        action: "Next candidate",
        gui: "Ctrl N",
        tui: "Ctrl N",
    },
    SheetRow {
        section: "Fix",
        action: "Send to role",
        gui: "P S T U",
        tui: "P S T U",
    },
    SheetRow {
        section: "Settings",
        action: "Close and revert",
        gui: "Esc",
        tui: "Esc",
    },
    SheetRow {
        section: "Settings",
        action: "Next page",
        gui: "Tab",
        tui: "Tab",
    },
    SheetRow {
        section: "Settings",
        action: "Save target",
        gui: "t",
        tui: "t",
    },
    SheetRow {
        section: "Settings",
        action: "Save theme",
        gui: "y",
        tui: "y",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_primary_chords_and_theme_has_none() {
        let settings = spec(Command::Settings);
        assert_eq!(settings.title, "Settings");
        assert_eq!(settings.kind, BindingKind::Primary);
        let tokens: Vec<_> = settings.chords.iter().copied().map(chord_token).collect();
        assert_eq!(tokens, vec!["f1", "ctrl-/", "f2", "ctrl-,"]);
        assert!(spec(Command::Theme).chords.is_empty());
        assert_eq!(spec(Command::Theme).kbd, "");
        assert!(!spec(Command::Theme).in_palette);
        for token in ["f1", "ctrl-/", "f2", "ctrl-,"] {
            let chord = Chord::parse(token).unwrap();
            assert_eq!(command_for_chord(chord), Some(Command::Settings));
        }
        assert_eq!(
            command_for_chord(Chord::parse("ctrl-q").unwrap()),
            Some(Command::Quit)
        );
        assert!(command_for_chord(Chord::parse("q").unwrap()).is_none());
        let first = shortcut_sheet()[0];
        assert_eq!(first.section, "Global");
        assert_eq!(
            format!("{} · {} · {}", first.action, first.gui, first.tui),
            "Settings · F1 · F1"
        );
    }
}
