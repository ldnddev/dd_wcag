use crate::app::App;
use std::io;
use std::path::PathBuf;
use std::process::Command;

const PREVIEW_PATH: &str = "/tmp/dd_wcag_preview.html";

pub fn sync(app: &App) -> io::Result<()> {
    let fg = app.foreground.to_hex();
    let bg = app.background.to_hex();
    let font_weight = if app.is_bold { "700" } else { "400" };
    let text = escape_html(&app.preview_text);

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta http-equiv="refresh" content="0.4" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>dd_wcag Preview</title>
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
      font-size: {size}px;
      font-weight: {weight};
      line-height: 1.35;
      word-break: break-word;
    }}
  </style>
</head>
<body>
  <div class="panel">
    <div class="meta">
      <div><strong>Size:</strong> {size}px</div>
      <div><strong>Weight:</strong> {weight_label}</div>
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
        size = app.font_size_px,
        weight = font_weight,
        weight_label = if app.is_bold { "bold" } else { "normal" },
        text = text
    );

    std::fs::write(PREVIEW_PATH, html)
}

pub fn open_in_browser() -> io::Result<()> {
    let path = preview_path();
    let path_str = path.to_string_lossy();

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path_str.as_ref()).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path_str.as_ref()).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", path_str.as_ref()])
            .spawn()?;
    }

    Ok(())
}

pub fn preview_path() -> PathBuf {
    PathBuf::from(PREVIEW_PATH)
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
