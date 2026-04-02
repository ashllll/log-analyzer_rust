#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import process from 'node:process';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

const versionFiles = [
  { kind: 'json', path: 'log-analyzer/package.json', label: 'package.json' },
  { kind: 'cargo', path: 'log-analyzer/src-tauri/Cargo.toml', label: 'src-tauri/Cargo.toml' },
  { kind: 'json', path: 'log-analyzer/src-tauri/tauri.conf.json', label: 'tauri.conf.json' },
  { kind: 'cargo', path: 'log-analyzer/src-tauri/crates/la-core/Cargo.toml', label: 'la-core/Cargo.toml' },
  { kind: 'cargo', path: 'log-analyzer/src-tauri/crates/la-storage/Cargo.toml', label: 'la-storage/Cargo.toml' },
  { kind: 'cargo', path: 'log-analyzer/src-tauri/crates/la-search/Cargo.toml', label: 'la-search/Cargo.toml' },
  { kind: 'cargo', path: 'log-analyzer/src-tauri/crates/la-archive/Cargo.toml', label: 'la-archive/Cargo.toml' },
];

function usage() {
  console.error(`Usage:
  node scripts/prepare-release.mjs plan
  node scripts/prepare-release.mjs apply --version <semver>
  node scripts/prepare-release.mjs check`);
  process.exit(1);
}

function isSemver(value) {
  return /^\d+\.\d+\.\d+$/.test(value);
}

function parseSemver(version) {
  if (!isSemver(version)) {
    throw new Error(`Invalid semver version: ${version}`);
  }
  return version.split('.').map((part) => Number(part));
}

function compareSemver(left, right) {
  const [la, lb, lc] = parseSemver(left);
  const [ra, rb, rc] = parseSemver(right);
  if (la !== ra) return la - ra;
  if (lb !== rb) return lb - rb;
  return lc - rc;
}

function bumpPatch(version) {
  const [major, minor, patch] = parseSemver(version);
  return `${major}.${minor}.${patch + 1}`;
}

function git(args) {
  return execFileSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
  }).trim();
}

function readFileVersion(file) {
  const absPath = path.join(repoRoot, file.path);
  const content = fs.readFileSync(absPath, 'utf8');

  if (file.kind === 'json') {
    const parsed = JSON.parse(content);
    if (!parsed.version || !isSemver(parsed.version)) {
      throw new Error(`${file.label} does not contain a valid version`);
    }
    return parsed.version;
  }

  const match = content.match(/^version = "([^"]+)"/m);
  if (!match || !isSemver(match[1])) {
    throw new Error(`${file.label} does not contain a valid Cargo version`);
  }
  return match[1];
}

function writeFileVersion(file, version) {
  const absPath = path.join(repoRoot, file.path);
  const content = fs.readFileSync(absPath, 'utf8');

  if (file.kind === 'json') {
    const parsed = JSON.parse(content);
    parsed.version = version;
    fs.writeFileSync(absPath, `${JSON.stringify(parsed, null, 2)}\n`);
    return;
  }

  const updated = content.replace(/^version = "[^"]+"/m, `version = "${version}"`);
  fs.writeFileSync(absPath, updated);
}

function readWorkspaceVersionState() {
  const versions = versionFiles.map((file) => ({
    ...file,
    version: readFileVersion(file),
  }));

  const unique = [...new Set(versions.map((entry) => entry.version))];
  if (unique.length !== 1) {
    const details = versions.map((entry) => `${entry.label}=${entry.version}`).join(', ');
    throw new Error(`Workspace versions are inconsistent: ${details}`);
  }

  return {
    currentVersion: unique[0],
    versions,
  };
}

function findLatestTagVersion() {
  const tags = git(['tag', '-l', 'v*', '--sort=-v:refname'])
    .split('\n')
    .map((tag) => tag.trim())
    .filter(Boolean);

  const latestTag = tags.find((tag) => /^v\d+\.\d+\.\d+$/.test(tag)) ?? 'v0.0.0';
  return {
    latestTag,
    latestTagVersion: latestTag.slice(1),
  };
}

function planReleaseVersion() {
  const { currentVersion } = readWorkspaceVersionState();
  const { latestTag, latestTagVersion } = findLatestTagVersion();
  const nextPatchVersion = bumpPatch(latestTagVersion);

  let targetVersion = nextPatchVersion;
  let strategy = 'bump_from_latest_tag';

  if (compareSemver(currentVersion, latestTagVersion) > 0) {
    targetVersion = currentVersion;
    strategy = compareSemver(currentVersion, nextPatchVersion) === 0
      ? 'use_current_workspace_version'
      : 'preserve_ahead_workspace_version';
  }

  const changed = currentVersion !== targetVersion;

  return {
    latestTag,
    latestTagVersion,
    currentVersion,
    nextPatchVersion,
    targetVersion,
    tag: `v${targetVersion}`,
    changed: String(changed),
    strategy,
  };
}

function emitOutputs(result) {
  const lines = Object.entries(result).map(([key, value]) => `${key}=${value}`);
  if (process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join('\n')}\n`);
  }
  process.stdout.write(`${lines.join('\n')}\n`);
}

function main() {
  const [mode, ...rest] = process.argv.slice(2);
  if (!mode) usage();

  if (mode === 'check') {
    const state = readWorkspaceVersionState();
    emitOutputs({
      currentVersion: state.currentVersion,
      checked: 'true',
    });
    return;
  }

  if (mode === 'plan') {
    emitOutputs(planReleaseVersion());
    return;
  }

  if (mode === 'apply') {
    const versionIndex = rest.indexOf('--version');
    if (versionIndex === -1 || !rest[versionIndex + 1]) {
      usage();
    }

    const targetVersion = rest[versionIndex + 1];
    if (!isSemver(targetVersion)) {
      throw new Error(`Invalid target version: ${targetVersion}`);
    }

    const state = readWorkspaceVersionState();
    if (state.currentVersion !== targetVersion) {
      for (const file of versionFiles) {
        writeFileVersion(file, targetVersion);
      }
    }

    const verified = readWorkspaceVersionState();
    emitOutputs({
      currentVersion: verified.currentVersion,
      targetVersion,
      changed: String(state.currentVersion !== targetVersion),
      tag: `v${targetVersion}`,
      applied: 'true',
    });
    return;
  }

  usage();
}

try {
  main();
} catch (error) {
  console.error(`prepare-release failed: ${error.message}`);
  process.exit(1);
}
