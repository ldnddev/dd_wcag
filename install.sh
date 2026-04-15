#!/usr/bin/env bash
set -euo pipefail

REPO_URL=""
BRANCH=""
PREFIX="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.config/ldnddev"
THEME_FILE="dd_wcag_theme.yml"

usage() {
  cat <<'USAGE'
Install dd_wcag from local source or by cloning a repo.

Usage:
  ./install.sh [--repo URL] [--branch NAME] [--prefix DIR] [--config-dir DIR]

Examples:
  ./install.sh
  ./install.sh --repo https://github.com/your-org/dd_wcag.git
  ./install.sh --repo https://github.com/your-org/dd_wcag.git --branch main
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      REPO_URL="${2:-}"
      shift 2
      ;;
    --branch)
      BRANCH="${2:-}"
      shift 2
      ;;
    --prefix)
      PREFIX="${2:-}"
      shift 2
      ;;
    --config-dir)
      CONFIG_DIR="${2:-}"
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

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required. Install Rust first: https://rustup.rs" >&2
  exit 1
fi

WORKDIR=""
cleanup() {
  if [[ -n "${WORKDIR}" && -d "${WORKDIR}" ]]; then
    rm -rf "${WORKDIR}"
  fi
}
trap cleanup EXIT

if [[ -n "${REPO_URL}" ]]; then
  WORKDIR="$(mktemp -d)"
  if [[ -n "${BRANCH}" ]]; then
    git clone --depth 1 --branch "${BRANCH}" "${REPO_URL}" "${WORKDIR}/src"
  else
    git clone --depth 1 "${REPO_URL}" "${WORKDIR}/src"
  fi
  SRC_DIR="${WORKDIR}/src"
else
  SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

echo "Building dd_wcag..."
cargo build --release --manifest-path "${SRC_DIR}/Cargo.toml"

mkdir -p "${PREFIX}"
install -m 0755 "${SRC_DIR}/target/release/dd_wcag" "${PREFIX}/dd_wcag"

mkdir -p "${CONFIG_DIR}"
if [[ ! -f "${CONFIG_DIR}/${THEME_FILE}" ]]; then
  install -m 0644 "${SRC_DIR}/${THEME_FILE}" "${CONFIG_DIR}/${THEME_FILE}"
  echo "Installed default theme to ${CONFIG_DIR}/${THEME_FILE}"
else
  echo "Theme file already exists at ${CONFIG_DIR}/${THEME_FILE} (left unchanged)."
fi

echo ""
echo "Installed: ${PREFIX}/dd_wcag"
echo "Theme path: ${CONFIG_DIR}/${THEME_FILE}"
echo ""
echo "Ensure ${PREFIX} is in PATH, then run:"
echo "  dd_wcag"
