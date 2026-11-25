//! Shortcut sheet. Both columns come from the shared command list.

pub use dd_wcag::commands::SheetRow as Shortcut;

pub fn rows() -> &'static [Shortcut] {
    dd_wcag::commands::shortcut_sheet()
}

pub fn lines() -> Vec<String> {
    let mut out = vec!["Shortcut sheet".to_string()];
    let mut section = "";
    for row in rows() {
        if row.section != section {
            section = row.section;
            out.push(section.to_string());
        }
        out.push(format!("{} · {} · {}", row.action, row.gui, row.tui));
    }
    out
}
