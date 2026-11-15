//! Process-level checks that `main` maps CLI results onto exit codes.
//! Behavior of the commands themselves is covered in `src/cli.rs`.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dd_wcag"))
}

#[test]
fn help_and_version_do_not_open_the_tui() {
    let help = bin().arg("--help").output().unwrap();
    assert!(help.status.success());
    let stdout = String::from_utf8(help.stdout).unwrap();
    assert!(stdout.contains("check"), "{stdout}");
    assert!(stdout.contains("palette"), "{stdout}");
    assert!(help.stderr.is_empty());

    let version = bin().arg("--version").output().unwrap();
    assert!(version.status.success());
    let stdout = String::from_utf8(version.stdout).unwrap();
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")), "{stdout}");
}

#[test]
fn check_exit_codes_match_the_report() {
    let pass = bin()
        .args(["check", "--fg", "#000000", "--bg", "#ffffff"])
        .output()
        .unwrap();
    assert_eq!(pass.status.code(), Some(0));
    let stdout = String::from_utf8(pass.stdout).unwrap();
    assert!(stdout.contains("#000000 on #ffffff"), "{stdout}");
    assert!(stdout.contains("WCAG 21.00:1"), "{stdout}");
    assert!(stdout.contains("AA pass"), "{stdout}");
    assert!(stdout.contains("AAA pass"), "{stdout}");
    assert!(pass.stderr.is_empty());

    let fail = bin()
        .args(["check", "--fg", "#888888", "--bg", "#999999", "--json"])
        .output()
        .unwrap();
    assert_eq!(fail.status.code(), Some(1));
    let stdout = String::from_utf8(fail.stdout).unwrap();
    assert!(stdout.contains("\"pass\":false"), "{stdout}");

    let bad = bin()
        .args(["check", "--fg", "nope", "--bg", "#ffffff"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    let stderr = String::from_utf8(bad.stderr).unwrap();
    assert!(stderr.contains("invalid foreground color"), "{stderr}");
}

#[test]
fn palette_export_writes_both_files() {
    let dir = std::env::temp_dir().join(format!("dd_wcag-bin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let output = bin()
        .args([
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--out",
        ])
        .arg(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(dir.join("_palette.scss").is_file());
    assert!(dir.join("_palette.tokens.json").is_file());
    let scss = std::fs::read_to_string(dir.join("_palette.scss")).unwrap();
    assert!(scss.contains("$c_primary_default"));
    assert!(scss.contains("$c_text_primary"));
    let _ = std::fs::remove_dir_all(&dir);
}
