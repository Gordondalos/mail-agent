#!/usr/bin/env bash
# Mail Agent (Gmail Tray Notifier) — dependency checker/installer for Linux
#
# Usage from repo root:
#   bash scripts/setup.sh [--auto] [--dev|--build]
#
# Flags:
#   -a, --auto   Try to install missing parts automatically using the system package manager.
#   -d, --dev    After checks pass, run `cargo tauri dev`.
#   -b, --build  After checks pass, run `cargo tauri build`.
#
# This script detects distro (Debian/Ubuntu, Fedora, Arch) and installs prerequisites:
# - System libs for Tauri/WebKitGTK and tray
# - Node.js + npm
# - Rust (rustup + cargo)
# - Tauri CLI (cargo-tauri or @tauri-apps/cli)
set -euo pipefail

AUTO=0
DO_DEV=0
DO_BUILD=0

for arg in "$@"; do
  case "$arg" in
    -a|--auto) AUTO=1 ;;
    -d|--dev) DO_DEV=1 ;;
    -b|--build) DO_BUILD=1 ;;
    *) echo "Unknown arg: $arg" >&2; exit 1 ;;
  esac
done

say() { echo -e "\n=== $* ==="; }
info() { echo "$*"; }
warn() { echo "$*" >&2; }
run() { echo "> $*"; "$@"; }

have_cmd() { command -v "$1" >/dev/null 2>&1; }

# Detect package manager / distro
PKG=""
if have_cmd apt; then PKG=apt; fi
if have_cmd dnf; then PKG=dnf; fi
if have_cmd pacman; then PKG=pacman; fi

say "Checking prerequisites"

# Node and npm
NODE_OK=0; NPM_OK=0
if have_cmd node; then info "Node: $(node -v)"; NODE_OK=1; else warn "Node: not found"; fi
if have_cmd npm; then info "npm:  $(npm -v)"; NPM_OK=1; else warn "npm:  not found"; fi

install_node() {
  case "$PKG" in
    apt)
      run sudo apt update
      run sudo apt install -y nodejs npm
      ;;
    dnf)
      run sudo dnf install -y nodejs npm
      ;;
    pacman)
      run sudo pacman -Sy --needed --noconfirm nodejs npm
      ;;
    *) warn "Install Node.js LTS from https://nodejs.org/en/download or via your package manager." ;;
  esac
}

if [ $AUTO -eq 1 ] && { [ $NODE_OK -eq 0 ] || [ $NPM_OK -eq 0 ]; }; then
  install_node || warn "Automatic Node.js installation failed."
  have_cmd node && NODE_OK=1
  have_cmd npm && NPM_OK=1
fi

# Rust toolchain
RUSTUP_OK=0; CARGO_OK=0
if have_cmd rustup; then info "rustup: $(rustup -V)"; RUSTUP_OK=1; else warn "rustup: not found"; fi
if have_cmd cargo; then info "cargo:  $(cargo -V)"; CARGO_OK=1; else warn "cargo:  not found"; fi

install_rust() {
  if curl --version >/dev/null 2>&1; then
    run curl https://sh.rustup.rs -sSf | sh -s -- -y
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
  else
    warn "curl not found; install curl and rerun --auto to install Rust (https://rustup.rs)."
  fi
}

if [ $AUTO -eq 1 ] && { [ $RUSTUP_OK -eq 0 ] || [ $CARGO_OK -eq 0 ]; }; then
  # Ensure curl for rustup installer
  if ! have_cmd curl; then
    case "$PKG" in
      apt) run sudo apt update; run sudo apt install -y curl ;; 
      dnf) run sudo dnf install -y curl ;; 
      pacman) run sudo pacman -Sy --needed --noconfirm curl ;;
    esac
  fi
  install_rust || warn "Automatic Rust installation failed."
  have_cmd rustup && RUSTUP_OK=1
  have_cmd cargo && CARGO_OK=1
fi

# System libraries required by Tauri (WebKitGTK, GTK3, appindicator, openssl, pkg-config, etc.)
install_system_libs() {
  case "$PKG" in
    apt)
      # Try WebKitGTK 4.1; fallback to 4.0 if 4.1 unavailable
      run sudo apt update
      if apt-cache show libwebkit2gtk-4.1-dev >/dev/null 2>&1; then
        run sudo apt install -y build-essential curl wget libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
      else
        run sudo apt install -y build-essential curl wget libssl-dev pkg-config libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev
      fi
      ;;
    dnf)
      if dnf info webkit2gtk4.1-devel >/dev/null 2>&1; then
        run sudo dnf install -y @"Development Tools" curl wget openssl-devel pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel libappindicator-gtk3 librsvg2-devel
      else
        run sudo dnf install -y @"Development Tools" curl wget openssl-devel pkgconf-pkg-config gtk3-devel webkit2gtk3-devel libappindicator-gtk3 librsvg2-devel
      fi
      ;;
    pacman)
      run sudo pacman -Sy --needed --noconfirm base-devel curl wget pkgconf openssl gtk3 webkit2gtk libappindicator-gtk3 librsvg
      ;;
    *) warn "Please install Tauri Linux prerequisites for your distro: https://tauri.app/v1/guides/getting-started/prerequisites/#linux" ;;
  esac
}

if [ $AUTO -eq 1 ]; then
  install_system_libs || warn "System libraries installation step reported issues."
fi

# Tauri CLI
CARGO_TAURI_OK=0; NPM_TAURI_OK=0
if have_cmd cargo; then if cargo tauri -V >/dev/null 2>&1; then CARGO_TAURI_OK=1; fi; fi
if have_cmd npm; then if npm ls -g --depth=0 @tauri-apps/cli >/dev/null 2>&1; then NPM_TAURI_OK=1; fi; fi

if [ $CARGO_TAURI_OK -eq 0 ] && [ $NPM_TAURI_OK -eq 0 ] && [ $AUTO -eq 1 ]; then
  if have_cmd cargo; then
    run cargo install tauri-cli || warn "cargo install tauri-cli failed"
  elif have_cmd npm; then
    run npm i -g @tauri-apps/cli || warn "npm i -g @tauri-apps/cli failed"
  fi
  if cargo tauri -V >/dev/null 2>&1; then CARGO_TAURI_OK=1; fi
  if npm ls -g --depth=0 @tauri-apps/cli >/dev/null 2>&1; then NPM_TAURI_OK=1; fi
fi

say "Summary"
missing=()
have_cmd node || missing+=("Node")
have_cmd npm || missing+=("npm")
have_cmd rustup || missing+=("rustup")
have_cmd cargo || missing+=("cargo")
# WebKitGTK etc. are best-effort; skip strict checks here
if [ ${#missing[@]} -eq 0 ]; then
  info "All core dependencies look good."
else
  warn "Missing: ${missing[*]}"
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE_DIR="$REPO_ROOT/release"
copy_artifacts_to_release() {
  mkdir -p "$RELEASE_DIR"
  local target_dir="$REPO_ROOT/src-tauri/target/release"
  if [ -d "$target_dir" ]; then
    say "Copying build artifacts to release/"
    # Copy main binaries and shared libs
    find "$target_dir" -maxdepth 1 -type f \( -name "*.exe" -o -name "*.dll" -o -name "*.pdb" -o -name "gmail*" \) -print -exec cp -f {} "$RELEASE_DIR" \; || true
    # Copy bundle directory recursively (deb, rpm, appimage, dmg, msi, nsis, macos, etc.)
    if [ -d "$target_dir/bundle" ]; then
      mkdir -p "$RELEASE_DIR/bundle"
      cp -rf "$target_dir/bundle"/* "$RELEASE_DIR/bundle" 2>/dev/null || true
    fi
  else
    warn "Build output directory not found: $target_dir"
  fi
}

if [ $DO_DEV -eq 1 ] || [ $DO_BUILD -eq 1 ]; then
  if [ ${#missing[@]} -ne 0 ]; then
    warn "Some dependencies are missing. Dev/Build may fail."
  fi
  pushd "$(dirname "$0")/../src-tauri" >/dev/null
  if [ $DO_DEV -eq 1 ]; then run cargo tauri dev; fi
  if [ $DO_BUILD -eq 1 ]; then run cargo tauri build; fi
  popd >/dev/null
  if [ $DO_BUILD -eq 1 ]; then copy_artifacts_to_release; fi
fi

echo "Done."
