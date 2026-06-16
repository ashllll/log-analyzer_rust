#!/usr/bin/env bash
# Poll GitHub Actions failures, repair safe mechanical issues locally, push,
# then keep watching the pushed commit until CI succeeds or fails again.
#
# Required:
#   gh auth login
#
# Common usage:
#   make ci-autofix-loop-push
#   CI_AUTOFIX_INTERVAL_SECONDS=60 make ci-autofix-loop-push
#
# Tunables:
#   CI_AUTOFIX_INTERVAL_SECONDS=300
#   CI_AUTOFIX_STATUS_INTERVAL_SECONDS=30
#   CI_AUTOFIX_REPO=owner/name
#   CI_AUTOFIX_WORKFLOWS="CI/CD Pipeline,PR Validation"
#   CI_AUTOFIX_VALIDATE=quick|full|none
#   CI_AUTOFIX_PUSH=0|1
#   CI_AUTOFIX_WAIT_FOR_CI=1

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_DIR="$ROOT/.ci-autofix"
STATE_FILE="$STATE_DIR/processed-runs"
LOG_DIR="$STATE_DIR/logs"
WORKTREE_DIR="$STATE_DIR/worktrees"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$STATE_DIR/cargo-target}"

INTERVAL_SECONDS="${CI_AUTOFIX_INTERVAL_SECONDS:-300}"
STATUS_INTERVAL_SECONDS="${CI_AUTOFIX_STATUS_INTERVAL_SECONDS:-30}"
WORKFLOWS="${CI_AUTOFIX_WORKFLOWS:-CI/CD Pipeline,PR Validation}"
VALIDATE_MODE="${CI_AUTOFIX_VALIDATE:-quick}"
PUSH_CHANGES="${CI_AUTOFIX_PUSH:-0}"
WAIT_FOR_CI="${CI_AUTOFIX_WAIT_FOR_CI:-1}"
REPO="${CI_AUTOFIX_REPO:-}"

log() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"
}

die() {
  log "ERROR: $*"
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

resolve_repo() {
  if [[ -n "$REPO" ]]; then
    printf '%s\n' "$REPO"
    return
  fi

  gh repo view --json nameWithOwner --jq .nameWithOwner
}

ensure_ready() {
  require_cmd cargo
  require_cmd gh
  require_cmd git
  require_cmd jq
  require_cmd node
  require_cmd npm

  mkdir -p "$STATE_DIR" "$LOG_DIR" "$WORKTREE_DIR" "$CARGO_TARGET_DIR"
  touch "$STATE_FILE"

  gh auth status >/dev/null 2>&1 || die "GitHub CLI is not authenticated. Run: gh auth login"
}

trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s\n' "$value"
}

workflow_wanted() {
  local workflow="$1"
  local IFS=','
  local item

  for item in $WORKFLOWS; do
    item="$(trim "$item")"
    [[ "$workflow" == "$item" ]] && return 0
  done

  return 1
}

sanitize_branch() {
  printf '%s' "$1" | tr -c 'A-Za-z0-9._-' '_'
}

latest_failed_run_json() {
  local repo="$1"

  gh run list \
    --repo "$repo" \
    --status failure \
    --limit 20 \
    --json databaseId,workflowName,headBranch,headSha,conclusion,status,url,createdAt,event \
    --jq '.[] | select(.status == "completed" and .conclusion == "failure")'
}

run_processed() {
  local run_id="$1"
  grep -qx "$run_id" "$STATE_FILE"
}

mark_processed() {
  local run_id="$1"
  printf '%s\n' "$run_id" >>"$STATE_FILE"
}

prepare_worktree() {
  local branch="$1"
  local safe_branch
  local worktree

  safe_branch="$(sanitize_branch "$branch")"
  worktree="$WORKTREE_DIR/$safe_branch"

  git -C "$ROOT" fetch origin "$branch" --prune

  if [[ -d "$worktree/.git" ]]; then
    git -C "$worktree" fetch origin "$branch" --prune
    git -C "$worktree" checkout --detach "origin/$branch"
  else
    mkdir -p "$WORKTREE_DIR"
    git -C "$ROOT" worktree add --detach "$worktree" "origin/$branch"
  fi

  # This worktree lives under .ci-autofix and is owned by the automation loop.
  git -C "$worktree" reset --hard "origin/$branch"
  git -C "$worktree" clean -fd

  printf '%s\n' "$worktree"
}

ensure_frontend_deps() {
  local worktree="$1"

  if [[ ! -d "$worktree/log-analyzer/node_modules" ]]; then
    log "Installing frontend dependencies in automation worktree."
    (
      cd "$worktree/log-analyzer"
      npm ci
    )
  fi
}

download_run_log() {
  local repo="$1"
  local run_id="$2"
  local out="$LOG_DIR/$run_id.log"

  gh run view "$run_id" --repo "$repo" --log-failed >"$out" || true
  log "Failed job log saved: $out"
}

apply_mechanical_fixes() {
  local worktree="$1"

  log "Running mechanical fixers."
  ensure_frontend_deps "$worktree" || return

  (
    cd "$worktree/log-analyzer"
    npm run lint:fix
    npx prettier --write "../.github/workflows/*.{yml,yaml}" "../.github/actions/**/*.{yml,yaml}" || true
  ) || return

  (
    cd "$worktree/log-analyzer/src-tauri"
    CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo fmt
  ) || return
}

validate_changes() {
  local worktree="$1"

  case "$VALIDATE_MODE" in
    none)
      log "Skipping validation because CI_AUTOFIX_VALIDATE=none."
      ;;
    quick)
      log "Running quick validation."
      (
        cd "$worktree/log-analyzer"
        npm run lint
        npm run type-check
      ) || return
      (
        cd "$worktree/log-analyzer/src-tauri"
        CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo fmt -- --check
        CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo clippy --all-features --all-targets -- -D warnings
      ) || return
      ;;
    full)
      log "Running full local CI validation."
      bash "$worktree/scripts/validate-ci.sh" || return
      ;;
    *)
      die "Unknown CI_AUTOFIX_VALIDATE=$VALIDATE_MODE. Use quick, full, or none."
      ;;
  esac
}

commit_and_optionally_push() {
  local worktree="$1"
  local run_id="$2"
  local branch="$3"

  if [[ -z "$(git -C "$worktree" status --porcelain)" ]]; then
    log "No mechanical fixes produced changes."
    return 2
  fi

  git -C "$worktree" add -A || return 1
  git -C "$worktree" commit -m "chore: auto-fix CI mechanical issues" -m "Triggered by failed GitHub Actions run $run_id." || return 1

  if [[ "$PUSH_CHANGES" == "1" ]]; then
    git -C "$worktree" push origin "HEAD:$branch" || return 1
    log "Pushed auto-fix commit to $branch."
  else
    log "Created local auto-fix commit in $worktree. Set CI_AUTOFIX_PUSH=1 to push automatically."
  fi
}

ci_state_for_sha() {
  local repo="$1"
  local sha="$2"
  local saw_matching_run=0
  local saw_pending=0
  local saw_failure=0
  local saw_success=0
  local run_json workflow status conclusion

  while IFS= read -r run_json; do
    [[ -n "$run_json" ]] || continue

    workflow="$(jq -r '.workflowName' <<<"$run_json")"
    workflow_wanted "$workflow" || continue

    saw_matching_run=1
    status="$(jq -r '.status' <<<"$run_json")"
    conclusion="$(jq -r '.conclusion' <<<"$run_json")"

    if [[ "$status" != "completed" ]]; then
      saw_pending=1
    elif [[ "$conclusion" == "success" ]]; then
      saw_success=1
    else
      saw_failure=1
    fi
  done < <(
    gh run list \
      --repo "$repo" \
      --commit "$sha" \
      --limit 50 \
      --json databaseId,workflowName,status,conclusion,url,createdAt
  )

  if [[ "$saw_matching_run" == "0" || "$saw_pending" == "1" ]]; then
    printf 'pending\n'
  elif [[ "$saw_failure" == "1" ]]; then
    printf 'failure\n'
  elif [[ "$saw_success" == "1" ]]; then
    printf 'success\n'
  else
    printf 'pending\n'
  fi
}

wait_for_pushed_ci() {
  local repo="$1"
  local sha="$2"
  local state

  [[ "$PUSH_CHANGES" == "1" && "$WAIT_FOR_CI" == "1" ]] || return 0

  log "Waiting for CI result for pushed commit $sha."

  while true; do
    state="$(ci_state_for_sha "$repo" "$sha")"

    case "$state" in
      success)
        log "CI passed for pushed commit $sha."
        return 0
        ;;
      failure)
        log "CI failed again for pushed commit $sha; the loop will process the new failed run."
        return 1
        ;;
      pending)
        sleep "$STATUS_INTERVAL_SECONDS"
        ;;
      *)
        die "Unknown CI state: $state"
        ;;
    esac
  done
}

handle_run() {
  local repo="$1"
  local run_json="$2"
  local run_id workflow branch sha url worktree pushed_sha commit_status

  run_id="$(jq -r '.databaseId' <<<"$run_json")"
  workflow="$(jq -r '.workflowName' <<<"$run_json")"
  branch="$(jq -r '.headBranch' <<<"$run_json")"
  sha="$(jq -r '.headSha' <<<"$run_json")"
  url="$(jq -r '.url' <<<"$run_json")"

  [[ -n "$branch" && "$branch" != "null" ]] || {
    log "Skipping run $run_id because it has no branch."
    mark_processed "$run_id"
    return 0
  }

  workflow_wanted "$workflow" || return 0
  run_processed "$run_id" && return 0

  log "Handling failed run $run_id ($workflow): $url"
  download_run_log "$repo" "$run_id"
  worktree="$(prepare_worktree "$branch")"

  if [[ "$(git -C "$worktree" rev-parse HEAD)" != "$sha" ]]; then
    log "Branch moved since failed run; continuing on latest origin/$branch."
  fi

  if ! apply_mechanical_fixes "$worktree"; then
    log "Mechanical fixer failed for run $run_id."
    mark_processed "$run_id"
    return 1
  fi

  if ! validate_changes "$worktree"; then
    log "Validation failed after mechanical fixes for run $run_id; not pushing."
    mark_processed "$run_id"
    return 1
  fi

  commit_status=0
  commit_and_optionally_push "$worktree" "$run_id" "$branch" || commit_status=$?
  if [[ "$commit_status" == "2" ]]; then
    mark_processed "$run_id"
    return 0
  fi
  if [[ "$commit_status" != "0" ]]; then
    log "Commit or push failed for run $run_id; leaving it unprocessed for retry."
    return 1
  fi

  pushed_sha="$(git -C "$worktree" rev-parse HEAD)"
  mark_processed "$run_id"
  wait_for_pushed_ci "$repo" "$pushed_sha" || return 1
}

main() {
  ensure_ready

  local repo run_json
  repo="$(resolve_repo)"
  log "Monitoring $repo workflows: $WORKFLOWS"
  log "Poll interval: ${INTERVAL_SECONDS}s; CI wait interval: ${STATUS_INTERVAL_SECONDS}s"
  log "Validation: $VALIDATE_MODE; push: $PUSH_CHANGES; wait-for-ci: $WAIT_FOR_CI"

  while true; do
    while IFS= read -r run_json; do
      [[ -n "$run_json" ]] || continue
      handle_run "$repo" "$run_json" || true
    done < <(latest_failed_run_json "$repo")

    sleep "$INTERVAL_SECONDS"
  done
}

main "$@"
