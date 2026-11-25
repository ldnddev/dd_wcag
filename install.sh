#!/usr/bin/env bash
# Install dd_wcag from a GitHub Release (preferred) or from source.
#
#   curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --both
#   curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- -uninstall
#
# INSTALL_GUI=1 with no --tui/--gui/--both flag installs both.
# With no terminal to answer, install selects the terminal and uninstall removes both.
set -euo pipefail

APP_NAME="dd_wcag"
GUI_NAME="dd_wcag-gui"
GUI_PACKAGE="dd_wcag_gui"
APP_ID="dev.ldnddev.dd_wcag"
GH_OWNER="ldnddev"
GH_REPO="dd_wcag"
DEFAULT_GIT_URL="https://github.com/${GH_OWNER}/${GH_REPO}.git"
DEFAULT_BRANCH="master"
THEME_FILE="dd_wcag_theme.yml"

REPO_URL=""
BRANCH=""
VERSION="${DD_WCAG_VERSION:-}"
PREFIX="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/ldnddev"
UNINSTALL=0
FROM_SOURCE=0
PREBUILT=0
PRINT_TARGET=0
DRY_RUN=0
WORKDIR=""
TARGET=""
EXPLICIT=""
TUI_EXTRACTED=""
GUI_EXTRACTED=""

cleanup() {
  if [[ -n "${WORKDIR}" && -d "${WORKDIR}" ]]; then
    rm -rf "${WORKDIR}"
  fi
  WORKDIR=""
}
trap cleanup EXIT

# ldnddev wordmark. Printed only when stdout is a terminal.
print_mark() {
  [[ -t 1 ]] || return 0
  printf '\033[40;97m'
  cat <<'EOF'

█▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀█
█  ██                ██                         ██  █
█  ██                ██                         ██  █
█  ██         ▄▄▄▄▄▄ ██     ▄▄▄▄▄▄       ▄▄▄▄▄▄ ██  █
█  ██       ▄██▀▀▀▀▀███  ▄██████████▄  ▄██▀▀▀▀▀███  █
█  ██      ██▀       ██  ██        ██ ██▀       ██  █
█  ██      ██        ██  ██        ██ ██        ██  █
█  ██      ▀█▄       ██  ██        ██ ▀█▄       ██  █
█  ▀█▄▄▄▄▄  ▀██▄▄▄▄██▀   ██        ██  ▀██▄▄▄▄██▀   █
█   ▀▀▀▀▀▀    ▀▀▀▀▀▀     ▀▀        ▀▀    ▀▀▀▀▀▀     █
█                                                   █
█                     ██                            █
█                     ██                            █
█              ▄▄▄▄▄▄ ██     ▄▄▄▄▄▄    ▄▄      ▄▄▄  █
█            ▄██▀▀▀▀▀███   ██▀▀▀▀███   ██     ▄██   █
█           ██▀       ██  ██    ██▀ ▄▄ ██    ▄██    █
█           ██        ██  ██  ▄██   ██ ██    ██     █
█           ▀█▄       ██  ██▄██▀   ▄█▀ ██  ▄██      █
█  ▄▄▄▄▄▄▄▄  ▀██▄▄▄▄██▀    ▀███▄▄▄██▀  ██▄██▀       █
█  ▀▀▀▀▀▀▀▀    ▀▀▀▀▀▀        ▀▀▀▀▀▀    ▀▀▀▀         █
▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
EOF
  printf '\033[0m\n'
}

usage() {
  print_mark
  cat <<USAGE
Install or uninstall ${APP_NAME}.

Usage:
  install.sh [--tui | --gui | --both] [options]
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- [options]

  ./install.sh
      ask: terminal, desktop window, or both
  ./install.sh --tui
      install ${APP_NAME}
  ./install.sh --gui
      install ${GUI_NAME}
  ./install.sh --both
      install both
  ./install.sh -uninstall
      ask which of those to remove

With no terminal to answer, install selects the terminal and uninstall removes
both. INSTALL_GUI=1 with no target flag installs both. An explicit flag wins.

Options:
  --tui               Install the terminal only
  --gui               Install the desktop window only
  --both              Install the terminal and the window
  --version TAG       GitHub Release tag (default: latest). Also DD_WCAG_VERSION.
  --from-source       Build with cargo instead of downloading a release package
  --prebuilt          Download a release package even from a local checkout
  --repo URL          Clone this git URL and build from source
  --branch NAME       Branch to clone with --repo (default: ${DEFAULT_BRANCH})
  --prefix DIR        Binary install directory (default: ~/.local/bin)
  --config-dir DIR    Theme install directory (default: ~/.config/ldnddev)
  --print-target      Print the detected package target and exit
  --dry-run           Print what would be installed, without installing
  -uninstall          Remove the chosen binary and launcher. Leaves the theme file in place
  -h, --help          Show this help

Examples:
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --both
  ./install.sh --tui --from-source
  ./install.sh -uninstall --gui

Install behavior:
  - Detects OS and CPU (Linux gnu/musl, macOS Intel/Apple Silicon)
  - Downloads the matching GitHub Release tarball when one exists
  - Falls back to a cargo source build if no package is published for this machine
  - Installs binaries to ~/.local/bin by default
  - Installs the theme to ~/.config/ldnddev/${THEME_FILE} only if that file is missing
  - A window-only install does not replace the theme file
  - On Linux, writes a launcher for each binary that is installed
  - The window needs Vulkan and Wayland or X11

Uninstall behavior:
  - Removes only the chosen binary and its .desktop launcher
  - Leaves ${CONFIG_DIR}/${THEME_FILE} and ~/.config/ldnddev in place
  Pass the same --prefix and XDG_DATA_HOME used at install time.
USAGE
}

die() {
  echo "error: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

need_arg() {
  local flag="$1"
  local value="${2:-}"
  [[ -n "${value}" && "${value}" != -* ]] || die "${flag} requires a value"
}

local_src_dir() {
  # curl | bash leaves BASH_SOURCE unset. Accessing BASH_SOURCE[0] with
  # set -u then fails with "unbound variable" / "cd: null directory".
  local script_path="" dir
  set +u
  script_path="${BASH_SOURCE[0]}"
  set -u
  if [[ -z "${script_path}" || "${script_path}" == "bash" || "${script_path}" == "-" ]]; then
    return 1
  fi
  if [[ ! -f "${script_path}" ]]; then
    return 1
  fi
  dir="$(cd "$(dirname "${script_path}")" && pwd)" || return 1
  if [[ -f "${dir}/Cargo.toml" && -f "${dir}/${THEME_FILE}" ]]; then
    printf '%s\n' "${dir}"
    return 0
  fi
  return 1
}

detect_target() {
  local os arch libc
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "${arch}" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) die "unsupported CPU architecture: ${arch}" ;;
  esac

  case "${os}" in
    linux)
      libc="gnu"
      if [[ -f /etc/alpine-release ]]; then
        libc="musl"
      elif command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; then
        libc="musl"
      fi
      printf '%s\n' "${arch}-unknown-linux-${libc}"
      ;;
    darwin)
      printf '%s\n' "${arch}-apple-darwin"
      ;;
    mingw*|msys*|cygwin*)
      die "Windows is not supported by this installer. Use WSL, or build from source with cargo."
      ;;
    *)
      die "unsupported OS: $(uname -s)"
      ;;
  esac
}

download() {
  local url="$1"
  local dest="$2"
  if command -v curl >/dev/null 2>&1; then
    if [[ -n "${DD_WCAG_ASSET_BASE:-}" ]]; then
      curl --retry 3 --retry-delay 1 -fL --silent --show-error -o "${dest}" "${url}" || return 1
    else
      curl --proto '=https' --tlsv1.2 --retry 3 --retry-delay 1 -fL --silent --show-error -o "${dest}" "${url}" || return 1
    fi
  elif command -v wget >/dev/null 2>&1; then
    if [[ -n "${DD_WCAG_ASSET_BASE:-}" ]]; then
      wget -q -O "${dest}" "${url}" || return 1
    else
      wget --https-only -q -O "${dest}" "${url}" || return 1
    fi
  else
    die "curl or wget is required to download packages"
  fi
}

file_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    printf '\n'
  fi
}

normalize_tag() {
  local tag="$1"
  if [[ "${tag}" == latest ]]; then
    printf 'latest\n'
  elif [[ "${tag}" == v* ]]; then
    printf '%s\n' "${tag}"
  else
    printf 'v%s\n' "${tag}"
  fi
}

asset_base_url() {
  local tag="$1"
  if [[ -n "${DD_WCAG_ASSET_BASE:-}" ]]; then
    printf '%s\n' "${DD_WCAG_ASSET_BASE%/}"
    return 0
  fi
  if [[ -z "${tag}" || "${tag}" == latest ]]; then
    printf 'https://github.com/%s/%s/releases/latest/download\n' "${GH_OWNER}" "${GH_REPO}"
  else
    printf 'https://github.com/%s/%s/releases/download/%s\n' "${GH_OWNER}" "${GH_REPO}" "${tag}"
  fi
}

install_named_binary() {
  local src="$1"
  local name="$2"
  [[ -f "${src}" ]] || die "binary not found: ${src}"
  mkdir -p "${PREFIX}"
  install -m 0755 "${src}" "${PREFIX}/${name}"
}

install_theme_from() {
  local src_dir="$1"
  local src="${src_dir}/${THEME_FILE}"
  mkdir -p "${CONFIG_DIR}"

  if [[ ! -f "${src}" ]]; then
    if [[ -f "${CONFIG_DIR}/${THEME_FILE}" ]]; then
      echo "Theme file missing from package; leaving existing ${CONFIG_DIR}/${THEME_FILE} unchanged."
      return 0
    fi
    echo "Theme file missing from package; downloading default theme..."
    download "https://raw.githubusercontent.com/${GH_OWNER}/${GH_REPO}/${DEFAULT_BRANCH}/${THEME_FILE}" "${CONFIG_DIR}/${THEME_FILE}"
    chmod 0644 "${CONFIG_DIR}/${THEME_FILE}"
    echo "Installed default theme to ${CONFIG_DIR}/${THEME_FILE}"
    return 0
  fi

  if [[ ! -f "${CONFIG_DIR}/${THEME_FILE}" ]]; then
    install -m 0644 "${src}" "${CONFIG_DIR}/${THEME_FILE}"
    echo "Installed default theme to ${CONFIG_DIR}/${THEME_FILE}"
  else
    echo "Theme file already exists at ${CONFIG_DIR}/${THEME_FILE} (left unchanged)."
  fi
}

on_linux() {
  [[ "$(uname -s)" == "Linux" ]]
}

# Desktop menus search XDG_DATA_HOME, then ~/.local/share. A prefix under
# $HOME still publishes the launcher where the session looks. A system
# prefix such as /usr/local/bin uses the sibling share/applications dir.
apps_dir() {
  if [[ -n "${XDG_DATA_HOME:-}" ]]; then
    case "${PREFIX}" in
      "${HOME}" | "${HOME}"/*)
        printf '%s/applications\n' "${XDG_DATA_HOME}"
        return
        ;;
    esac
  fi
  case "${PREFIX}" in
    "${HOME}" | "${HOME}"/*)
      printf '%s/.local/share/applications\n' "${HOME}"
      ;;
    *)
      printf '%s/share/applications\n' "$(dirname "${PREFIX}")"
      ;;
  esac
}

# Quote one Exec argument. A literal % is %%.
desktop_quote() {
  local s="$1"
  [[ "${s}" != *$'\n'* ]] || die "desktop path contains a newline"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//\`/\\\`}"
  s="${s//\$/\\\$}"
  s="${s//%/%%}"
  printf '"%s"' "${s}"
}

refresh_desktop_database() {
  local dir="$1"
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${dir}" >/dev/null 2>&1 || true
  fi
}

write_desktop() {
  local dest="$1"
  local name="$2"
  local comment="$3"
  local bin="$4"
  local terminal="$5"
  local startup="$6"
  local wmclass="${7:-}"
  local icon="${8:-utilities-terminal}"
  local exec_q
  exec_q="$(desktop_quote "${bin}")"
  {
    printf '%s\n' "[Desktop Entry]"
    printf '%s\n' "Type=Application"
    printf '%s\n' "Version=1.0"
    printf '%s\n' "Name=${name}"
    printf '%s\n' "GenericName=Contrast checker"
    printf '%s\n' "Comment=${comment}"
    printf '%s\n' "Exec=${exec_q}"
    printf '%s\n' "TryExec=${bin}"
    printf '%s\n' "Terminal=${terminal}"
    printf '%s\n' "Icon=${icon}"
    printf '%s\n' "Categories=Utility;"
    printf '%s\n' "Keywords=wcag;apca;contrast;accessibility;color;"
    printf '%s\n' "StartupNotify=${startup}"
    if [[ -n "${wmclass}" ]]; then
      printf '%s\n' "StartupWMClass=${wmclass}"
    fi
  } > "${dest}"
  chmod 0644 "${dest}"
}

# Write a launcher for every binary that is already installed, so installing
# the terminal does not drop an existing window launcher.
install_launchers() {
  local dir
  if ! on_linux; then
    echo "Desktop launchers are written on Linux. Skipped on $(uname -s)."
    return 0
  fi
  dir="$(apps_dir)"
  mkdir -p "${dir}"
  if [[ -x "${PREFIX}/${APP_NAME}" ]]; then
    write_desktop \
      "${dir}/${APP_NAME}.desktop" \
      "${APP_NAME}" \
      "Check WCAG and APCA contrast in the terminal" \
      "${PREFIX}/${APP_NAME}" \
      "true" \
      "false"
    echo "Launcher: ${dir}/${APP_NAME}.desktop"
  fi
  if [[ -x "${PREFIX}/${GUI_NAME}" ]]; then
    write_desktop \
      "${dir}/${GUI_NAME}.desktop" \
      "${APP_NAME} GUI" \
      "Check WCAG and APCA contrast in a window" \
      "${PREFIX}/${GUI_NAME}" \
      "false" \
      "true" \
      "${APP_ID}" \
      "applications-graphics"
    echo "Launcher: ${dir}/${GUI_NAME}.desktop"
  fi
  refresh_desktop_database "${dir}"
}

print_launcher_plan() {
  if ! on_linux; then
    echo "launcher: skipped ($(uname -s))"
    return 0
  fi
  case "${TARGET}" in
    tui|both) echo "launcher: $(apps_dir)/${APP_NAME}.desktop" ;;
  esac
  case "${TARGET}" in
    gui|both) echo "launcher: $(apps_dir)/${GUI_NAME}.desktop" ;;
  esac
}

print_theme_plan() {
  if [[ "${TARGET}" == "gui" ]]; then
    echo "theme: left unchanged (window-only install)"
  else
    echo "theme: ${CONFIG_DIR}/${THEME_FILE} (only if missing)"
  fi
}

note_window_runtime() {
  if [[ "${TARGET}" != "tui" ]]; then
    echo "The window needs Vulkan and Wayland or X11."
  fi
}

print_done() {
  echo ""
  case "${TARGET}" in
    tui|both) echo "Installed: ${PREFIX}/${APP_NAME}" ;;
  esac
  case "${TARGET}" in
    gui|both) echo "Installed: ${PREFIX}/${GUI_NAME}" ;;
  esac
  if [[ "${TARGET}" == "gui" ]]; then
    echo "Theme file was not changed."
  else
    echo "Theme path: ${CONFIG_DIR}/${THEME_FILE}"
  fi
  if on_linux; then
    echo "App menu directory: $(apps_dir)"
  fi
  note_window_runtime
  echo ""
  case ":${PATH}:" in
    *":${PREFIX}:"*) ;;
    *)
      echo "Warning: ${PREFIX} is not on PATH. Add it, then run the installed program."
      echo ""
      echo "For bash, add this to ~/.bashrc and reload the shell:"
      echo "  export PATH=\"${PREFIX}:\$PATH\""
      return 0
      ;;
  esac
  echo "Ensure ${PREFIX} is in PATH, then run the installed program."
}

verify_checksum() {
  local asset="$1"
  local archive="$2"
  local sha_url="$3"
  local sha_file expected actual
  sha_file="${WORKDIR}/${asset}.sha256"
  if download "${sha_url}" "${sha_file}" 2>/dev/null; then
    expected="$(awk '{print $1}' "${sha_file}")"
    actual="$(file_sha256 "${archive}")"
    if [[ -z "${actual}" ]]; then
      echo "warning: sha256sum/shasum not found; skipped checksum verification." >&2
    elif [[ "${expected}" != "${actual}" ]]; then
      die "checksum mismatch for ${asset} (expected ${expected}, got ${actual})"
    else
      echo "Checksum OK."
    fi
  else
    echo "warning: no checksum file at ${sha_url}; skipped verification." >&2
  fi
}

fetch_release_asset() {
  local name="$1"
  local triple="$2"
  local asset url sha_url archive extracted
  asset="${name}-${triple}.tar.gz"
  url="$(asset_base_url "${VERSION}")/${asset}"
  sha_url="${url}.sha256"
  echo "Downloading ${asset}..."
  archive="${WORKDIR}/${asset}"
  if ! download "${url}" "${archive}"; then
    return 1
  fi
  verify_checksum "${asset}" "${archive}" "${sha_url}"
  mkdir -p "${WORKDIR}/extract-${name}"
  tar -xzf "${archive}" -C "${WORKDIR}/extract-${name}"
  extracted="$(find "${WORKDIR}/extract-${name}" -type f -name "${name}" -print -quit)"
  [[ -n "${extracted}" ]] || die "archive did not contain ${name}"
  if [[ "${name}" == "${APP_NAME}" ]]; then
    TUI_EXTRACTED="${extracted}"
  else
    GUI_EXTRACTED="${extracted}"
  fi
}

install_from_prebuilt() {
  local triple tui_url gui_url theme_dir
  triple="$(detect_target)"
  tui_url="$(asset_base_url "${VERSION}")/${APP_NAME}-${triple}.tar.gz"
  gui_url="$(asset_base_url "${VERSION}")/${GUI_NAME}-${triple}.tar.gz"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "target: ${triple}"
    case "${TARGET}" in
      tui|both)
        echo "package: ${tui_url}"
        echo "checksum: ${tui_url}.sha256"
        ;;
    esac
    case "${TARGET}" in
      gui|both)
        echo "package: ${gui_url}"
        echo "checksum: ${gui_url}.sha256"
        ;;
    esac
    print_launcher_plan
    print_theme_plan
    note_window_runtime
    return 0
  fi

  echo "Detected platform: ${triple}"
  WORKDIR="$(mktemp -d)"
  TUI_EXTRACTED=""
  GUI_EXTRACTED=""
  case "${TARGET}" in
    tui)
      fetch_release_asset "${APP_NAME}" "${triple}" || { cleanup; return 1; }
      ;;
    gui)
      fetch_release_asset "${GUI_NAME}" "${triple}" || { cleanup; return 1; }
      ;;
    both)
      fetch_release_asset "${APP_NAME}" "${triple}" || { cleanup; return 1; }
      fetch_release_asset "${GUI_NAME}" "${triple}" || { cleanup; return 1; }
      ;;
  esac

  if [[ -n "${TUI_EXTRACTED}" ]]; then
    install_named_binary "${TUI_EXTRACTED}" "${APP_NAME}"
  fi
  if [[ -n "${GUI_EXTRACTED}" ]]; then
    install_named_binary "${GUI_EXTRACTED}" "${GUI_NAME}"
  fi
  if [[ "${TARGET}" == "gui" ]]; then
    echo "Window install leaves ${CONFIG_DIR}/${THEME_FILE} unchanged."
  else
    theme_dir="$(dirname "${TUI_EXTRACTED}")"
    install_theme_from "${theme_dir}"
  fi
  install_launchers
  print_done
  cleanup
  return 0
}

install_from_source() {
  local src_dir
  if [[ "${DRY_RUN}" -eq 1 ]]; then
    if [[ -n "${REPO_URL}" ]]; then
      echo "Would clone ${REPO_URL}${BRANCH:+ (branch ${BRANCH})} and build from source"
    elif src_dir="$(local_src_dir)"; then
      echo "Would build from local checkout: ${src_dir}"
    else
      echo "Would clone ${DEFAULT_GIT_URL} (branch ${BRANCH:-${DEFAULT_BRANCH}}) and build from source"
    fi
    case "${TARGET}" in
      tui|both) echo "cargo: cargo build --release -p ${APP_NAME}" ;;
    esac
    case "${TARGET}" in
      gui|both) echo "cargo: cargo build --release -p ${GUI_PACKAGE}" ;;
    esac
    print_launcher_plan
    print_theme_plan
    note_window_runtime
    return 0
  fi

  need_cmd cargo

  if [[ -n "${REPO_URL}" ]]; then
    need_cmd git
    WORKDIR="$(mktemp -d)"
    echo "Cloning ${REPO_URL}..."
    if [[ -n "${BRANCH}" ]]; then
      git clone --depth 1 --branch "${BRANCH}" "${REPO_URL}" "${WORKDIR}/src"
    else
      git clone --depth 1 "${REPO_URL}" "${WORKDIR}/src"
    fi
    src_dir="${WORKDIR}/src"
  elif src_dir="$(local_src_dir)"; then
    echo "Building from local checkout: ${src_dir}"
  else
    need_cmd git
    WORKDIR="$(mktemp -d)"
    echo "Cloning ${DEFAULT_GIT_URL}..."
    git clone --depth 1 --branch "${BRANCH:-${DEFAULT_BRANCH}}" "${DEFAULT_GIT_URL}" "${WORKDIR}/src"
    src_dir="${WORKDIR}/src"
  fi

  case "${TARGET}" in
    tui|both)
      echo "Building ${APP_NAME}..."
      cargo build --release --manifest-path "${src_dir}/Cargo.toml" -p "${APP_NAME}"
      install_named_binary "${src_dir}/target/release/${APP_NAME}" "${APP_NAME}"
      ;;
  esac
  case "${TARGET}" in
    gui|both)
      echo "Building ${GUI_NAME}..."
      cargo build --release --manifest-path "${src_dir}/Cargo.toml" -p "${GUI_PACKAGE}"
      install_named_binary "${src_dir}/target/release/${GUI_NAME}" "${GUI_NAME}"
      ;;
  esac
  if [[ "${TARGET}" == "gui" ]]; then
    echo "Window install leaves ${CONFIG_DIR}/${THEME_FILE} unchanged."
  else
    install_theme_from "${src_dir}"
  fi
  install_launchers
  print_done
}

remove_path() {
  local path="$1"
  if [[ -e "${path}" || -L "${path}" ]]; then
    rm -f -- "${path}"
    echo "Removed ${path}"
  else
    echo "Not found: ${path}"
  fi
}

uninstall_installed() {
  local apps=""
  case "${TARGET}" in
    tui|both)
      remove_path "${PREFIX}/${APP_NAME}"
      ;;
  esac
  case "${TARGET}" in
    gui|both)
      remove_path "${PREFIX}/${GUI_NAME}"
      ;;
  esac
  if on_linux; then
    apps="$(apps_dir)"
    case "${TARGET}" in
      tui|both) remove_path "${apps}/${APP_NAME}.desktop" ;;
    esac
    case "${TARGET}" in
      gui|both) remove_path "${apps}/${GUI_NAME}.desktop" ;;
    esac
    refresh_desktop_database "${apps}"
  fi
  echo "Left theme in place: ${CONFIG_DIR}/${THEME_FILE}"
  echo "Left ~/.config/ldnddev in place."
  echo ""
  echo "Uninstall complete."
}

# The device node is mode rw-rw-rw, so -r/-w stay true with no controlling
# terminal. Opening it is the check that matters.
have_tty() {
  { : <>/dev/tty; } 2>/dev/null
}

# Read 1, 2, or 3 from the terminal. stdout is tui, gui, or both.
# $1 is the verb. $2 is the default number.
# Prompts go to /dev/tty. The caller captures stdout.
ask_choice() {
  local verb="$1"
  local default="$2"
  local answer choice
  while true; do
    {
      printf '%s which program?\n' "${verb}"
      printf '%s\n' "  1) terminal (${APP_NAME})"
      printf '%s\n' "  2) desktop window (${GUI_NAME})"
      printf '%s\n' "  3) both"
      printf 'Choice [%s]: ' "${default}"
    } >/dev/tty 2>/dev/null || return 1
    if ! IFS= read -r answer </dev/tty 2>/dev/null; then
      printf '\n' >/dev/tty 2>/dev/null || true
      answer="${default}"
    fi
    choice=$(printf '%s' "${answer}" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')
    case "${choice}" in
      '') choice="${default}" ;;
    esac
    case "${choice}" in
      1|tui|terminal) printf '%s\n' tui; return 0 ;;
      2|gui|window) printf '%s\n' gui; return 0 ;;
      3|both) printf '%s\n' both; return 0 ;;
      *) printf '%s\n' "Enter 1, 2, or 3." >/dev/tty 2>/dev/null || return 1 ;;
    esac
  done
}

# stdout is only the word tui, gui, or both. Notices go to stderr.
choose_target() {
  local action="$1"
  local explicit="$2"
  if [[ -n "${explicit}" ]]; then
    printf '%s\n' "${explicit}"
    return 0
  fi
  if have_tty; then
    if [[ "${action}" == "uninstall" ]]; then
      ask_choice "Remove" 3 && return 0
    else
      ask_choice "Install" 1 && return 0
    fi
  fi
  if [[ "${action}" == "uninstall" ]]; then
    echo "No terminal for a prompt. Removing both programs." >&2
    printf '%s\n' both
  else
    echo "No terminal for a prompt. Installing the terminal app." >&2
    printf '%s\n' tui
  fi
}

set_explicit() {
  local name="$1"
  if [[ -n "${EXPLICIT}" ]]; then
    die "pass only one of --tui, --gui, or --both"
  fi
  EXPLICIT="${name}"
}

main() {
  local action="install"
  local triple

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --tui)
        set_explicit tui
        shift
        ;;
      --gui)
        set_explicit gui
        shift
        ;;
      --both)
        set_explicit both
        shift
        ;;
      -uninstall|--uninstall)
        UNINSTALL=1
        shift
        ;;
      --from-source)
        FROM_SOURCE=1
        shift
        ;;
      --prebuilt)
        PREBUILT=1
        shift
        ;;
      --print-target)
        PRINT_TARGET=1
        shift
        ;;
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      --repo)
        need_arg "$1" "${2-}"
        REPO_URL="$2"
        shift 2
        ;;
      --branch)
        need_arg "$1" "${2-}"
        BRANCH="$2"
        shift 2
        ;;
      --version)
        need_arg "$1" "${2-}"
        VERSION="$2"
        shift 2
        ;;
      --prefix)
        need_arg "$1" "${2-}"
        PREFIX="$2"
        shift 2
        ;;
      --config-dir)
        need_arg "$1" "${2-}"
        CONFIG_DIR="$2"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown argument: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if [[ -n "${VERSION}" ]]; then
    VERSION="$(normalize_tag "${VERSION}")"
  fi

  if [[ "${PRINT_TARGET}" -eq 1 ]]; then
    detect_target
    exit 0
  fi

  if [[ "${UNINSTALL}" -eq 1 ]]; then
    if [[ -n "${REPO_URL}" || -n "${BRANCH}" || -n "${VERSION}" || "${FROM_SOURCE}" -eq 1 || "${PREBUILT}" -eq 1 ]]; then
      die "--repo, --branch, --version, --from-source, and --prebuilt are not valid with -uninstall."
    fi
    action="uninstall"
  fi

  if [[ "${FROM_SOURCE}" -eq 1 && "${PREBUILT}" -eq 1 ]]; then
    die "--from-source and --prebuilt cannot be used together."
  fi
  if [[ -n "${REPO_URL}" && "${PREBUILT}" -eq 1 ]]; then
    die "--repo and --prebuilt cannot be used together."
  fi
  if [[ -n "${VERSION}" && "${FROM_SOURCE}" -eq 1 ]]; then
    die "--version is not valid with --from-source."
  fi
  if [[ -n "${BRANCH}" && -z "${REPO_URL}" && "${FROM_SOURCE}" -eq 0 ]]; then
    die "--branch requires --repo."
  fi

  print_mark

  if [[ -z "${EXPLICIT}" && "${INSTALL_GUI:-}" == "1" ]]; then
    EXPLICIT="both"
  fi
  TARGET="$(choose_target "${action}" "${EXPLICIT}")"

  if [[ "${action}" == "uninstall" ]]; then
    uninstall_installed
    exit 0
  fi

  if [[ "${FROM_SOURCE}" -eq 1 || -n "${REPO_URL}" ]]; then
    install_from_source
    exit 0
  fi

  if [[ "${PREBUILT}" -eq 0 ]] && local_src_dir >/dev/null; then
    install_from_source
    exit 0
  fi

  if install_from_prebuilt; then
    exit 0
  fi

  triple="$(detect_target)"
  echo ""
  echo "No prebuilt package for ${triple}."
  case "${TARGET}" in
    tui|both)
      echo "Tried: $(asset_base_url "${VERSION}")/${APP_NAME}-${triple}.tar.gz"
      ;;
  esac
  case "${TARGET}" in
    gui|both)
      echo "Tried: $(asset_base_url "${VERSION}")/${GUI_NAME}-${triple}.tar.gz"
      ;;
  esac
  if [[ "${PREBUILT}" -eq 1 ]]; then
    die "no matching GitHub Release package for this machine."
  fi
  if command -v cargo >/dev/null 2>&1; then
    echo "Falling back to a source build..."
    echo ""
    install_from_source
  else
    die "no matching GitHub Release package, and cargo is not installed. Install Rust from https://rustup.rs or download a package from https://github.com/${GH_OWNER}/${GH_REPO}/releases"
  fi
}

# `curl | bash` reads this file from stdin. BASH_SOURCE is unset then, and
# `set -u` aborts on a bare ${BASH_SOURCE[0]}. An empty source means the
# file is being executed, not sourced.
if [[ -z "${BASH_SOURCE[0]:-}" || "${BASH_SOURCE[0]:-}" == "$0" ]]; then
  main "$@"
fi
