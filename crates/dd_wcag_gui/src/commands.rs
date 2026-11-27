//! Thin window adapter over [`dd_wcag::commands`].
//!
//! Chords live in the library. This module turns a shell key token into that chord.

use dd_wcag::commands::{self as core, Chord};

pub use core::Command;

pub fn command_for_key(key: &str) -> Option<Command> {
    let chord = Chord::parse(key.trim())?;
    core::command_for_chord(chord)
}

pub fn is_settings(key: &str) -> bool {
    matches!(
        command_for_key(key),
        Some(Command::Settings | Command::Theme)
    )
}

pub fn is_quit(key: &str) -> bool {
    command_for_key(key) == Some(Command::Quit)
}
