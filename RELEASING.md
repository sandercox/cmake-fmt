# Releasing cmake-fmt

## Prerequisites

### Repository Secrets

The following GitHub repository secrets must be configured before the first release:

| Secret | Source | Purpose |
|--------|--------|---------|
| `VSCE_PAT` | Azure DevOps PAT with **Marketplace > Manage** scope | Publish to VS Code Marketplace |
| `OVSX_PAT` | [Open VSX access token](https://open-vsx.org/user-settings/tokens) | Publish to Open VSX Registry |
| `DOCKERHUB_USERNAME` | [Docker Hub](https://hub.docker.com/settings/security) account username | Push images to Docker Hub |
| `DOCKERHUB_TOKEN` | [Docker Hub](https://hub.docker.com/settings/security) access token | Push images to Docker Hub |
| `APPLE_CERTIFICATE_P12_BASE64` | Base64-encoded `.p12` Developer ID Application certificate | Sign macOS binaries |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the `.p12` certificate | Sign macOS binaries |
| `APPLE_SIGNING_IDENTITY` | Certificate common name (e.g. `Developer ID Application: Name (TEAMID)`) | Sign macOS binaries |
| `AZURE_TENANT_ID` | Azure Active Directory tenant ID | Sign Windows binaries |
| `AZURE_CLIENT_ID` | Azure app registration client ID | Sign Windows binaries |
| `AZURE_CLIENT_SECRET` | Azure app registration client secret | Sign Windows binaries |
| `AZURE_SIGNING_ENDPOINT` | Azure Trusted Signing endpoint URL | Sign Windows binaries |
| `AZURE_SIGNING_ACCOUNT` | Azure Trusted Signing account name | Sign Windows binaries |
| `AZURE_CERTIFICATE_PROFILE` | Azure Trusted Signing certificate profile name | Sign Windows binaries |

Note: GHCR authentication uses the built-in `GITHUB_TOKEN` — no additional secret needed.

### crates.io Authentication

Configure **one** of these:

- **Trusted publishing (recommended):** Configure the repository (`sandercox/cmake-fmt`) and workflow (`release.yml`) in your [crates.io account settings](https://crates.io/settings/tokens). No secret needed — authentication uses GitHub OIDC.
- **Token-based:** Create a crates.io API token and add it as the `CARGO_REGISTRY_TOKEN` repository secret. Only needed if trusted publishing is not configured.

### One-Time Open VSX Setup

Create the namespace before the first publish:

```bash
npx ovsx create-namespace cmake-fmt --pat <your-ovsx-token>
```

## Release Checklist

1. **Bump version in `Cargo.toml`:**

   ```toml
   version = "x.y.z"
   ```

2. **Bump version in `editors/vscode/package.json` to match exactly:**

   ```json
   "version": "x.y.z"
   ```

   Both files must have identical version strings, including any pre-release suffix (e.g., `1.0.0-beta.1`).

3. **Add a changelog section in `CHANGELOG.md`:**

   ```markdown
   ## [x.y.z] - YYYY-MM-DD
   ### Added
   - ...
   ### Fixed
   - ...
   ```

4. **Commit the version bump:**

   ```bash
   git commit -am "chore: bump version to x.y.z"
   ```

5. **Create the tag:**

   ```bash
   git tag vx.y.z
   ```

6. **Push the commit and tag:**

   ```bash
   git push && git push --tags
   ```

The tag **must** be on the `main` branch. The workflow validates this and will fail if the tagged commit is not on `main`.

## What the Workflow Does

The `release.yml` workflow runs automatically when a `v*.*.*` tag is pushed. Jobs run serially — each depends on the previous. If any job fails, all subsequent jobs are skipped.

### 1. Validate (tag pushes only)

Runs pre-build checks to catch errors before burning 6-platform build time:

- Verifies the tag version matches `Cargo.toml` version
- Verifies `editors/vscode/package.json` version matches `Cargo.toml`
- Checks that `CHANGELOG.md` has a `## [x.y.z]` section for this version
- Confirms the tagged commit exists on the `main` branch
- Runs `cargo publish --dry-run` to catch packaging/metadata issues

This job is **skipped** on `workflow_dispatch` (manual trigger).

### 2. Build (6-platform matrix)

Compiles release binaries for all supported platforms:

| Platform | Target | Runner |
|----------|--------|--------|
| linux-x64 | x86_64-unknown-linux-gnu | ubuntu-latest |
| linux-arm64 | aarch64-unknown-linux-gnu | ubuntu-latest (cross) |
| darwin-x64 | x86_64-apple-darwin | macos-latest (cross) |
| darwin-arm64 | aarch64-apple-darwin | macos-latest |
| win32-x64 | x86_64-pc-windows-msvc | windows-latest |
| win32-arm64 | aarch64-pc-windows-msvc | windows-latest (cross) |

For each platform:
- Builds the `cmake-fmt` binary
- Signs the binary (macOS: Apple Developer ID with hardened runtime; Windows: Azure Trusted Signing)
- Packages a standalone archive (`.tar.gz` on Linux/macOS, `.zip` on Windows)
- Syncs `package.json` version from `Cargo.toml`
- Packages a platform-specific VSIX with the signed binary
- For pre-release versions (odd minor): adds `--pre-release` flag to VSIX packaging

### 3. Sign VSIX Packages

Runs on a Windows runner after all builds complete. Downloads all 6 unsigned VSIX artifacts, signs each one using `dotnet/sign` with Azure Trusted Signing, and uploads the signed VSIXs for downstream publish jobs.

### 4. Collect

Downloads all archive artifacts and generates a `SHA256SUMS` checksum file.

### 5. Publish to crates.io

Publishes the Rust crate using OIDC trusted publishing (no long-lived token). Pre-release versions (e.g., `1.0.0-beta.1`) are published natively — crates.io supports semver pre-release identifiers.

### 6. Create GitHub Release

Creates a GitHub Release with:
- All platform archives (`.tar.gz` and `.zip`)
- `SHA256SUMS` checksum file
- Release notes extracted from `CHANGELOG.md`
- Pre-release flag set for pre-release tags

### 7. Publish to VS Code Marketplace

Publishes all 6 platform-specific VSIXs to the VS Code Marketplace.

### 8. Publish to Open VSX Registry

Publishes all 6 platform-specific VSIXs to the Open VSX Registry. For pre-release tags, the `--pre-release` flag is passed.

### 9. Publish Docker Images

A separate `docker.yml` workflow runs in parallel with the release pipeline (triggered by the same `v*.*.*` tag). It builds and pushes Docker images to both Docker Hub (`paralleldimension/cmake-fmt`) and GHCR (`ghcr.io/sandercox/cmake-fmt`).

- **Stable releases:** Images are tagged with `latest` and the version number on both registries (e.g., `paralleldimension/cmake-fmt:0.8.0` and `paralleldimension/cmake-fmt:latest`)
- **Pre-releases:** Images are tagged with the version number only — `latest` is never updated (e.g., `paralleldimension/cmake-fmt:0.8.0-beta.1`)

Because `docker.yml` is independent, a Docker build failure does not block crates.io, GitHub Release, or VS Code Marketplace publishing.

## Recovery Procedures

If a job fails, check the GitHub Actions logs to identify which step failed. Then follow the appropriate recovery procedure below.

### Validate Failed

**Symptoms:** Version mismatch, missing CHANGELOG entry, tag not on main, or cargo dry-run failure.

**Recovery:** Fix the issue, delete the tag, re-commit, re-tag, and push.

```bash
# Delete the tag locally and remotely
git tag -d vx.y.z
git push origin :refs/tags/vx.y.z

# Fix the issue (version mismatch, missing CHANGELOG entry, etc.)
# ...

# Commit the fix, re-tag, and push
git commit -am "fix: correct version for release x.y.z"
git tag vx.y.z
git push && git push --tags
```

### Build Failed

**Symptoms:** Compilation error, cross-compilation failure, VSIX packaging error.

**Recovery:** Fix the code, delete the tag, re-tag, and push. The workflow restarts from scratch.

```bash
git tag -d vx.y.z
git push origin :refs/tags/vx.y.z

# Fix the build issue
# ...

git commit -am "fix: resolve build issue for x.y.z"
git tag vx.y.z
git push && git push --tags
```

### crates.io Publish Failed

**Symptoms:** Authentication error, metadata validation failure, or network issue in the `publish-crates` job.

**Recovery:** Publish manually, then complete remaining steps manually.

```bash
cargo publish --token $CARGO_REGISTRY_TOKEN
```

> **Important:** Do NOT delete the tag and re-run the workflow after crates.io succeeds. crates.io publishes are **permanent** — the same version cannot be published twice. Complete the remaining steps manually instead.

After manual crates.io publish, continue with [GitHub Release](#github-release-creation-failed).

### GitHub Release Creation Failed

**Symptoms:** Permission error or artifact download issue in the `publish-github-release` job.

**Recovery:** Create the release manually using the GitHub CLI.

```bash
# Download the build artifacts from the workflow run first,
# or rebuild locally if artifacts have expired

gh release create vx.y.z \
  archives/*.tar.gz \
  archives/*.zip \
  archives/SHA256SUMS \
  --title "cmake-fmt vx.y.z" \
  --notes-file release-notes.md
```

For pre-release versions, add `--prerelease`:

```bash
gh release create vx.y.z \
  archives/*.tar.gz \
  archives/*.zip \
  archives/SHA256SUMS \
  --title "cmake-fmt vx.y.z" \
  --notes-file release-notes.md \
  --prerelease
```

After manual GitHub Release, continue with [VS Code Marketplace](#vs-code-marketplace-publish-failed).

### VS Code Marketplace Publish Failed

**Symptoms:** Authentication error or VSIX upload failure in the `publish-vscode-marketplace` job.

**Recovery:** Publish each platform VSIX manually.

```bash
for vsix in vsix/*.vsix; do
  npx @vscode/vsce publish --packagePath "$vsix" --pat $VSCE_PAT
done
```

After manual VS Code Marketplace publish, continue with [Open VSX](#open-vsx-publish-failed).

### Open VSX Publish Failed

**Symptoms:** Authentication error or VSIX upload failure in the `publish-openvsx` job.

**Recovery:** Publish each platform VSIX manually.

```bash
for vsix in vsix/*.vsix; do
  npx ovsx publish "$vsix" --pat $OVSX_PAT
done
```

For pre-release versions, add `--pre-release`:

```bash
for vsix in vsix/*.vsix; do
  npx ovsx publish "$vsix" --pat $OVSX_PAT --pre-release
done
```

### Docker Publish Failed

**Symptoms:** Docker build failure, registry authentication error, or push failure in the `docker.yml` workflow.

**Recovery:** Build and push manually.

```bash
# Build the image
docker build --build-arg REV=vx.y.z -t paralleldimension/cmake-fmt:x.y.z -t ghcr.io/sandercox/cmake-fmt:x.y.z .

# For stable releases, also tag as latest
docker tag paralleldimension/cmake-fmt:x.y.z paralleldimension/cmake-fmt:latest
docker tag ghcr.io/sandercox/cmake-fmt:x.y.z ghcr.io/sandercox/cmake-fmt:latest

# Push to Docker Hub
docker push paralleldimension/cmake-fmt --all-tags

# Push to GHCR
docker push ghcr.io/sandercox/cmake-fmt --all-tags
```

## Pre-Release Releases

Pre-releases use **odd minor versions**. The VS Code marketplace does not allow a version number previously used as pre-release to be republished as a stable release, so odd/even minor parity determines the release channel:

- **Odd minor** (e.g. `0.9.x`) = pre-release
- **Even minor** (e.g. `0.10.x`) = stable release

Pre-release suffixes (`-beta.N`, `-rc.N`) are allowed on odd minor versions for crates.io but are optional:

```
v0.9.0-beta.1    # odd minor → pre-release
v0.9.1-rc.1      # odd minor → pre-release
v0.10.0           # even minor → stable
```

The `bump-version.sh` script enforces this: pre-release suffixes on even minor versions are rejected.

### What Changes for Pre-Releases

| Channel | Behavior |
|---------|----------|
| crates.io | Publishes the version natively (semver pre-release suffixes supported) |
| GitHub Release | Marked as **pre-release** (not shown as "Latest") |
| VS Code Marketplace | VSIX packaged and published with `--pre-release` flag |
| Open VSX Registry | Published with `--pre-release` flag |
| Docker Hub / GHCR | Tagged with version only (no `latest` tag) |

### Version in Source Files

Both `Cargo.toml` and `editors/vscode/package.json` must have identical version strings.

Pre-release extensions appear in VS Code only for users who opt into pre-release versions.

## Version Numbering

| Type | When to Use | Example |
|------|-------------|---------|
| **Patch** (x.y.**Z**) | Bug fixes, minor formatting improvements | `0.10.0` -> `0.10.1` |
| **Minor** (x.**Y**.0) | New features, new config options, new CMake command support | `0.10.0` -> `0.12.0` |
| **Major** (**X**.0.0) | Breaking changes to formatting output, config file format changes, CLI interface changes | `0.12.0` -> `1.0.0` |

**Note:** Stable releases must use even minor versions. Odd minor versions are reserved for pre-releases (see [Pre-Release Releases](#pre-release-releases)).

## Manual Workflow Dispatch

The workflow can be triggered manually from the GitHub Actions UI:

**Actions > Release > Run workflow**

Manual runs **skip the validate job entirely** — useful for testing the build and publish pipeline without a real tag. Pre-release detection defaults to non-pre-release for manual runs (safe default).
