#!/usr/bin/env bash
set -euo pipefail

# Build platform-specific VS Code extension packages (.vsix).
#
# Usage:
#   ./scripts/build-vsix.sh              # build all 6 platform targets
#   ./scripts/build-vsix.sh --native     # build for the current platform only
#   ./scripts/build-vsix.sh linux-x64    # build a single target

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$EXT_DIR/../.." && pwd)"

BIN_DIR="$EXT_DIR/bin"

# Map: vscode target -> rust target
declare -A TARGET_MAP=(
  [linux-x64]=x86_64-unknown-linux-gnu
  [linux-arm64]=aarch64-unknown-linux-gnu
  [darwin-x64]=x86_64-apple-darwin
  [darwin-arm64]=aarch64-apple-darwin
  [win32-x64]=x86_64-pc-windows-msvc
  [win32-arm64]=aarch64-pc-windows-msvc
)

ALL_TARGETS=(linux-x64 linux-arm64 darwin-x64 darwin-arm64 win32-x64 win32-arm64)

# ---------------------------------------------------------------------------
# Detect the best available build tool
# ---------------------------------------------------------------------------
detect_build_tool() {
  if command -v cross &>/dev/null && docker info &>/dev/null 2>&1; then
    echo "cross"
  elif command -v cargo-zigbuild &>/dev/null; then
    echo "cargo-zigbuild"
  else
    echo "cargo"
  fi
}

BUILD_TOOL="$(detect_build_tool)"

# ---------------------------------------------------------------------------
# Detect the native VS Code target for the current host
# ---------------------------------------------------------------------------
native_target() {
  local os arch
  case "$(uname -s)" in
    Linux)  os=linux ;;
    Darwin) os=darwin ;;
    MINGW*|MSYS*|CYGWIN*) os=win32 ;;
    *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch=x64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) echo "Unsupported arch: $(uname -m)" >&2; exit 1 ;;
  esac
  echo "${os}-${arch}"
}

# ---------------------------------------------------------------------------
# Build one target
# ---------------------------------------------------------------------------
build_target() {
  local vscode_target="$1"
  local rust_target="${TARGET_MAP[$vscode_target]}"

  echo "=== Building $vscode_target (rust: $rust_target) ==="

  # Clean bin/ so each .vsix gets exactly one binary
  rm -rf "$BIN_DIR"
  mkdir -p "$BIN_DIR"

  # Compile
  case "$BUILD_TOOL" in
    cross)
      cross build --release --target "$rust_target" --manifest-path "$REPO_ROOT/Cargo.toml"
      ;;
    cargo-zigbuild)
      cargo zigbuild --release --target "$rust_target" --manifest-path "$REPO_ROOT/Cargo.toml"
      ;;
    cargo)
      cargo build --release --target "$rust_target" --manifest-path "$REPO_ROOT/Cargo.toml"
      ;;
  esac

  # Copy binary into bin/
  local bin_name="cmake-fmt"
  if [[ "$vscode_target" == win32-* ]]; then
    bin_name="cmake-fmt.exe"
  fi

  local built="$REPO_ROOT/target/$rust_target/release/$bin_name"
  if [[ ! -f "$built" ]]; then
    echo "ERROR: expected binary not found: $built" >&2
    exit 1
  fi

  cp "$built" "$BIN_DIR/$bin_name"

  # Strip symbols to reduce size (skip Windows — MSVC binaries use a different toolchain)
  if [[ "$vscode_target" != win32-* ]]; then
    if command -v strip &>/dev/null; then
      strip "$BIN_DIR/$bin_name" 2>/dev/null || true
    fi
  fi

  # Install npm deps if needed
  if [[ ! -d "$EXT_DIR/node_modules" ]]; then
    npm --prefix "$EXT_DIR" install
  fi

  # Package .vsix
  (cd "$EXT_DIR" && npx @vscode/vsce package --target "$vscode_target")

  echo "=== Done: $vscode_target ==="
  echo
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  local targets=()

  if [[ $# -eq 0 ]]; then
    targets=("${ALL_TARGETS[@]}")
  elif [[ "$1" == "--native" ]]; then
    targets=("$(native_target)")
  else
    for arg in "$@"; do
      if [[ -z "${TARGET_MAP[$arg]+x}" ]]; then
        echo "Unknown target: $arg" >&2
        echo "Valid targets: ${ALL_TARGETS[*]}" >&2
        exit 1
      fi
      targets+=("$arg")
    done
  fi

  echo "Build tool: $BUILD_TOOL"
  echo "Targets:    ${targets[*]}"
  echo

  for t in "${targets[@]}"; do
    build_target "$t"
  done

  # Clean up bin/ after all builds
  rm -rf "$BIN_DIR"

  echo "All done. VSIX files:"
  ls -lh "$EXT_DIR"/*.vsix 2>/dev/null || echo "(none found)"
}

main "$@"
