#!/usr/bin/env bash
# Capture tutorial screenshots for docs/tutorial.html.
#
# Run from anywhere; the script finds the repo root:
#   ./docs/capture-screenshots.sh
#
# Required (Wayland session):
#   foot, tmux, grim, hyprctl
# Optional:
#   chromium          — web-preview screenshot
#
# Environment:
#   SKIP_WEB=1        skip the chromium shot
#   KEEP_WINDOW=1     leave the last capture window open
#   SHOT_SLEEP=0.35   delay before each grim capture (seconds)
#
# Image catalog (docs/images/):
#   contrast.png             Default Contrast tab (black on white)
#   help.png                 F1 Keys & Mouse popup
#   theme.png                F2 theme source + tokens
#   toast.png                Invalid HEX toast after typing junk
#   contrast-fail.png        Coral on gold, WCAG + APCA FAIL
#   contrast-pair.png        Brand pair #88D9F7 on #0F1114 (also copied to ./screenshot.png)
#   fix.png                  Fix pane on the brand pair
#   web-preview.png          Chromium shot of /tmp/dd_wcag_preview.html
#   palette.png              Palette tab before generate
#   palette-generated.png    Palette tab after Ctrl+G
#
# After adding a new PNG:
#   1. Give it a stable kebab-case name in this catalog
#   2. Capture it in the matching phase below
#   3. Add a <figure> in docs/tutorial.html with alt text that describes the UI, not "screenshot"
#   4. Open tutorial.html in a browser and confirm the figure loads
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMG="${ROOT}/docs/images"
BIN="${ROOT}/target/release/dd_wcag"
SESSION="dd_wcag_docs"
APP_ID="dd-wcag-docs"
TITLE="dd_wcag tutorial capture"
TMUX_CONF="$(mktemp /tmp/dd_wcag_tmux.XXXXXX.conf)"
SHOT_SLEEP="${SHOT_SLEEP:-0.35}"
FOOT_PID=""

mkdir -p "${IMG}"

if [[ ! -x "${BIN}" ]]; then
  echo "Building release binary..."
  cargo build --release --manifest-path "${ROOT}/Cargo.toml"
fi

for cmd in foot tmux grim hyprctl; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing required command: ${cmd}" >&2
    exit 1
  fi
done

cleanup() {
  tmux kill-session -t "${SESSION}" 2>/dev/null || true
  if [[ "${KEEP_WINDOW:-0}" != "1" && -n "${FOOT_PID}" ]]; then
    kill "${FOOT_PID}" 2>/dev/null || true
    wait "${FOOT_PID}" 2>/dev/null || true
  fi
  pkill -f "foot -a ${APP_ID}" 2>/dev/null || true
  rm -f "${TMUX_CONF}"
}
trap cleanup EXIT

cat > "${TMUX_CONF}" <<'EOF'
set -g default-terminal "tmux-256color"
set -as terminal-overrides ",*:RGB"
set -as terminal-features ",*:RGB"
set -g status off
set -g mouse off
set -s escape-time 0
set -g prefix None
unbind C-b
EOF

window_geom() {
  hyprctl clients -j | python3 -c '
import json, sys
want = sys.argv[1]
for c in json.load(sys.stdin):
    if c.get("class") == want and c.get("mapped"):
        x, y = c["at"]
        w, h = c["size"]
        print(f"{x},{y} {w}x{h}")
        sys.exit(0)
sys.exit(1)
' "${APP_ID}"
}

keys() {
  tmux send-keys -t "${SESSION}" "$@"
}

shot() {
  local name="$1"
  sleep "${SHOT_SLEEP}"
  local geom
  geom="$(window_geom)"
  grim -g "${geom}" "${IMG}/${name}.png"
  echo "  wrote docs/images/${name}.png"
}

stop_app() {
  tmux kill-session -t "${SESSION}" 2>/dev/null || true
  if [[ -n "${FOOT_PID}" ]]; then
    kill "${FOOT_PID}" 2>/dev/null || true
    wait "${FOOT_PID}" 2>/dev/null || true
  fi
  pkill -f "foot -a ${APP_ID}" 2>/dev/null || true
  FOOT_PID=""
  sleep 0.2
}

start_app() {
  stop_app
  echo "Opening capture window..."
  COLORTERM=truecolor TERM=tmux-256color \
    foot -a "${APP_ID}" -T "${TITLE}" -w 1600x1000 \
      tmux -f "${TMUX_CONF}" new-session -s "${SESSION}" -- "${BIN}" \
      >/dev/null 2>&1 &
  FOOT_PID=$!

  for _ in $(seq 1 50); do
    if tmux has-session -t "${SESSION}" 2>/dev/null && window_geom >/dev/null 2>&1; then
      break
    fi
    if ! kill -0 "${FOOT_PID}" 2>/dev/null; then
      echo "Capture window exited early." >&2
      exit 1
    fi
    sleep 0.12
  done

  if ! tmux has-session -t "${SESSION}" 2>/dev/null; then
    echo "tmux session ${SESSION} did not start." >&2
    exit 1
  fi
  if ! window_geom >/dev/null 2>&1; then
    echo "Could not find floating window class=${APP_ID}." >&2
    exit 1
  fi
  echo "  geometry: $(window_geom)"
  echo "Waiting for startup toast to expire..."
  sleep 5.4
}

clear_hex() {
  local i
  for i in $(seq 1 12); do
    keys BSpace
  done
}

echo "Registering floating window rule for class=${APP_ID}..."
hyprctl eval "hl.window_rule({ match = { class = \"${APP_ID}\" }, float = true })" >/dev/null

# ---------------------------------------------------------------------------
# Phase 1: Contrast chrome, help, theme, invalid toast, failing pair
# ---------------------------------------------------------------------------
start_app

echo "Capturing Contrast (default)..."
shot contrast

echo "Capturing help..."
keys F1
shot help
keys Escape

echo "Capturing theme debug..."
keys F2
shot theme
keys Escape

echo "Capturing invalid-input toast..."
keys -l 'zz'
keys Tab
shot toast
keys BSpace
keys BSpace

echo "Capturing failing pair..."
clear_hex
keys -l '#F98971'
keys Tab
sleep 0.15
clear_hex
keys -l '#FFCA76'
keys Tab
shot contrast-fail

# ---------------------------------------------------------------------------
# Phase 2: Brand pair, web preview, Fix pane
# ---------------------------------------------------------------------------
start_app

echo "Capturing brand pair..."
clear_hex
keys -l '#88D9F7'
keys Tab
sleep 0.15
clear_hex
keys -l '#0F1114'
keys Tab
shot contrast-pair

if [[ "${SKIP_WEB:-0}" != "1" ]] && command -v chromium >/dev/null 2>&1; then
  echo "Capturing web preview..."
  PREVIEW="/tmp/dd_wcag_preview.html"
  if [[ -f "${PREVIEW}" ]]; then
    chromium --headless=new --disable-gpu --hide-scrollbars \
      --window-size=980,560 \
      --screenshot="${IMG}/web-preview.png" \
      "file://${PREVIEW}" >/dev/null 2>&1 || true
    if [[ -f "${IMG}/web-preview.png" ]]; then
      echo "  wrote docs/images/web-preview.png"
    fi
  fi
fi

echo "Capturing Fix pane..."
keys C-f
shot fix

# ---------------------------------------------------------------------------
# Phase 3: Palette tab
# ---------------------------------------------------------------------------
start_app

echo "Capturing Palette..."
keys Escape
keys 2
shot palette

echo "Capturing generated palette..."
keys C-g
sleep 0.4
shot palette-generated

cp -f "${IMG}/contrast-pair.png" "${ROOT}/screenshot.png"
echo "  updated screenshot.png"

echo "Done. Screenshots are in docs/images/"
echo "Open docs/tutorial.html and confirm every figure still matches the UI."
