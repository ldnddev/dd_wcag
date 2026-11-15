//! Command line for headless contrast checks, palette export, and TUI seeds.
//!
//! Parsing finishes before the terminal enters raw mode. `check` and `palette`
//! print and exit. Any other invocation opens the TUI.

use crate::app::{ApcaTarget, App, InputTarget, Mode, WcagLevel};
use crate::color::{Color, apca_lookup_lc, is_large_text};
use crate::palette::{
    PALETTE_EXPORT_PATH, PALETTE_TOKENS_EXPORT_PATH, PaletteInput, PaletteState, generate_palette,
};
use clap::{Args, Parser, Subcommand, error::ErrorKind};
use serde::Serialize;
use std::path::{Path, PathBuf};

const DEFAULT_SIZE_PX: u16 = 16;
const DEFAULT_WEIGHT: u16 = 400;
const DEFAULT_APCA: u16 = 75;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Contrast,
    Palette,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeededColor {
    pub color: Color,
    pub text: String,
    pub format: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaunchSeed {
    pub tab: Tab,
    pub fg: Option<SeededColor>,
    pub bg: Option<SeededColor>,
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub tertiary: Option<String>,
    pub support: Option<String>,
    pub generate: bool,
    pub size_px: u16,
    pub weight: u16,
    pub wcag: WcagLevel,
    pub apca: ApcaTarget,
}

#[derive(Debug)]
pub struct CliOutput {
    pub code: u8,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum Dispatch {
    Tui(LaunchSeed),
    Output(CliOutput),
    ReadStdin(CheckSpec),
}

#[derive(Debug)]
pub struct CheckSpec {
    source: PairSource,
    size_px: u16,
    weight: u16,
    wcag: WcagLevel,
    apca: ApcaTarget,
    json: bool,
}

#[derive(Debug)]
enum PairSource {
    Inline(Vec<RawPair>),
    File(PathBuf),
    Stdin,
}

#[derive(Debug, Clone)]
struct RawPair {
    line: Option<usize>,
    fg: String,
    bg: String,
}

#[derive(Parser)]
#[command(
    name = "dd_wcag",
    version,
    about = "WCAG 2.x and APCA contrast checker",
    args_conflicts_with_subcommands = true,
    propagate_version = true,
    after_help = "\
With no subcommand, flags open the TUI. No arguments opens the Contrast tab.

Examples:
  dd_wcag
  dd_wcag --fg #1C1E21 --bg #F5F6F7 --size 18 --weight 700
  dd_wcag --primary #0B3D91 --secondary #C45508 --tertiary #1F7A4D --generate
  dd_wcag check --fg #1C1E21 --bg #FFFFFF --json
  dd_wcag check --pairs pairs.txt
  dd_wcag palette --primary #0B3D91 --secondary #C45508 --tertiary #1F7A4D --out ./theme
"
)]
struct LaunchArgs {
    #[command(subcommand)]
    command: Option<Command>,

    /// Tab to open: contrast or palette. Inferred from the color flags when omitted.
    #[arg(long, value_name = "contrast|palette")]
    tab: Option<String>,

    /// Foreground color for the Contrast tab.
    #[arg(long, value_name = "COLOR")]
    fg: Option<String>,

    /// Background color for the Contrast tab.
    #[arg(long, value_name = "COLOR")]
    bg: Option<String>,

    /// Primary brand color for the Palette tab.
    #[arg(long, value_name = "COLOR")]
    primary: Option<String>,

    /// Secondary brand color for the Palette tab.
    #[arg(long, value_name = "COLOR")]
    secondary: Option<String>,

    /// Tertiary brand color for the Palette tab.
    #[arg(long, value_name = "COLOR")]
    tertiary: Option<String>,

    /// Support color for the Palette tab.
    #[arg(long, value_name = "COLOR")]
    support: Option<String>,

    /// Generate the palette after opening the Palette tab.
    #[arg(long)]
    generate: bool,

    /// Font size in px, from 6 through 120.
    #[arg(long, value_name = "PX")]
    size: Option<u16>,

    /// Font weight, from 100 through 900 in steps of 100.
    #[arg(long, value_name = "WEIGHT")]
    weight: Option<u16>,

    /// Official WCAG level: aa or aaa.
    #[arg(long, value_name = "LEVEL")]
    wcag: Option<String>,

    /// Official APCA |Lc| bar: 45, 60, 75, or 90.
    #[arg(long, value_name = "LC")]
    apca: Option<u16>,
}

#[derive(Subcommand)]
enum Command {
    /// Print the WCAG ratio, AA/AAA results, and APCA Lc.
    Check(CheckArgs),
    /// Write _palette.scss and _palette.tokens.json from brand colors.
    Palette(PaletteArgs),
}

#[derive(Args)]
struct CheckArgs {
    /// Foreground (text) color. Required with --bg unless pairs come from a file or stdin.
    #[arg(long, value_name = "COLOR")]
    fg: Option<String>,

    /// Background (surface) color.
    #[arg(long, value_name = "COLOR")]
    bg: Option<String>,

    /// File of `fg bg` pairs, or `-` for stdin.
    #[arg(long, value_name = "FILE")]
    pairs: Option<String>,

    /// Font size in px, from 6 through 120.
    #[arg(long, default_value_t = DEFAULT_SIZE_PX, value_name = "PX")]
    size: u16,

    /// Font weight, from 100 through 900 in steps of 100.
    #[arg(long, default_value_t = DEFAULT_WEIGHT, value_name = "WEIGHT")]
    weight: u16,

    /// Official WCAG level used for the exit code: aa or aaa.
    #[arg(long, default_value = "aa", value_name = "LEVEL")]
    wcag: String,

    /// Official APCA |Lc| bar used for the exit code: 45, 60, 75, or 90.
    #[arg(long, default_value_t = DEFAULT_APCA, value_name = "LC")]
    apca: u16,

    /// Print JSON. One pair is an object. More than one pair is an array.
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct PaletteArgs {
    /// Required brand color.
    #[arg(long, value_name = "COLOR")]
    primary: Option<String>,

    /// Required brand color.
    #[arg(long, value_name = "COLOR")]
    secondary: Option<String>,

    /// Required brand color.
    #[arg(long, value_name = "COLOR")]
    tertiary: Option<String>,

    /// Optional support color. Defaults to #46BE8C.
    #[arg(long, value_name = "COLOR")]
    support: Option<String>,

    /// Directory for _palette.scss and _palette.tokens.json.
    #[arg(long, value_name = "DIR")]
    out: Option<PathBuf>,

    /// Write `scss` or the tokens `json` to stdout.
    #[arg(long, value_name = "scss|json", value_parser = ["scss", "json"])]
    stdout: Option<String>,

    /// Print the compliance summary as JSON.
    #[arg(long)]
    json: bool,
}

struct Metrics {
    size_px: u16,
    weight: u16,
    wcag: WcagLevel,
    apca: ApcaTarget,
}

#[derive(Debug, Serialize)]
struct PairReport {
    fg: String,
    bg: String,
    size_px: u16,
    weight: u16,
    large_text: bool,
    ratio: f64,
    wcag: WcagReport,
    apca: ApcaReport,
    pass: bool,
}

#[derive(Debug, Serialize)]
struct WcagReport {
    level: String,
    pass: bool,
    threshold: f64,
    aa: LevelVerdict,
    aaa: LevelVerdict,
}

#[derive(Debug, Serialize)]
struct LevelVerdict {
    pass: bool,
    threshold: f64,
}

#[derive(Debug, Serialize)]
struct ApcaReport {
    lc: f64,
    abs: f64,
    target: u16,
    label: String,
    pass: bool,
    lookup: f64,
    polarity: String,
    note: Option<String>,
}

enum StdoutArtifact {
    Scss,
    Tokens,
}

pub fn error(code: u8, message: impl std::fmt::Display) -> CliOutput {
    CliOutput {
        code,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

pub fn dispatch(args: &[String], stdin_is_tty: bool) -> Dispatch {
    match parse(args, stdin_is_tty) {
        Ok(Dispatch::ReadStdin(spec)) => Dispatch::ReadStdin(spec),
        Ok(other) => other,
        Err(output) => Dispatch::Output(output),
    }
}

fn parse(args: &[String], stdin_is_tty: bool) -> Result<Dispatch, CliOutput> {
    let parsed = match LaunchArgs::try_parse_from(args.iter().map(String::as_str)) {
        Ok(parsed) => parsed,
        Err(err) => return Err(from_clap(err)),
    };

    if let Some(command) = parsed.command {
        return match command {
            Command::Check(args) => dispatch_check(args, stdin_is_tty),
            Command::Palette(args) => Ok(Dispatch::Output(run_palette(args)?)),
        };
    }

    Ok(Dispatch::Tui(launch_seed(parsed)?))
}

fn from_clap(err: clap::Error) -> CliOutput {
    let code = u8::try_from(err.exit_code()).unwrap_or(2);
    let mut text = err.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if matches!(
        err.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        CliOutput {
            code,
            stdout: text,
            stderr: String::new(),
        }
    } else {
        CliOutput {
            code,
            stdout: String::new(),
            stderr: text,
        }
    }
}

fn launch_seed(args: LaunchArgs) -> Result<LaunchSeed, CliOutput> {
    let metrics = parse_metrics(
        args.size.unwrap_or(DEFAULT_SIZE_PX),
        args.weight.unwrap_or(DEFAULT_WEIGHT),
        args.wcag.as_deref().unwrap_or("aa"),
        args.apca.unwrap_or(DEFAULT_APCA),
    )
    .map_err(|message| error(2, message))?;

    let contrast_colors = args.fg.is_some() || args.bg.is_some();
    let palette_colors = args.primary.is_some()
        || args.secondary.is_some()
        || args.tertiary.is_some()
        || args.support.is_some();

    if contrast_colors && palette_colors {
        return Err(error(
            2,
            "cannot combine Contrast colors (--fg/--bg) with Palette colors",
        ));
    }
    if args.generate && contrast_colors {
        return Err(error(
            2,
            "cannot combine --generate with Contrast colors (--fg/--bg)",
        ));
    }

    let tab = match args.tab.as_deref() {
        Some(value) => parse_tab(value).map_err(|message| error(2, message))?,
        None if palette_colors || args.generate => Tab::Palette,
        None => Tab::Contrast,
    };
    if tab == Tab::Contrast && palette_colors {
        return Err(error(
            2,
            "cannot combine --tab contrast with Palette colors",
        ));
    }
    if tab == Tab::Palette && contrast_colors {
        return Err(error(
            2,
            "cannot combine --tab palette with Contrast colors (--fg/--bg)",
        ));
    }
    if tab == Tab::Contrast && args.generate {
        return Err(error(2, "--generate opens the Palette tab"));
    }

    Ok(LaunchSeed {
        tab,
        fg: optional_color("foreground", args.fg)?,
        bg: optional_color("background", args.bg)?,
        primary: optional_text(args.primary),
        secondary: optional_text(args.secondary),
        tertiary: optional_text(args.tertiary),
        support: optional_text(args.support),
        generate: args.generate,
        size_px: metrics.size_px,
        weight: metrics.weight,
        wcag: metrics.wcag,
        apca: metrics.apca,
    })
}

fn optional_color(label: &str, value: Option<String>) -> Result<Option<SeededColor>, CliOutput> {
    let Some(value) = value else {
        return Ok(None);
    };
    parse_seeded_color(label, &value)
        .map(Some)
        .map_err(|message| error(2, message))
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.map(|value| value.trim().to_string())
}

fn parse_seeded_color(label: &str, value: &str) -> Result<SeededColor, String> {
    let text = value.trim().to_string();
    if text.is_empty() {
        return Err(format!("invalid {label} color: empty color"));
    }
    let (color, format) =
        Color::parse_input(&text).map_err(|err| format!("invalid {label} color: {err}"))?;
    Ok(SeededColor {
        color,
        text,
        format,
    })
}

fn parse_tab(value: &str) -> Result<Tab, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "contrast" => Ok(Tab::Contrast),
        "palette" => Ok(Tab::Palette),
        _ => Err("tab must be contrast or palette".to_string()),
    }
}

fn parse_metrics(size: u16, weight: u16, wcag: &str, apca: u16) -> Result<Metrics, String> {
    if !(6..=120).contains(&size) {
        return Err("font size must be 6..=120".to_string());
    }
    if !(100..=900).contains(&weight) || !weight.is_multiple_of(100) {
        return Err("font weight must be 100..=900 in steps of 100".to_string());
    }
    let wcag = match wcag.trim().to_ascii_lowercase().as_str() {
        "aa" => WcagLevel::Aa,
        "aaa" => WcagLevel::Aaa,
        _ => return Err("WCAG level must be aa or aaa".to_string()),
    };
    let apca = match apca {
        45 => ApcaTarget::Lc45,
        60 => ApcaTarget::Lc60,
        75 => ApcaTarget::Lc75,
        90 => ApcaTarget::Lc90,
        _ => return Err("APCA target must be 45, 60, 75, or 90".to_string()),
    };
    Ok(Metrics {
        size_px: size,
        weight,
        wcag,
        apca,
    })
}

pub fn apply_launch_seed(app: &mut App, seed: &LaunchSeed) {
    app.font_size_px = seed.size_px;
    app.weight = seed.weight;
    app.is_bold = seed.weight >= 700;
    app.italic = false;
    app.style_chip = app
        .active_style_preset()
        .map(|preset| preset.index())
        .unwrap_or(0);
    app.targets.wcag = seed.wcag;
    app.targets.apca = seed.apca;

    if let Some(color) = &seed.fg {
        app.foreground = color.color;
        app.foreground_input = color.text.clone();
        app.parsed_fg = Some(color.color);
        app.last_parsed_format = Some(color.format.to_string());
    }
    if let Some(color) = &seed.bg {
        app.background = color.color;
        app.background_input = color.text.clone();
        app.parsed_bg = Some(color.color);
        app.last_parsed_format = Some(color.format.to_string());
    }
    app.update_contrast();

    if let Some(value) = &seed.primary {
        app.palette
            .set_role_hex(PaletteInput::Primary, value.clone());
    }
    if let Some(value) = &seed.secondary {
        app.palette
            .set_role_hex(PaletteInput::Secondary, value.clone());
    }
    if let Some(value) = &seed.tertiary {
        app.palette
            .set_role_hex(PaletteInput::Tertiary, value.clone());
    }
    if let Some(value) = &seed.support {
        app.palette
            .set_role_hex(PaletteInput::Support, value.clone());
    }
    app.palette.selected_idx = 0;

    match seed.tab {
        Tab::Contrast => {
            app.set_mode(Mode::Contrast);
            app.set_input_target(InputTarget::Foreground);
        }
        Tab::Palette => {
            app.set_mode(Mode::Palette);
            if seed.generate {
                app.generate_palette();
            }
        }
    }
}

fn dispatch_check(args: CheckArgs, stdin_is_tty: bool) -> Result<Dispatch, CliOutput> {
    let metrics = parse_metrics(args.size, args.weight, &args.wcag, args.apca)
        .map_err(|message| error(2, message))?;
    let source = pair_source(&args, stdin_is_tty).map_err(|message| error(2, message))?;
    let spec = CheckSpec {
        source,
        size_px: metrics.size_px,
        weight: metrics.weight,
        wcag: metrics.wcag,
        apca: metrics.apca,
        json: args.json,
    };
    if matches!(spec.source, PairSource::Stdin) {
        Ok(Dispatch::ReadStdin(spec))
    } else {
        Ok(Dispatch::Output(complete_check(&spec, "")))
    }
}

fn pair_source(args: &CheckArgs, stdin_is_tty: bool) -> Result<PairSource, String> {
    let has_fg = args
        .fg
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_bg = args
        .bg
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty());
    if args.fg.is_some() != args.bg.is_some() || has_fg != has_bg {
        return Err("pass both --fg and --bg".to_string());
    }
    if let Some(pairs) = &args.pairs {
        if has_fg {
            return Err("cannot combine --fg/--bg with --pairs".to_string());
        }
        if pairs == "-" {
            if stdin_is_tty {
                return Err("stdin is a terminal; pass --fg and --bg or --pairs FILE".to_string());
            }
            return Ok(PairSource::Stdin);
        }
        return Ok(PairSource::File(PathBuf::from(pairs)));
    }
    if has_fg {
        let fg = args.fg.clone().expect("fg is present");
        let bg = args.bg.clone().expect("bg is present");
        return Ok(PairSource::Inline(vec![RawPair {
            line: None,
            fg: fg.trim().to_string(),
            bg: bg.trim().to_string(),
        }]));
    }
    if stdin_is_tty {
        return Err("pass --fg and --bg, or --pairs FILE, or pipe pairs on stdin".to_string());
    }
    Ok(PairSource::Stdin)
}

pub fn complete_check(spec: &CheckSpec, stdin: &str) -> CliOutput {
    let loaded = match &spec.source {
        PairSource::Inline(pairs) => Ok(pairs.clone()),
        PairSource::File(path) => read_pairs_file(path),
        PairSource::Stdin => parse_pair_text(stdin),
    };
    let pairs = match loaded {
        Ok(pairs) => pairs,
        Err(message) => return error(2, message),
    };
    match evaluate_pairs(spec, &pairs) {
        Ok(reports) if reports.is_empty() => error(2, "no color pairs"),
        Ok(reports) => {
            let code = if reports.iter().all(|report| report.pass) {
                0
            } else {
                1
            };
            CliOutput {
                code,
                stdout: render_reports(&reports, spec.json),
                stderr: String::new(),
            }
        }
        Err(message) => error(2, message),
    }
}

fn read_pairs_file(path: &Path) -> Result<Vec<RawPair>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    parse_pair_text(&text)
}

fn parse_pair_text(text: &str) -> Result<Vec<RawPair>, String> {
    let mut pairs = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line_no = index + 1;
        let tokens = split_color_tokens(line).map_err(|err| format!("line {line_no}: {err}"))?;
        if tokens.len() != 2 {
            return Err(format!(
                "line {line_no}: expected two colors, found {}",
                tokens.len()
            ));
        }
        pairs.push(RawPair {
            line: Some(line_no),
            fg: tokens[0].clone(),
            bg: tokens[1].clone(),
        });
    }
    Ok(pairs)
}

fn split_color_tokens(line: &str) -> Result<Vec<String>, String> {
    let mut rest = line.trim();
    let mut tokens = Vec::new();
    while !rest.is_empty() {
        let lower = rest.to_ascii_lowercase();
        if lower.starts_with("rgba(") || lower.starts_with("rgb(") || lower.starts_with("hsl(") {
            let Some(close) = rest.find(')') else {
                return Err("unclosed color function".to_string());
            };
            tokens.push(rest[..=close].to_string());
            rest = rest[close + 1..].trim_start();
            continue;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        tokens.push(rest[..end].to_string());
        rest = rest[end..].trim_start();
    }
    Ok(tokens)
}

fn evaluate_pairs(spec: &CheckSpec, pairs: &[RawPair]) -> Result<Vec<PairReport>, String> {
    pairs.iter().map(|pair| evaluate_pair(spec, pair)).collect()
}

fn evaluate_pair(spec: &CheckSpec, pair: &RawPair) -> Result<PairReport, String> {
    let fg = parse_pair_color("foreground", pair, &pair.fg)?;
    let bg = parse_pair_color("background", pair, &pair.bg)?;
    Ok(report_pair(
        fg,
        bg,
        spec.size_px,
        spec.weight,
        spec.wcag,
        spec.apca,
    ))
}

fn parse_pair_color(label: &str, pair: &RawPair, value: &str) -> Result<Color, String> {
    match Color::parse_input(value) {
        Ok((color, _)) => Ok(color),
        Err(err) => {
            let message = format!("invalid {label} color: {err}");
            Err(match pair.line {
                Some(line) => format!("line {line}: {message}"),
                None => message,
            })
        }
    }
}

fn report_pair(
    fg: Color,
    bg: Color,
    size_px: u16,
    weight: u16,
    level: WcagLevel,
    apca_target: ApcaTarget,
) -> PairReport {
    let large_text = is_large_text(size_px, weight);
    let ratio = fg.contrast_ratio(&bg);
    let aa_threshold = WcagLevel::Aa.text_threshold(large_text);
    let aaa_threshold = WcagLevel::Aaa.text_threshold(large_text);
    let aa_pass = ratio >= aa_threshold;
    let aaa_pass = ratio >= aaa_threshold;
    let official_threshold = level.text_threshold(large_text);
    let wcag_pass = ratio >= official_threshold;
    let lc = fg.apca_lc(&bg);
    let abs = lc.abs();
    let target = apca_target.value();
    let apca_pass = abs >= target;
    let lookup = apca_lookup_lc(size_px, weight);
    PairReport {
        fg: fg.to_hex(),
        bg: bg.to_hex(),
        size_px,
        weight,
        large_text,
        ratio: round2(ratio),
        wcag: WcagReport {
            level: level.label().to_string(),
            pass: wcag_pass,
            threshold: official_threshold,
            aa: LevelVerdict {
                pass: aa_pass,
                threshold: aa_threshold,
            },
            aaa: LevelVerdict {
                pass: aaa_pass,
                threshold: aaa_threshold,
            },
        },
        apca: ApcaReport {
            lc: round2(lc),
            abs: round2(abs),
            target: target as u16,
            label: apca_target.label().to_string(),
            pass: apca_pass,
            lookup,
            polarity: if lc >= 0.0 {
                "light text".to_string()
            } else {
                "dark text".to_string()
            },
            note: apca_note(target, lookup),
        },
        pass: wcag_pass && apca_pass,
    }
}

fn apca_note(target: f64, lookup: f64) -> Option<String> {
    if target > lookup {
        Some(format!(
            "header Lc{target:.0} is stricter than lookup Lc{lookup:.0}"
        ))
    } else if target < lookup {
        Some(format!(
            "header Lc{target:.0} is looser than lookup Lc{lookup:.0}; body text may still need Lc{lookup:.0}"
        ))
    } else {
        None
    }
}

fn round2(value: f64) -> f64 {
    format!("{value:.2}").parse().expect("rounded number")
}

fn render_reports(reports: &[PairReport], json: bool) -> String {
    if json {
        let body = if reports.len() == 1 {
            serde_json::to_string(&reports[0]).expect("pair JSON")
        } else {
            serde_json::to_string(reports).expect("pair JSON")
        };
        return format!("{body}\n");
    }
    reports
        .iter()
        .map(render_pair_text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_pair_text(report: &PairReport) -> String {
    let kind = if report.large_text { "large" } else { "normal" };
    let mut text = format!(
        "{fg} on {bg}\nWCAG {ratio:.2}:1   AA {aa}   AAA {aaa}   {kind} text ({size}px / {weight})\nAPCA |Lc| {abs:.2}   {label} {apca}   lookup Lc{lookup:.0}\n",
        fg = report.fg,
        bg = report.bg,
        ratio = report.ratio,
        aa = pass_word(report.wcag.aa.pass),
        aaa = pass_word(report.wcag.aaa.pass),
        size = report.size_px,
        weight = report.weight,
        abs = report.apca.abs,
        label = report.apca.label,
        apca = pass_word(report.apca.pass),
        lookup = report.apca.lookup,
    );
    if let Some(note) = &report.apca.note {
        text.push_str(note);
        text.push('\n');
    }
    text
}

fn pass_word(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

fn run_palette(args: PaletteArgs) -> Result<CliOutput, CliOutput> {
    let primary = require_role("Primary", args.primary)?;
    let secondary = require_role("Secondary", args.secondary)?;
    let tertiary = require_role("Tertiary", args.tertiary)?;
    let support = match args.support {
        Some(value) => Some(require_role("Support", Some(value))?),
        None => None,
    };
    if args.stdout.is_some() && args.json {
        return Err(error(2, "--stdout and --json both write to stdout"));
    }
    let artifact = match args.stdout.as_deref() {
        Some("scss") => Some(StdoutArtifact::Scss),
        Some("json") => Some(StdoutArtifact::Tokens),
        Some(_) => return Err(error(2, "--stdout must be scss or json")),
        None => None,
    };
    if args.out.is_none() && artifact.is_none() && !args.json {
        return Err(error(2, "pass --out DIR, --stdout scss|json, or --json"));
    }

    let mut state = PaletteState::default();
    state.set_role_hex(PaletteInput::Primary, primary);
    state.set_role_hex(PaletteInput::Secondary, secondary);
    state.set_role_hex(PaletteInput::Tertiary, tertiary);
    if let Some(support) = support {
        state.set_role_hex(PaletteInput::Support, support);
    }
    let generated = generate_palette(&state).map_err(|message| error(2, message))?;

    let mut written = Vec::new();
    let scss_path = if let Some(dir) = &args.out {
        Some(write_palette_files(
            dir,
            &generated.scss,
            &generated.tokens_json,
        )?)
    } else {
        None
    };
    if let Some((scss, tokens)) = &scss_path {
        written.push(format!("Wrote {}", scss.display()));
        written.push(format!("Wrote {}", tokens.display()));
    }

    let code = if generated.blocking_failures().is_empty() {
        0
    } else {
        1
    };
    let human = render_palette_text(&written, &generated);
    let (stdout, stderr) = if args.json {
        let summary = palette_summary(&state, &generated, scss_path.as_ref());
        let stdout = format!(
            "{}\n",
            serde_json::to_string(&summary).expect("palette JSON")
        );
        let stderr = if written.is_empty() {
            String::new()
        } else {
            format!("{}\n", written.join("\n"))
        };
        (stdout, stderr)
    } else if let Some(artifact) = artifact {
        let body = match artifact {
            StdoutArtifact::Scss => ensure_newline(generated.scss),
            StdoutArtifact::Tokens => ensure_newline(generated.tokens_json),
        };
        (body, human)
    } else {
        (human, String::new())
    };
    Ok(CliOutput {
        code,
        stdout,
        stderr,
    })
}

fn require_role(name: &str, value: Option<String>) -> Result<String, CliOutput> {
    let Some(value) = value.map(|value| value.trim().to_string()) else {
        return Err(error(2, format!("{name} color is required")));
    };
    if value.is_empty() {
        return Err(error(2, format!("{name} color is required")));
    }
    parse_seeded_color(&name.to_ascii_lowercase(), &value).map_err(|message| error(2, message))?;
    Ok(value)
}

fn write_palette_files(
    dir: &Path,
    scss: &str,
    tokens: &str,
) -> Result<(PathBuf, PathBuf), CliOutput> {
    if dir.exists() && !dir.is_dir() {
        return Err(error(
            2,
            format!("output path is not a directory: {}", dir.display()),
        ));
    }
    std::fs::create_dir_all(dir)
        .map_err(|err| error(2, format!("failed to create {}: {err}", dir.display())))?;
    let scss_path = dir.join(PALETTE_EXPORT_PATH);
    let tokens_path = dir.join(PALETTE_TOKENS_EXPORT_PATH);
    std::fs::write(&scss_path, scss)
        .map_err(|err| error(2, format!("failed to write {}: {err}", scss_path.display())))?;
    if let Err(err) = std::fs::write(&tokens_path, tokens) {
        return Err(error(
            2,
            format!("failed to write {}: {err}", tokens_path.display()),
        ));
    }
    Ok((scss_path, tokens_path))
}

fn render_palette_text(written: &[String], generated: &crate::palette::GeneratedPalette) -> String {
    let mut out = String::new();
    for line in written {
        out.push_str(line);
        out.push('\n');
    }
    let blocking = generated.blocking_failures();
    out.push_str(&format!("Blocking failures: {}\n", blocking.len()));
    for check in blocking {
        out.push_str(&format!(
            "  {}  {:.2} < {:.2}\n",
            check.label, check.ratio, check.threshold
        ));
    }
    let advisory = generated.advisory_failures();
    out.push_str(&format!("Advisory warnings: {}\n", advisory.len()));
    for check in advisory {
        out.push_str(&format!(
            "  {}  {:.2} < {:.2}\n",
            check.label, check.ratio, check.threshold
        ));
    }
    out
}

#[derive(Serialize)]
struct PaletteSummary {
    primary: String,
    secondary: String,
    tertiary: String,
    support: String,
    pass: bool,
    blocking_failures: usize,
    advisory_warnings: usize,
    checks: Vec<CheckJson>,
    scss: Option<String>,
    tokens: Option<String>,
}

#[derive(Serialize)]
struct CheckJson {
    label: String,
    ratio: f64,
    threshold: f64,
    passes: bool,
    blocking: bool,
}

fn palette_summary(
    state: &PaletteState,
    generated: &crate::palette::GeneratedPalette,
    written: Option<&(PathBuf, PathBuf)>,
) -> PaletteSummary {
    PaletteSummary {
        primary: hex_of(&state.primary_input),
        secondary: hex_of(&state.secondary_input),
        tertiary: hex_of(&state.tertiary_input),
        support: hex_of(&state.support_input),
        pass: generated.blocking_failures().is_empty(),
        blocking_failures: generated.blocking_failures().len(),
        advisory_warnings: generated.advisory_failures().len(),
        checks: generated
            .checks
            .iter()
            .map(|check| CheckJson {
                label: check.label.clone(),
                ratio: round2(check.ratio),
                threshold: check.threshold,
                passes: check.passes,
                blocking: check.blocking,
            })
            .collect(),
        scss: written.map(|(path, _)| path.display().to_string()),
        tokens: written.map(|(_, path)| path.display().to_string()),
    }
}

fn hex_of(value: &str) -> String {
    Color::parse_input(value)
        .map(|(color, _)| color.to_hex())
        .unwrap_or_else(|_| value.to_string())
}

fn ensure_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ActiveTab, FocusId};
    use serde_json::Value;

    fn invoke(args: &[&str]) -> Invoked {
        invoke_with(args, true, "")
    }

    fn invoke_with(args: &[&str], tty: bool, stdin: &str) -> Invoked {
        let mut full = vec!["dd_wcag".to_string()];
        full.extend(args.iter().map(|arg| (*arg).to_string()));
        match dispatch(&full, tty) {
            Dispatch::Tui(seed) => Invoked::Tui(seed),
            Dispatch::Output(output) => Invoked::Out(output),
            Dispatch::ReadStdin(spec) => Invoked::Out(complete_check(&spec, stdin)),
        }
    }

    enum Invoked {
        Tui(LaunchSeed),
        Out(CliOutput),
    }

    #[track_caller]
    fn expect_tui(args: &[&str]) -> LaunchSeed {
        match invoke(args) {
            Invoked::Tui(seed) => seed,
            Invoked::Out(output) => panic!(
                "expected the TUI, got code {} stdout {:?} stderr {:?}",
                output.code, output.stdout, output.stderr
            ),
        }
    }

    #[track_caller]
    fn expect_out(args: &[&str]) -> CliOutput {
        expect_out_with(args, true, "")
    }

    #[track_caller]
    fn expect_out_with(args: &[&str], tty: bool, stdin: &str) -> CliOutput {
        match invoke_with(args, tty, stdin) {
            Invoked::Out(output) => output,
            Invoked::Tui(_) => panic!("expected CLI output, got the TUI"),
        }
    }

    #[track_caller]
    fn expect_err(args: &[&str], message: &str) {
        let output = expect_out(args);
        assert_eq!(
            output.code, 2,
            "stdout {:?} stderr {:?}",
            output.stdout, output.stderr
        );
        assert!(output.stdout.is_empty(), "{}", output.stdout);
        assert!(
            output.stderr.contains(message),
            "stderr {:?} missing {message}",
            output.stderr
        );
    }

    fn json_of(output: &CliOutput) -> Value {
        serde_json::from_str(output.stdout.trim())
            .unwrap_or_else(|err| panic!("invalid JSON {err}: {}", output.stdout))
    }

    fn gray(n: u8) -> String {
        format!("#{n:02x}{n:02x}{n:02x}")
    }

    fn color(value: &str) -> Color {
        Color::parse_input(value).expect("color").0
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("dd_wcag-cli-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn no_args_opens_contrast_with_app_defaults() {
        let seed = expect_tui(&[]);
        assert_eq!(seed.tab, Tab::Contrast);
        assert!(seed.fg.is_none());
        assert!(seed.bg.is_none());
        assert!(!seed.generate);
        assert_eq!(seed.size_px, 16);
        assert_eq!(seed.weight, 400);
        assert_eq!(seed.wcag, WcagLevel::Aa);
        assert_eq!(seed.apca, ApcaTarget::Lc75);
    }

    #[test]
    fn help_and_version_exit_before_the_tui() {
        for args in [
            &["--help"][..],
            &["-h"],
            &["check", "--help"],
            &["palette", "--help"],
        ] {
            let output = expect_out(args);
            assert_eq!(output.code, 0, "{args:?} stderr {}", output.stderr);
            assert!(output.stderr.is_empty(), "{}", output.stderr);
            assert!(
                output.stdout.contains("Usage"),
                "{args:?}\n{}",
                output.stdout
            );
        }
        let help = expect_out(&["--help"]);
        assert!(help.stdout.contains("check"));
        assert!(help.stdout.contains("palette"));
        assert!(help.stdout.contains("--fg"));
        assert!(help.stdout.contains("--generate"));

        let version = expect_out(&["--version"]);
        assert_eq!(version.code, 0);
        assert!(version.stdout.contains(env!("CARGO_PKG_VERSION")));
        let check_version = expect_out(&["check", "--version"]);
        assert_eq!(check_version.code, 0);
        assert!(check_version.stdout.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn contrast_flags_seed_the_contrast_tab() {
        let seed = expect_tui(&["--fg", "#abc", "--size", "18"]);
        assert_eq!(seed.tab, Tab::Contrast);
        assert_eq!(seed.fg.unwrap().text, "#abc");
        assert!(seed.bg.is_none());
        assert_eq!(seed.size_px, 18);

        let seed = expect_tui(&["--tab", "CONTRAST", "--bg", "rgb(1, 2, 3)"]);
        assert_eq!(seed.tab, Tab::Contrast);
        assert!(seed.fg.is_none());
        assert_eq!(seed.bg.unwrap().text, "rgb(1, 2, 3)");
    }

    #[test]
    fn palette_flags_seed_the_palette_tab() {
        let seed = expect_tui(&["--primary", "#112233"]);
        assert_eq!(seed.tab, Tab::Palette);
        assert_eq!(seed.primary.as_deref(), Some("#112233"));
        assert!(seed.secondary.is_none());
        assert!(!seed.generate);

        let seed = expect_tui(&["--generate"]);
        assert_eq!(seed.tab, Tab::Palette);
        assert!(seed.generate);
        assert!(seed.primary.is_none());

        let seed = expect_tui(&[
            "--tab",
            "palette",
            "--secondary",
            "#445566",
            "--support",
            "#778899",
            "--generate",
        ]);
        assert_eq!(seed.tab, Tab::Palette);
        assert!(seed.generate);
        assert_eq!(seed.secondary.as_deref(), Some("#445566"));
        assert_eq!(seed.support.as_deref(), Some("#778899"));
    }

    #[test]
    fn mixed_groups_and_bad_metrics_are_rejected() {
        expect_err(
            &["--fg", "#000", "--primary", "#fff"],
            "cannot combine Contrast colors",
        );
        expect_err(
            &["--generate", "--fg", "#000"],
            "cannot combine --generate with Contrast colors",
        );
        expect_err(
            &["--tab", "contrast", "--generate"],
            "--generate opens the Palette tab",
        );
        expect_err(
            &["--tab", "contrast", "--primary", "#112233"],
            "cannot combine --tab contrast with Palette colors",
        );
        expect_err(
            &["--tab", "palette", "--bg", "#fff"],
            "cannot combine --tab palette with Contrast colors",
        );
        expect_err(&["--tab", "both"], "tab must be contrast or palette");
        expect_err(&["--fg", "nope"], "invalid foreground color");
        expect_err(&["--size", "5"], "font size must be 6..=120");
        expect_err(&["--size", "121"], "font size must be 6..=120");
        expect_err(&["--weight", "450"], "font weight must be 100..=900");
        expect_err(&["--weight", "50"], "font weight must be 100..=900");
        expect_err(&["--wcag", "a"], "WCAG level must be aa or aaa");
        expect_err(&["--apca", "50"], "APCA target must be 45, 60, 75, or 90");
        expect_err(
            &["--fg", "#000", "check", "--fg", "#000", "--bg", "#fff"],
            "cannot be used with",
        );

        let seed = expect_tui(&["--wcag", "AAA", "--apca", "45", "--weight", "700"]);
        assert_eq!(seed.wcag, WcagLevel::Aaa);
        assert_eq!(seed.apca, ApcaTarget::Lc45);
        assert_eq!(seed.weight, 700);
        assert_eq!(seed.tab, Tab::Contrast);
    }

    #[test]
    fn launch_seed_fills_the_app() {
        let seed = expect_tui(&[
            "--fg",
            "#abc",
            "--bg",
            "rgb(0, 0, 0)",
            "--size",
            "18",
            "--weight",
            "700",
            "--wcag",
            "aaa",
            "--apca",
            "60",
        ]);
        let mut app = App::new();
        apply_launch_seed(&mut app, &seed);
        assert_eq!(app.mode, Mode::Contrast);
        assert_eq!(app.active_tab, ActiveTab::Contrast);
        assert_eq!(app.focus, FocusId::FgHex);
        assert_eq!(app.font_size_px, 18);
        assert_eq!(app.weight, 700);
        assert!(app.is_bold);
        assert!(!app.italic);
        assert_eq!(app.style_chip, 1);
        assert_eq!(app.targets.wcag, WcagLevel::Aaa);
        assert_eq!(app.targets.apca, ApcaTarget::Lc60);
        assert_eq!(app.foreground.to_hex(), "#aabbcc");
        assert_eq!(app.foreground_input, "#abc");
        assert_eq!(app.current_input, "#abc");
        assert_eq!(app.background_input, "rgb(0, 0, 0)");
        assert_eq!(app.background.to_hex(), "#000000");
        assert!((app.contrast_ratio - app.foreground.contrast_ratio(&app.background)).abs() < 1e-9);

        let seed = expect_tui(&["--tab", "palette", "--primary", "#112233"]);
        let mut app = App::new();
        apply_launch_seed(&mut app, &seed);
        assert_eq!(app.mode, Mode::Palette);
        assert_eq!(app.active_tab, ActiveTab::Palette);
        assert_eq!(app.focus, FocusId::Role(0));
        assert_eq!(app.palette.primary_input, "#112233");
        assert_eq!(app.palette.secondary_input, "#FFCA76");
        assert!(app.palette.generated.is_none());

        let seed = expect_tui(&["--primary", "#112233", "--generate", "--size", "20"]);
        let mut app = App::new();
        apply_launch_seed(&mut app, &seed);
        assert_eq!(app.mode, Mode::Palette);
        assert_eq!(app.font_size_px, 20);
        assert_eq!(app.palette.primary_input, "#112233");
        let generated = app.palette.generated.expect("palette generated");
        assert!(generated.scss.contains("$c_primary_default"));
    }

    #[test]
    fn check_black_on_white_reports_ratio_and_both_levels() {
        let output = expect_out(&["check", "--fg", "#000", "--bg", "#fff"]);
        assert_eq!(output.code, 0, "{}", output.stderr);
        assert!(output.stderr.is_empty());
        let lines: Vec<_> = output.stdout.lines().collect();
        assert_eq!(lines.len(), 3, "{}", output.stdout);
        assert_eq!(lines[0], "#000000 on #ffffff");
        assert_eq!(
            lines[1],
            "WCAG 21.00:1   AA pass   AAA pass   normal text (16px / 400)"
        );
        assert!(lines[2].starts_with("APCA |Lc| "));
        assert!(lines[2].contains("Lc75 pass"));
        assert!(lines[2].contains("lookup Lc75"));

        let output = expect_out(&["check", "--fg", "#000000", "--bg", "#ffffff", "--json"]);
        assert_eq!(output.code, 0);
        assert!(output.stdout.trim_start().starts_with('{'));
        let report = json_of(&output);
        assert_eq!(report["fg"], "#000000");
        assert_eq!(report["bg"], "#ffffff");
        assert_eq!(report["ratio"], 21.0);
        assert_eq!(report["large_text"], false);
        assert_eq!(report["pass"], true);
        assert_eq!(report["wcag"]["level"], "AA");
        assert_eq!(report["wcag"]["aa"]["pass"], true);
        assert_eq!(report["wcag"]["aa"]["threshold"], 4.5);
        assert_eq!(report["wcag"]["aaa"]["pass"], true);
        assert_eq!(report["wcag"]["aaa"]["threshold"], 7.0);
        assert_eq!(report["apca"]["target"], 75);
        assert_eq!(report["apca"]["pass"], true);
        assert_eq!(report["apca"]["lookup"], 75.0);
        assert!(report["apca"]["note"].is_null());
    }

    #[test]
    fn check_accepts_the_same_color_syntax_as_the_tui() {
        let output = expect_out(&[
            "check",
            "--fg",
            "rgb(0, 0, 0)",
            "--bg",
            "hsl(0, 0%, 100%)",
            "--json",
        ]);
        assert_eq!(output.code, 0, "{}", output.stderr);
        let report = json_of(&output);
        assert_eq!(report["fg"], "#000000");
        assert_eq!(report["bg"], "#ffffff");
        assert_eq!(report["ratio"], 21.0);

        let output = expect_out(&["check", "--fg", "rgba(0,0,0,1)", "--bg", "fff", "--json"]);
        assert_eq!(output.code, 0, "{}", output.stderr);
        assert_eq!(json_of(&output)["ratio"], 21.0);

        expect_err(
            &["check", "--fg", "rgba(0,0,0,2)", "--bg", "#fff"],
            "invalid foreground color",
        );
        expect_err(&["check", "--fg", "#000"], "pass both --fg and --bg");
        expect_err(
            &["check"],
            "pass --fg and --bg, or --pairs FILE, or pipe pairs on stdin",
        );
    }

    #[test]
    fn check_large_text_and_selected_wcag_level() {
        let fg = (0u8..=255)
            .rev()
            .map(gray)
            .find(|value| {
                let ratio = color(value).contrast_ratio(&color("#ffffff"));
                (5.2..6.4).contains(&ratio)
            })
            .expect("a gray between AA and AAA");

        let aa = expect_out(&[
            "check", "--fg", &fg, "--bg", "#ffffff", "--wcag", "aa", "--json",
        ]);
        let aaa = expect_out(&[
            "check", "--fg", &fg, "--bg", "#ffffff", "--wcag", "AAA", "--json",
        ]);
        let aa_report = json_of(&aa);
        let aaa_report = json_of(&aaa);
        assert_eq!(aa_report["large_text"], false);
        assert_eq!(aa_report["wcag"]["aa"]["pass"], true);
        assert_eq!(aa_report["wcag"]["aaa"]["pass"], false);
        assert_eq!(aa_report["wcag"]["pass"], true);
        assert_eq!(aa_report["wcag"]["threshold"], 4.5);
        assert_eq!(aaa_report["wcag"]["level"], "AAA");
        assert_eq!(aaa_report["wcag"]["pass"], false);
        assert_eq!(aaa_report["wcag"]["threshold"], 7.0);
        assert_eq!(aa_report["wcag"]["aa"], aaa_report["wcag"]["aa"]);
        assert_eq!(aa.code == 0, aa_report["pass"] == true);
        assert_eq!(aaa.code == 0, aaa_report["pass"] == true);

        let large = expect_out(&[
            "check", "--fg", &fg, "--bg", "#ffffff", "--size", "18", "--wcag", "aaa", "--json",
        ]);
        let large_report = json_of(&large);
        assert_eq!(large_report["large_text"], true);
        assert_eq!(large_report["wcag"]["aa"]["threshold"], 3.0);
        assert_eq!(large_report["wcag"]["aaa"]["threshold"], 4.5);
        assert_eq!(large_report["wcag"]["aaa"]["pass"], true);

        let text = expect_out(&["check", "--fg", &fg, "--bg", "#fff", "--size", "18"]);
        assert!(
            text.stdout.contains("large text (18px / 400)"),
            "{}",
            text.stdout
        );

        for (args, large_text) in [
            (&["--size", "14", "--weight", "400"][..], false),
            (&["--size", "14", "--weight", "700"][..], true),
            (&["--size", "13", "--weight", "700"][..], false),
            (&["--size", "17", "--weight", "400"][..], false),
        ] {
            let mut full = vec!["check", "--fg", "#000", "--bg", "#fff", "--json"];
            full.extend_from_slice(args);
            let report = json_of(&expect_out(&full));
            assert_eq!(report["large_text"], large_text, "{args:?}");
        }
    }

    #[test]
    fn check_apca_bar_changes_pass_and_lookup_stays_advisory() {
        let fg = (0u8..=255)
            .map(gray)
            .find(|value| {
                let lc = color(value).apca_lc(&color("#ffffff")).abs();
                (50.0..70.0).contains(&lc)
            })
            .expect("a gray between Lc45 and Lc75");
        let low = json_of(&expect_out(&[
            "check", "--fg", &fg, "--bg", "#ffffff", "--apca", "45", "--json",
        ]));
        let high = json_of(&expect_out(&[
            "check", "--fg", &fg, "--bg", "#ffffff", "--apca", "75", "--json",
        ]));
        assert_eq!(low["apca"]["pass"], true);
        assert_eq!(low["apca"]["target"], 45);
        assert_eq!(high["apca"]["pass"], false);
        assert_eq!(high["apca"]["target"], 75);
        assert!(
            (low["apca"]["abs"].as_f64().unwrap() - high["apca"]["abs"].as_f64().unwrap()).abs()
                < 0.001
        );

        let looser = expect_out(&["check", "--fg", "#000", "--bg", "#fff", "--apca", "45"]);
        assert_eq!(looser.code, 0, "{}", looser.stdout);
        assert!(
            looser.stdout.contains("looser than lookup Lc75"),
            "{}",
            looser.stdout
        );
        assert!(looser.stdout.contains("body text may still need Lc75"));
        let looser_json = json_of(&expect_out(&[
            "check", "--fg", "#000", "--bg", "#fff", "--apca", "45", "--json",
        ]));
        assert_eq!(looser_json["pass"], true);
        assert!(
            looser_json["apca"]["note"]
                .as_str()
                .unwrap()
                .contains("looser")
        );

        let stricter = expect_out(&[
            "check", "--fg", "#000", "--bg", "#fff", "--size", "24", "--apca", "75",
        ]);
        assert_eq!(stricter.code, 0);
        assert!(
            stricter
                .stdout
                .contains("header Lc75 is stricter than lookup Lc60"),
            "{}",
            stricter.stdout
        );

        let matched = json_of(&expect_out(&[
            "check", "--fg", "#000", "--bg", "#fff", "--size", "12", "--apca", "90", "--json",
        ]));
        assert_eq!(matched["apca"]["lookup"], 90.0);
        assert!(matched["apca"]["note"].is_null());
    }

    #[test]
    fn check_failing_pair_exits_1() {
        let output = expect_out(&["check", "--fg", "#888888", "--bg", "#999999"]);
        assert_eq!(output.code, 1, "{}", output.stdout);
        assert!(output.stdout.contains("AA fail"), "{}", output.stdout);
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn check_batch_file_and_stdin() {
        let scratch = Scratch::new("pairs");
        let path = scratch.path().join("pairs.txt");
        std::fs::write(
            &path,
            "\n#000000 #ffffff\nrgb(255, 255, 255) hsl(0, 0%, 0%)\n#888888 #999999\n",
        )
        .unwrap();
        let output = expect_out(&["check", "--pairs", path.to_str().unwrap(), "--json"]);
        assert_eq!(output.code, 1, "{}", output.stderr);
        let reports = json_of(&output);
        let reports = reports.as_array().expect("batch JSON is an array");
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0]["pass"], true);
        assert_eq!(reports[1]["fg"], "#ffffff");
        assert_eq!(reports[1]["bg"], "#000000");
        assert_eq!(reports[1]["ratio"], 21.0);
        assert_eq!(reports[2]["pass"], false);

        let passing = scratch.path().join("ok.txt");
        std::fs::write(&passing, "#000 #fff\n\n#fff #000\n").unwrap();
        let output = expect_out(&["check", "--pairs", passing.to_str().unwrap(), "--json"]);
        assert_eq!(output.code, 0, "{}", output.stdout);
        assert!(output.stdout.trim_start().starts_with('['));

        let stdin = expect_out_with(
            &["check", "--json"],
            false,
            "#000000 #ffffff\n#111111 #000000\n",
        );
        assert_eq!(stdin.code, 1);
        assert_eq!(json_of(&stdin).as_array().unwrap().len(), 2);

        let dashed = expect_out_with(&["check", "--pairs", "-", "--json"], false, "#000 #fff\n");
        assert_eq!(dashed.code, 0, "{}", dashed.stderr);
        assert!(dashed.stdout.trim_start().starts_with('{'));

        expect_err(&["check", "--pairs", "-"], "stdin is a terminal");
        expect_err(
            &[
                "check",
                "--fg",
                "#000",
                "--bg",
                "#fff",
                "--pairs",
                "pairs.txt",
            ],
            "cannot combine --fg/--bg with --pairs",
        );

        let bad = scratch.path().join("bad.txt");
        std::fs::write(&bad, "#000 #fff\nonly-one\n").unwrap();
        let output = expect_out(&["check", "--pairs", bad.to_str().unwrap()]);
        assert_eq!(output.code, 2);
        assert!(output.stdout.is_empty(), "{}", output.stdout);
        assert!(
            output
                .stderr
                .contains("line 2: expected two colors, found 1")
        );

        let broken = scratch.path().join("broken.txt");
        std::fs::write(&broken, "#000 #fff\nnotacolor #fff\n").unwrap();
        let output = expect_out(&["check", "--pairs", broken.to_str().unwrap()]);
        assert_eq!(output.code, 2);
        assert!(output.stderr.contains("line 2: invalid foreground color"));

        let empty = expect_out_with(&["check"], false, "\n\n");
        assert_eq!(empty.code, 2);
        assert!(empty.stderr.contains("no color pairs"));

        let missing = expect_out(&["check", "--pairs", "/no/such/dd_wcag_pairs.txt"]);
        assert_eq!(missing.code, 2);
        assert!(missing.stderr.contains("failed to read"));
    }

    #[test]
    fn palette_export_matches_the_generator() {
        let scratch = Scratch::new("export");
        let dir = scratch.path().join("theme");
        let output = expect_out(&[
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--support",
            "#46BE8C",
            "--out",
            dir.to_str().unwrap(),
        ]);
        assert_eq!(output.code, 0, "{}", output.stderr);
        let expected = generate_palette(&PaletteState::default()).unwrap();
        let scss = std::fs::read_to_string(dir.join("_palette.scss")).unwrap();
        let tokens = std::fs::read_to_string(dir.join("_palette.tokens.json")).unwrap();
        assert_eq!(scss, expected.scss);
        assert_eq!(tokens, expected.tokens_json);
        assert!(
            output.stdout.contains("Blocking failures: 0"),
            "{}",
            output.stdout
        );
        assert!(
            output
                .stdout
                .contains(&dir.join("_palette.scss").display().to_string())
        );
        assert!(expected.blocking_failures().is_empty());

        let summary = expect_out(&[
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--json",
        ]);
        assert_eq!(summary.code, 0, "{}", summary.stderr);
        let summary = json_of(&summary);
        assert_eq!(summary["pass"], true);
        assert_eq!(summary["blocking_failures"], 0);
        assert_eq!(summary["support"], "#46be8c");
        assert!(summary["scss"].is_null());
        assert!(summary["checks"].as_array().unwrap().len() > 10);

        let piped = expect_out(&[
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--stdout",
            "scss",
        ]);
        assert_eq!(piped.code, 0);
        assert_eq!(piped.stdout, ensure_newline(expected.scss.clone()));
        assert!(piped.stderr.contains("Blocking failures: 0"));
        assert!(!piped.stdout.contains("Blocking failures"));
    }

    #[test]
    fn palette_blocking_failure_still_writes_and_exits_1() {
        let scratch = Scratch::new("failing");
        let dir = scratch.path().join("nested").join("out");
        let primary = (0u8..=255)
            .map(|channel| format!("#{channel:02x}{channel:02x}{channel:02x}"))
            .find(|hex| {
                let mut state = PaletteState::default();
                state.set_role_hex(PaletteInput::Primary, hex.clone());
                generate_palette(&state)
                    .expect("palette")
                    .blocking_failures()
                    .iter()
                    .any(|check| check.label.contains("primary"))
            })
            .expect("a primary gray that fails a blocking check");
        let mut state = PaletteState::default();
        state.set_role_hex(PaletteInput::Primary, primary.clone());
        let expected = generate_palette(&state).unwrap();

        let output = expect_out(&[
            "palette",
            "--primary",
            &primary,
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--out",
            dir.to_str().unwrap(),
            "--json",
        ]);
        assert_eq!(output.code, 1, "{}", output.stderr);
        assert!(output.stderr.contains("Wrote"));
        let summary = json_of(&output);
        assert_eq!(summary["pass"], false);
        assert!(summary["blocking_failures"].as_u64().unwrap() > 0);
        assert_eq!(summary["primary"], primary);
        let scss = std::fs::read_to_string(dir.join("_palette.scss")).unwrap();
        let tokens = std::fs::read_to_string(dir.join("_palette.tokens.json")).unwrap();
        assert_eq!(scss, expected.scss);
        assert_eq!(tokens, expected.tokens_json);
        assert!(scss.contains("$c_text_primary"));
    }

    #[test]
    fn palette_usage_rules() {
        expect_err(&["palette"], "Primary color is required");
        expect_err(
            &["palette", "--primary", "#000", "--secondary", "#111"],
            "Tertiary color is required",
        );
        expect_err(
            &[
                "palette",
                "--primary",
                "#000",
                "--secondary",
                "#111",
                "--tertiary",
                "nope",
            ],
            "invalid tertiary color",
        );
        expect_err(
            &[
                "palette",
                "--primary",
                "#000",
                "--secondary",
                "#111",
                "--tertiary",
                "#222",
            ],
            "pass --out DIR, --stdout scss|json, or --json",
        );
        expect_err(
            &[
                "palette",
                "--primary",
                "#000",
                "--secondary",
                "#111",
                "--tertiary",
                "#222",
                "--stdout",
                "scss",
                "--json",
            ],
            "--stdout and --json both write to stdout",
        );

        let scratch = Scratch::new("not-dir");
        let file_path = scratch.path().join("file.txt");
        std::fs::write(&file_path, "x").unwrap();
        let output = expect_out(&[
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--out",
            file_path.to_str().unwrap(),
        ]);
        assert_eq!(output.code, 2);
        assert!(output.stderr.contains("not a directory"));

        let tokens = expect_out(&[
            "palette",
            "--primary",
            "#88D9F7",
            "--secondary",
            "#FFCA76",
            "--tertiary",
            "#F98971",
            "--stdout",
            "json",
        ]);
        assert_eq!(tokens.code, 0, "{}", tokens.stderr);
        assert!(tokens.stdout.trim_start().starts_with('{'));
        assert!(tokens.stdout.contains("\"Light\""));
        assert!(tokens.stderr.contains("Advisory warnings"));
    }
}
