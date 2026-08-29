use crate::app::App;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const PREVIEW_PATH: &str = "/tmp/dd_wcag_preview.html";

pub fn sync(app: &App) -> io::Result<()> {
    let fg = app.foreground.to_hex();
    let bg = app.background.to_hex();
    let font_family = app.preview_font_family.trim();
    let font_query = google_font_family_query(font_family);
    let google_font_link = if font_query.is_empty() {
        String::new()
    } else {
        format!(
            r#"<link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family={font_query}&display=swap" rel="stylesheet">"#
        )
    };
    let text = escape_html(&app.preview_text);

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>dd_wcag Preview</title>
  {google_font_link}
  <style>
    :root {{ color-scheme: light dark; }}
    body {{
      margin: 0;
      font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
      background: #111;
      color: #eee;
      display: grid;
      place-items: center;
      min-height: 100vh;
      padding: 24px;
      box-sizing: border-box;
    }}
    .panel {{
      width: min(900px, 95vw);
      border: 1px solid #333;
      border-radius: 8px;
      overflow: hidden;
      background: #1a1a1a;
    }}
    .meta {{
      font-size: 13px;
      color: #bbb;
      padding: 10px 12px;
      border-bottom: 1px solid #333;
      display: flex;
      gap: 14px;
      flex-wrap: wrap;
    }}
    .sample {{
      padding: 20px 24px;
      background: {bg};
      color: {fg};
      font-family: '{font_family}', ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
      font-size: {size}px;
      line-height: 1.35;
      word-break: break-word;
    }}
  </style>
</head>
<body>
  <div class="panel">
    <div class="meta">
      <div><strong>Size:</strong> {size}px</div>
      <div><strong>Weight:</strong> {weight_label} (not applied in web preview)</div>
      <div><strong>Font:</strong> {font_family}</div>
      <div><strong>FG:</strong> {fg}</div>
      <div><strong>BG:</strong> {bg}</div>
    </div>
    <div class="sample">{text}</div>
  </div>
</body>
</html>
"#,
        fg = fg,
        bg = bg,
        font_family = font_family,
        size = app.font_size_px,
        weight_label = if app.is_bold { "bold" } else { "normal" },
        google_font_link = google_font_link,
        text = text
    );

    std::fs::write(PREVIEW_PATH, html)
}

pub fn open_in_browser() -> io::Result<()> {
    let path = preview_path();
    let path_str = path.to_string_lossy();

    #[cfg(target_os = "linux")]
    {
        spawn_detached("xdg-open", &[path_str.as_ref()])?;
    }
    #[cfg(target_os = "macos")]
    {
        spawn_detached("open", &[path_str.as_ref()])?;
    }
    #[cfg(target_os = "windows")]
    {
        spawn_detached("cmd", &["/C", "start", path_str.as_ref()])?;
    }

    Ok(())
}

pub fn preview_path() -> PathBuf {
    PathBuf::from(PREVIEW_PATH)
}

fn spawn_detached(program: &str, args: &[&str]) -> io::Result<()> {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn google_font_family_query(font_family: &str) -> String {
    font_family
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => Some(c),
            ' ' => Some('+'),
            _ => None,
        })
        .collect::<String>()
}
