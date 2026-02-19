#!/usr/bin/env bash
set -euo pipefail

# Bump the version number in Cargo.toml and editors/vscode/package.json,
# regenerate Cargo.lock via cargo check, and create a git commit.
#
# Usage:
#   ./scripts/bump-version.sh <new-version>
#   ./scripts/bump-version.sh --dry-run <new-version>
#
# Supported version formats:
#   X.Y.Z           (stable release)
#   X.Y.Z-alpha.N   (alpha pre-release)
#   X.Y.Z-beta.N    (beta pre-release)
#   X.Y.Z-rc.N      (release candidate)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CARGO_TOML="$REPO_ROOT/Cargo.toml"
PACKAGE_JSON="$REPO_ROOT/editors/vscode/package.json"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
DRY_RUN=false

usage() {
  echo "Usage: $0 [--dry-run] <new-version>"
  echo ""
  echo "Examples:"
  echo "  $0 0.8.0"
  echo "  $0 0.8.0-beta.1"
  echo "  $0 --dry-run 0.8.0"
  exit 1
}

if [[ $# -eq 0 ]]; then
  usage
fi

if [[ "$1" == "--dry-run" ]]; then
  DRY_RUN=true
  shift
fi

if [[ $# -ne 1 ]]; then
  usage
fi

NEW_VERSION="$1"

# ---------------------------------------------------------------------------
# Semver parsing
# ---------------------------------------------------------------------------
# Outputs 5 space-separated fields: major minor patch pre_tag pre_num
# For stable versions, pre_tag is the literal string "stable" and pre_num is 0.
# For pre-releases: pre_tag is alpha/beta/rc and pre_num is the number.
parse_semver() {
  local version="$1"
  local base pre_tag pre_num

  # Split on '-' to separate base from pre-release
  if [[ "$version" == *-* ]]; then
    base="${version%%-*}"
    local pre_part="${version#*-}"

    # pre_part is like "alpha.1", "beta.2", "rc.3"
    if [[ "$pre_part" =~ ^(alpha|beta|rc)\.([0-9]+)$ ]]; then
      pre_tag="${BASH_REMATCH[1]}"
      pre_num="${BASH_REMATCH[2]}"
    else
      echo "ERROR: Invalid pre-release format '${pre_part}'. Expected alpha.N, beta.N, or rc.N." >&2
      exit 1
    fi
  else
    base="$version"
    pre_tag="stable"
    pre_num=0
  fi

  # Validate base is X.Y.Z
  if [[ "$base" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    echo "${BASH_REMATCH[1]} ${BASH_REMATCH[2]} ${BASH_REMATCH[3]} ${pre_tag} ${pre_num}"
  else
    echo "ERROR: Invalid version format '${version}'. Expected X.Y.Z or X.Y.Z-{alpha|beta|rc}.N." >&2
    exit 1
  fi
}

# Returns numeric ordering weight for a pre-release tag:
#   alpha=1, beta=2, rc=3, stable=4
pre_tag_weight() {
  case "$1" in
    alpha)  echo 1 ;;
    beta)   echo 2 ;;
    rc)     echo 3 ;;
    stable) echo 4 ;;
    *) echo "ERROR: Unknown pre-release tag '$1'" >&2; exit 1 ;;
  esac
}

# ---------------------------------------------------------------------------
# Validate version ordering: new_version > current_version
# Exits with error message if invalid.
# ---------------------------------------------------------------------------
validate_version_order() {
  local cur_ver="$1"
  local new_ver="$2"

  local cur_parsed new_parsed
  cur_parsed="$(parse_semver "$cur_ver")"
  new_parsed="$(parse_semver "$new_ver")"

  local cur_maj cur_min cur_pat cur_pre_tag cur_pre_num
  local new_maj new_min new_pat new_pre_tag new_pre_num
  read -r cur_maj cur_min cur_pat cur_pre_tag cur_pre_num <<< "$cur_parsed"
  read -r new_maj new_min new_pat new_pre_tag new_pre_num <<< "$new_parsed"

  local cur_base_gt=false
  local cur_base_lt=false
  local base_equal=false

  # Compare base versions numerically
  if   (( new_maj > cur_maj )); then cur_base_gt=true
  elif (( new_maj < cur_maj )); then cur_base_lt=true
  elif (( new_min > cur_min )); then cur_base_gt=true
  elif (( new_min < cur_min )); then cur_base_lt=true
  elif (( new_pat > cur_pat )); then cur_base_gt=true
  elif (( new_pat < cur_pat )); then cur_base_lt=true
  else base_equal=true
  fi

  # If new base > current base: always valid (regardless of pre-release)
  if [[ "$cur_base_gt" == true ]]; then
    return 0
  fi

  # If new base < current base: always invalid
  if [[ "$cur_base_lt" == true ]]; then
    local new_base="${new_maj}.${new_min}.${new_pat}"
    local cur_base="${cur_maj}.${cur_min}.${cur_pat}"
    echo "ERROR: Cannot downgrade from ${cur_ver} to ${new_ver}: new base version ${new_base} is less than current base version ${cur_base}." >&2
    exit 1
  fi

  # Base versions are equal — compare pre-release
  # ($base_equal is always true here, but kept for clarity)
  if [[ "$base_equal" == true ]]; then
    # Case: same version (no change)
    if [[ "$new_ver" == "$cur_ver" ]]; then
      echo "ERROR: New version ${new_ver} is the same as the current version ${cur_ver}. Nothing to bump." >&2
      exit 1
    fi

    # Case: stable -> pre-release of same base (e.g. 0.7.0 -> 0.7.0-beta.1)
    if [[ "$cur_pre_tag" == "stable" && "$new_pre_tag" != "stable" ]]; then
      echo "ERROR: Cannot downgrade from ${cur_ver} to ${new_ver}: a stable release cannot transition to a pre-release of the same base version." >&2
      exit 1
    fi

    # Case: pre-release -> stable of same base (e.g. 0.7.0-beta.1 -> 0.7.0): ALLOW
    if [[ "$cur_pre_tag" != "stable" && "$new_pre_tag" == "stable" ]]; then
      return 0
    fi

    # Case: pre-release -> pre-release of same base
    if [[ "$cur_pre_tag" != "stable" && "$new_pre_tag" != "stable" ]]; then
      local cur_weight new_weight
      cur_weight=$(pre_tag_weight "$cur_pre_tag")
      new_weight=$(pre_tag_weight "$new_pre_tag")

      if   (( new_weight > cur_weight )); then
        # Higher tier (e.g. beta.1 -> rc.1): ALLOW
        return 0
      elif (( new_weight < cur_weight )); then
        echo "ERROR: Cannot downgrade from ${cur_ver} to ${new_ver}: pre-release tier '${new_pre_tag}' is lower than '${cur_pre_tag}'." >&2
        exit 1
      else
        # Same tier — compare number
        if (( new_pre_num > cur_pre_num )); then
          return 0
        else
          echo "ERROR: Cannot downgrade from ${cur_ver} to ${new_ver}: ${new_pre_tag}.${new_pre_num} is not greater than ${cur_pre_tag}.${cur_pre_num}." >&2
          exit 1
        fi
      fi
    fi
  fi
}

# ---------------------------------------------------------------------------
# Read current version from Cargo.toml
# ---------------------------------------------------------------------------
read_cargo_version() {
  # Match the first 'version = "..."' line (which belongs to [package])
  local version
  version=$(grep -m1 '^version = ' "$CARGO_TOML" | sed 's/^version = "\(.*\)"/\1/')
  if [[ -z "$version" ]]; then
    echo "ERROR: Could not read version from $CARGO_TOML" >&2
    exit 1
  fi
  echo "$version"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
CURRENT_VERSION="$(read_cargo_version)"

echo "Current version : ${CURRENT_VERSION}"
echo "New version     : ${NEW_VERSION}"
echo ""

# Validate the new version format (parse_semver exits on error)
parse_semver "$NEW_VERSION" > /dev/null

# Validate ordering
validate_version_order "$CURRENT_VERSION" "$NEW_VERSION"

echo "Version transition is valid."
echo ""

if [[ "$DRY_RUN" == true ]]; then
  echo "[DRY RUN] Would update:"
  echo "  - $CARGO_TOML"
  echo "  - $PACKAGE_JSON"
  echo "  - Cargo.lock (via cargo check)"
  echo ""
  echo "[DRY RUN] Would create commit: chore: bump version from ${CURRENT_VERSION} to ${NEW_VERSION}"
  echo ""
  echo "No files were modified."
  exit 0
fi

# ---------------------------------------------------------------------------
# Update Cargo.toml
# ---------------------------------------------------------------------------
echo "Updating $CARGO_TOML..."
sed -i "s/^version = \"${CURRENT_VERSION}\"/version = \"${NEW_VERSION}\"/" "$CARGO_TOML"

# Verify the update took effect
UPDATED_CARGO_VERSION="$(read_cargo_version)"
if [[ "$UPDATED_CARGO_VERSION" != "$NEW_VERSION" ]]; then
  echo "ERROR: Failed to update version in $CARGO_TOML (got '$UPDATED_CARGO_VERSION', expected '$NEW_VERSION')" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Update editors/vscode/package.json
# ---------------------------------------------------------------------------
echo "Updating $PACKAGE_JSON..."
node -e "
  const fs = require('fs');
  const path = '$PACKAGE_JSON';
  const pkg = JSON.parse(fs.readFileSync(path, 'utf8'));
  if (pkg.version !== '$CURRENT_VERSION') {
    console.error('ERROR: package.json version ' + pkg.version + ' does not match expected ' + '$CURRENT_VERSION');
    process.exit(1);
  }
  pkg.version = '$NEW_VERSION';
  fs.writeFileSync(path, JSON.stringify(pkg, null, 2) + '\n');
  console.log('Updated package.json version to ' + pkg.version);
"

# ---------------------------------------------------------------------------
# Regenerate Cargo.lock via cargo check
# ---------------------------------------------------------------------------
echo "Regenerating Cargo.lock via cargo check..."
cargo check --manifest-path "$CARGO_TOML" --quiet

# ---------------------------------------------------------------------------
# Git commit
# ---------------------------------------------------------------------------
echo ""
echo "Staging files and creating commit..."
git -C "$REPO_ROOT" add "$CARGO_TOML" "$REPO_ROOT/Cargo.lock" "$PACKAGE_JSON"
git -C "$REPO_ROOT" commit -m "chore: bump version from ${CURRENT_VERSION} to ${NEW_VERSION}"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "Version bump complete!"
echo ""
echo "  Old version : ${CURRENT_VERSION}"
echo "  New version : ${NEW_VERSION}"
echo ""
echo "  Files updated:"
echo "    - Cargo.toml"
echo "    - Cargo.lock"
echo "    - editors/vscode/package.json"
echo ""
COMMIT_HASH=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
echo "  Commit created: ${COMMIT_HASH} (chore: bump version from ${CURRENT_VERSION} to ${NEW_VERSION})"
echo ""
echo "  To tag this release: git tag v${NEW_VERSION}"
