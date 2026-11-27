//! dd_wcag window. Print-only commands never open a window.

mod clipboard;
mod commands;
mod paint;
mod product;
mod screen;
mod settings;
mod shell;
mod shortcuts;
mod theme_load;
mod windows;

#[cfg(test)]
mod tests;

use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use dd_wcag::cli::{self, Dispatch};
use gpui::{App, AppContext, Bounds, Focusable, px, size};
use gpui_component::Root;
use shell::{Shell, ShellOptions};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let stdin_is_tty = io::stdin().is_terminal();
    match cli::dispatch(&args, stdin_is_tty) {
        Dispatch::Output(output) => emit(output),
        Dispatch::ReadStdin(spec) => {
            let mut stdin = String::new();
            if io::stdin().read_to_string(&mut stdin).is_err() {
                let _ = writeln!(io::stderr(), "error: failed to read stdin");
                return ExitCode::from(2);
            }
            emit(cli::complete_check(&spec, &stdin))
        }
        Dispatch::Tui(seed) => {
            open_window(seed);
            ExitCode::SUCCESS
        }
    }
}

fn emit(output: dd_wcag::cli::CliOutput) -> ExitCode {
    let _ = write!(io::stdout(), "{}", output.stdout);
    let _ = write!(io::stderr(), "{}", output.stderr);
    ExitCode::from(output.code)
}

fn open_window(seed: dd_wcag::cli::LaunchSeed) {
    gpui_platform::application().run(move |cx: &mut App| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            cx.update(|cx| {
                let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
                cx.open_window(windows::main_window_options(bounds), |window, cx| {
                    let view = cx.new(|cx| {
                        let shell = Shell::open(ShellOptions::production(seed), cx);
                        window.focus(&shell.focus_handle(cx), cx);
                        shell
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("open window");
            });
        })
        .detach();
    });
}
