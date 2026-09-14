#!/usr/bin/env bash
# Install dd_wcag from a GitHub Release (preferred) or from source.
#
#   curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- -uninstall
set -euo pipefail

APP_NAME="dd_wcag"
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

cleanup() {
  if [[ -n "${WORKDIR}" && -d "${WORKDIR}" ]]; then
    rm -rf "${WORKDIR}"
  fi
  WORKDIR=""
}
trap cleanup EXIT

usage() {
  cat <<USAGE
Install or uninstall ${APP_NAME}.

Usage:
  install.sh [options]
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- [options]

Options:
  --version TAG       GitHub Release tag (default: latest). Also DD_WCAG_VERSION.
  --from-source       Build with cargo instead of downloading a release package
  --prebuilt          Download a release package even from a local checkout
  --repo URL          Clone this git URL and build from source
  --branch NAME       Branch to clone with --repo (default: ${DEFAULT_BRANCH})
  --prefix DIR        Binary install directory (default: ~/.local/bin)
  --config-dir DIR    Theme install directory (default: ~/.config/ldnddev)
  --print-target      Print the detected package target and exit
  --dry-run           Print what would be installed, without installing
  -uninstall          Remove the installed binary and theme
  -h, --help          Show this help

Examples:
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/ldnddev/dd_wcag/master/install.sh | bash -s -- --version v0.5.0
  ./install.sh
  ./install.sh --from-source
  ./install.sh -uninstall

Install behavior:
  - Detects OS and CPU (Linux gnu/musl, macOS Intel/Apple Silicon)
  - Downloads the matching GitHub Release tarball when one exists
  - Falls back to a cargo source build if no package is published for this machine
  - Installs the binary to ~/.local/bin/${APP_NAME} by default
  - Installs the theme to ~/.config/ldnddev/${THEME_FILE} only if that file is missing

Uninstall behavior:
  - Removes the installed ${APP_NAME} binary
  - Removes ~/.config/ldnddev/${THEME_FILE}
  - Removes ~/.config/ldnddev only if it is empty
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

install_binary() {
  local src="$1"
  [[ -f "${src}" ]] || die "binary not found: ${src}"
  mkdir -p "${PREFIX}"
  install -m 0755 "${src}" "${PREFIX}/${APP_NAME}"
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

print_done() {
  echo ""
  echo "Installed: ${PREFIX}/${APP_NAME}"
  echo "Theme path: ${CONFIG_DIR}/${THEME_FILE}"
  echo ""
  case ":${PATH}:" in
    *":${PREFIX}:"*) ;;
    *)
      echo "Warning: ${PREFIX} is not on PATH. Add it, then run:"
      echo "  ${APP_NAME}"
      echo ""
      echo "For bash, add this to ~/.bashrc and reload the shell:"
      echo "  export PATH=\"${PREFIX}:\$PATH\""
      return 0
      ;;
  esac
  echo "Ensure ${PREFIX} is in PATH, then run:"
  echo "  ${APP_NAME}"
}

install_from_prebuilt() {
  local target asset url sha_url archive sha_file expected actual extracted
  target="$(detect_target)"
  asset="${APP_NAME}-${target}.tar.gz"
  url="$(asset_base_url "${VERSION}")/${asset}"
  sha_url="${url}.sha256"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "target: ${target}"
    echo "package: ${url}"
    echo "checksum: ${sha_url}"
    return 0
  fi

  echo "Detected platform: ${target}"
  echo "Downloading ${asset}..."

  WORKDIR="$(mktemp -d)"
  archive="${WORKDIR}/${asset}"
  sha_file="${WORKDIR}/${asset}.sha256"
  if ! download "${url}" "${archive}"; then
    cleanup
    return 1
  fi

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

  tar -xzf "${archive}" -C "${WORKDIR}"
  extracted="$(find "${WORKDIR}" -type f -name "${APP_NAME}" -print -quit)"
  [[ -n "${extracted}" ]] || die "archive did not contain ${APP_NAME}"

  install_binary "${extracted}"
  install_theme_from "$(dirname "${extracted}")"
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

  echo "Building ${APP_NAME}..."
  cargo build --release --manifest-path "${src_dir}/Cargo.toml"

  install_binary "${src_dir}/target/release/${APP_NAME}"
  install_theme_from "${src_dir}"
  print_done
}

while [[ $# -gt 0 ]]; do
  case "$1" in
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

  BIN_PATH="${PREFIX}/${APP_NAME}"
  THEME_PATH="${CONFIG_DIR}/${THEME_FILE}"

  if [[ -f "${BIN_PATH}" ]]; then
    rm -f "${BIN_PATH}"
    echo "Removed binary: ${BIN_PATH}"
  else
    echo "Binary not found: ${BIN_PATH}"
  fi

  if [[ -f "${THEME_PATH}" ]]; then
    rm -f "${THEME_PATH}"
    echo "Removed theme: ${THEME_PATH}"
  else
    echo "Theme not found: ${THEME_PATH}"
  fi

  if [[ -d "${CONFIG_DIR}" ]]; then
    if rmdir "${CONFIG_DIR}" 2>/dev/null; then
      echo "Removed empty config directory: ${CONFIG_DIR}"
    else
      echo "Config directory not empty, left in place: ${CONFIG_DIR}"
    fi
  fi

  echo ""
  echo "Uninstall complete."
  exit 0
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

echo ""
echo "No prebuilt package for $(detect_target)."
echo "Tried: $(asset_base_url "${VERSION}")/${APP_NAME}-$(detect_target).tar.gz"
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
