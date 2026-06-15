#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workflowsDir = join(projectRoot, ".github", "workflows");
const setupAction = "./.github/actions/setup-tauri-linux";
const verifyRemote = process.argv.includes("--verify-remote");
const errors = [];
const remoteRefs = new Map();
const remoteQueryAttempts = 3;

const workflowFiles = readdirSync(workflowsDir)
  .filter((name) => name.endsWith(".yml") || name.endsWith(".yaml"))
  .sort();

function addError(file, message) {
  errors.push(`${file}: ${message}`);
}

function sleep(milliseconds) {
  Atomics.wait(
    new Int32Array(new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT)),
    0,
    0,
    milliseconds,
  );
}

function queryRemoteRefs(repository) {
  let lastError;

  for (let attempt = 1; attempt <= remoteQueryAttempts; attempt += 1) {
    try {
      return execFileSync(
        "git",
        ["ls-remote", `https://github.com/${repository}.git`],
        {
          encoding: "utf8",
          env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
          stdio: ["ignore", "pipe", "pipe"],
          timeout: 30_000,
        },
      );
    } catch (error) {
      lastError = error;
      if (attempt < remoteQueryAttempts) {
        sleep(attempt * 1_000);
      }
    }
  }

  throw lastError;
}

function jobBlocks(source) {
  const lines = source.split(/\r?\n/);
  const jobsLine = lines.findIndex((line) => line === "jobs:");
  if (jobsLine === -1) {
    return [];
  }

  const blocks = [];
  for (let index = jobsLine + 1; index < lines.length; index += 1) {
    const match = lines[index].match(/^  ([A-Za-z0-9_-]+):\s*$/);
    if (!match) {
      continue;
    }

    const start = index;
    let end = lines.length;
    for (let next = index + 1; next < lines.length; next += 1) {
      if (/^  [A-Za-z0-9_-]+:\s*$/.test(lines[next])) {
        end = next;
        break;
      }
    }
    blocks.push({ name: match[1], text: lines.slice(start, end).join("\n") });
    index = end - 1;
  }
  return blocks;
}

for (const file of workflowFiles) {
  const relativeFile = `.github/workflows/${file}`;
  const source = readFileSync(join(workflowsDir, file), "utf8");

  if (source.includes("\t")) {
    addError(relativeFile, "YAML must not contain tab indentation");
  }

  for (const match of source.matchAll(/^\s*uses:\s*([^\s#]+)\s*$/gm)) {
    const action = match[1];
    if (action.startsWith("./")) {
      continue;
    }

    const separator = action.lastIndexOf("@");
    if (separator === -1) {
      addError(relativeFile, `action is missing a ref: ${action}`);
      continue;
    }

    const actionPath = action.slice(0, separator);
    const ref = action.slice(separator + 1);
    if (!/^[0-9a-f]{40}$/.test(ref)) {
      addError(relativeFile, `action must use a full commit SHA: ${action}`);
      continue;
    }

    const repository = actionPath.split("/").slice(0, 2).join("/");
    if (!remoteRefs.has(repository)) {
      remoteRefs.set(repository, new Set());
    }
    remoteRefs.get(repository).add(ref);
  }

  if (source.includes("libwebkit2gtk-4.1-dev")) {
    addError(
      relativeFile,
      `native dependencies must be defined only in ${setupAction}`,
    );
  }

  for (const job of jobBlocks(source)) {
    const targetsUbuntu =
      /runs-on:\s*ubuntu-[^\s]+/.test(job.text) ||
      /-\s*os:\s*ubuntu-[^\s]+/.test(job.text);
    const buildsTauri =
      /\bcargo\s+(build|check|clippy|test|tarpaulin)\b/.test(job.text) ||
      /\bnpm\s+run\s+tauri\s+build\b/.test(job.text) ||
      /uses:\s*tauri-apps\/tauri-action@/.test(job.text);

    if (
      targetsUbuntu &&
      buildsTauri &&
      !job.text.includes(`uses: ${setupAction}`)
    ) {
      addError(
        relativeFile,
        `job "${job.name}" builds Rust/Tauri on Ubuntu without ${setupAction}`,
      );
    }
  }
}

if (verifyRemote && errors.length === 0) {
  for (const [repository, refs] of [...remoteRefs].sort()) {
    let output;
    try {
      output = queryRemoteRefs(repository);
    } catch {
      addError(
        repository,
        `unable to query remote refs after ${remoteQueryAttempts} attempts`,
      );
      continue;
    }

    const advertisedShas = new Set(
      output
        .split(/\r?\n/)
        .filter(Boolean)
        .map((line) => line.split(/\s+/)[0]),
    );
    for (const ref of refs) {
      if (!advertisedShas.has(ref)) {
        addError(repository, `pinned SHA is not advertised by the remote: ${ref}`);
      }
    }
  }
}

if (errors.length > 0) {
  console.error("CI workflow validation failed:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

console.log(
  `CI workflow validation passed (${workflowFiles.length} workflows, ${remoteRefs.size} action repositories${verifyRemote ? ", remote refs verified" : ""}).`,
);
