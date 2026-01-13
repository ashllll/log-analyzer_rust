# Log Analyzer Release Process

This document outlines the complete release process for the Log Analyzer project, including CI/CD alignment and validation procedures.

## Overview

The Log Analyzer uses GitHub Actions for CI/CD with automated releases triggered by:
- Pushes to `main` branch (auto-increment patch version)
- Tag pushes with `v*` pattern (manual version specification)
- Manual workflow dispatch (flexible release control)

## Version Management

### Current Version Sources
- **Frontend**: `log-analyzer/package.json`
- **Backend**: `log-analyzer/src-tauri/Cargo.toml`
- **Tauri**: `log-analyzer/src-tauri/tauri.conf.json`

All version numbers must be consistent across these files.

### Version Strategy
- **Patch**: Bug fixes and minor improvements
- **Minor**: New features (backward compatible)
- **Major**: Breaking changes or significant new features

## Pre-Release Validation

### Automated Validation
Run the validation script before any release:

**Windows (PowerShell):**
```powershell
.\scripts\validate-release.ps1
```

**Linux/macOS (Bash):**
```bash
./scripts/validate-release.sh
```

### Manual Validation Steps
1. **Version Consistency**: Ensure all version files match
2. **Build Success**: Verify both frontend and backend build successfully
3. **Tests Pass**: All unit and integration tests pass
4. **Security Check**: Run `cargo audit` and `cargo outdated`
5. **Documentation**: Update CHANGELOG.md with recent changes
6. **Clean Working Directory**: Ensure no uncommitted changes

## Release Triggers

### 1. Automatic Release (Main Branch)
When code is pushed to `main`, the release workflow:
- Auto-increments patch version
- Updates version in all files
- Creates and pushes new tag
- Builds for all platforms
- Creates GitHub release with artifacts

### 2. Manual Tag Release
Create a specific version:
```bash
git tag v1.2.3
git push origin v1.2.3
```

### 3. Workflow Dispatch
Use GitHub CLI or web interface:
```bash
gh workflow run release.yml --ref main -f version=1.2.3 -f prerelease=false
```

## CI/CD Workflow Structure

### 1. PR Validation (`.github/workflows/pr-validation.yml`)
- **Triggers**: Pull requests to main/develop
- **Purpose**: Ensure code quality before merge
- **Checks**:
  - Version consistency
  - Build validation
  - Release readiness

### 2. CI Pipeline (`.github/workflows/ci.yml`)
- **Triggers**: Pushes to main/develop, PRs
- **Purpose**: Continuous integration testing
- **Jobs**:
  - Rust tests (multi-platform)
  - Frontend tests
  - Integration tests
  - Security scanning
  - Code quality reports

### 3. Release Pipeline (`.github/workflows/release.yml`)
- **Triggers**: Main branch pushes, tags, manual dispatch
- **Purpose**: Automated releases
- **Features**:
  - Version management
  - Multi-platform builds
  - Artifact creation
  - GitHub releases

## Platform Support

### Currently Supported
- **Windows**: x86_64-pc-windows-msvc
- **macOS**: x86_64-apple-darwin, aarch64-apple-darwin
- **Linux**: x86_64-unknown-linux-gnu

### Temporarily Disabled
- **Linux ARM64**: aarch64-unknown-linux-gnu (missing unrar binary)

## Release Artifacts

### Windows
- `.msi` installer
- `.exe` installer (NSIS)

### macOS
- `.dmg` disk image
- `.app` bundle

### Linux
- `.deb` package
- `.AppImage` portable

## Security Considerations

### Secrets Required
- `TAURI_PRIVATE_KEY`: Tauri updater signing key
- `TAURI_KEY_PASSWORD`: Password for signing key
- `GITHUB_TOKEN`: Automatic GitHub token (provided)

### Security Checks
- `cargo audit`: Security vulnerability scanning
- `cargo outdated`: Dependency freshness check
- Code signing for all platforms

## Troubleshooting

### Common Issues

#### Version Mismatch
```bash
# Check all version files
jq -r '.version' log-analyzer/package.json
grep '^version =' log-analyzer/src-tauri/Cargo.toml
jq -r '.version' log-analyzer/src-tauri/tauri.conf.json
```

#### Build Failures
1. Check Node.js version (requires v22+)
2. Verify Rust toolchain is installed
3. Ensure all dependencies are installed
4. Check Tauri system dependencies

#### Release Not Triggering
1. Verify branch protection rules
2. Check GitHub Actions permissions
3. Ensure secrets are configured
4. Review workflow logs

### Debug Commands

#### GitHub CLI
```bash
# List workflows
gh workflow list

# Run workflow manually
gh workflow run release.yml

# View workflow runs
gh run list

# Download artifacts
gh run download <run-id>
```

#### Local Testing
```bash
# Test build locally
cd log-analyzer
npm run tauri build

# Test specific target
npm run tauri build -- --target x86_64-unknown-linux-gnu
```

## Release Checklist

### Before Release
- [ ] All tests pass
- [ ] Version numbers consistent
- [ ] CHANGELOG.md updated
- [ ] Security audit clean
- [ ] Dependencies up to date
- [ ] Working directory clean

### During Release
- [ ] Monitor GitHub Actions
- [ ] Verify all platforms build
- [ ] Check artifact integrity
- [ ] Validate release notes

### After Release
- [ ] Test installation on each platform
- [ ] Verify auto-updater works
- [ ] Monitor for issues
- [ ] Update documentation

## Emergency Procedures

### Rollback Release
1. Delete problematic release on GitHub
2. Delete associated tag: `git push origin :v1.2.3`
3. Fix issues and create new release

### Hotfix Process
1. Create hotfix branch from main
2. Apply minimal fix
3. Follow normal release process
4. Version will auto-increment

## Support

For release-related issues:
1. Check GitHub Actions logs
2. Review this documentation
3. Create issue with release tag
4. Include platform and error details