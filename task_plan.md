# Task Plan: Code Review and Claude Code Repair Loop

## Goal
Review the project for actionable code issues, delegate scoped fixes to Claude Code, then review the diff and run validation before reporting back.

## Current Phase
All planned tasks complete

## Phases

### Phase 1: Discovery and Review
- [x] Capture user intent and repo constraints.
- [x] Inspect project structure and relevant validation entry points.
- [x] Identify actionable defects or high-confidence maintenance fixes.
- [x] Document findings in findings.md.
- **Status:** complete

### Phase 2: Delegate Scoped Fixes
- [x] Choose a narrowly scoped repair task with clear acceptance criteria.
- [x] Dispatch implementation to Claude Code without commit, push, or deploy.
- [x] Capture Claude Code result in progress.md.
- **Status:** complete

### Phase 3: Codex Review
- [x] Inspect the real working-tree diff.
- [x] Reject or correct unrelated churn, broad refactors, or behavioral bugs.
- [x] Update planning files with review outcome.
- **Status:** complete

### Phase 4: Validation
- [x] Run focused validation relevant to the changed files.
- [x] Record command results in progress.md.
- [x] Address validation failures if feasible.
- **Status:** complete

### Phase 5: Delivery
- [x] Summarize findings, changes, validation, and residual risk.
- [x] Note pre-existing unrelated worktree changes.
- **Status:** complete

### Phase 6: Architecture Review Discovery
- [x] Read architecture review skill language and report format.
- [x] Read project domain context and architecture notes.
- [x] Inspect high-friction backend and frontend flows.
- [x] Select deepening candidates.
- **Status:** complete

### Phase 7: Architecture Report
- [x] Write HTML report to OS temp directory.
- [x] Open the report for the user.
- [x] Record report path and top recommendation.
- **Status:** complete

### Phase 8: Repair Slice 1 - Search Session Ownership
- [x] Define scoped acceptance criteria from architecture review.
- [x] Delegate SearchSession module implementation to Claude Code.
- [x] Review diff for ownership consistency and scope.
- [x] Run Rust validation for affected backend modules.
- **Status:** complete

### Phase 9: Repair Slice 2 - Import Orchestration
- [x] Define scoped acceptance criteria.
- [x] Delegate or implement ImportPipeline module.
- [x] Review diff and validate import-related tests.
- **Status:** complete

### Phase 10: Repair Slice 3 - Workspace Runtime Internals
- [x] Define scoped acceptance criteria.
- [x] Split WorkspaceServiceImpl internals while preserving external interface.
- [x] Review diff and validate workspace/search/watch paths.
- **Status:** complete

### Phase 11: Repair Slice 4 - Frontend Event Projection
- [x] Define scoped acceptance criteria.
- [x] Deepen event projection module behind a small mount/unmount interface.
- [x] Review diff and validate frontend event tests.
- **Status:** complete

### Phase 12: Repair Slice 5 - Frontend Config Sync
- [x] Define scoped acceptance criteria.
- [x] Deepen config sync if still worthwhile after prior slices.
- [x] Review diff and validate config tests.
- **Status:** complete

### Phase 13: CI Autofix Loop Cleanup
- [x] Inspect remaining CI autofix loop worktree changes.
- [x] Validate shell syntax and shellcheck coverage.
- [x] Commit the scoped automation helper changes.
- **Status:** complete

## Key Questions
1. Which issues are high-confidence enough to fix without more product context?
2. Which validation commands give useful coverage without running the entire CI unnecessarily?
3. Did Claude Code keep changes scoped and avoid user-owned worktree changes?
4. Which modules are shallow under the deletion test?
5. Which seams are real because there are multiple adapters, and which are hypothetical?
6. Where would deeper modules improve locality and leverage for tests?
7. Can each repair slice preserve public Tauri commands and frontend IPC names?
8. Which slices should remain speculative after earlier repairs reduce pressure?

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Use planning-with-files | User asked to use planning, and this task spans review, delegation, verification, and reporting. |
| Use Claude Code only after a scoped issue is identified | The review loop requires Codex to provide clear acceptance criteria before delegation. |
| Use CodeGraph directly for architecture exploration | Project instructions require CodeGraph for structural questions and discourage delegating exploration. |
| Repair slices in recommendation order | The report ranked search session ownership first, then import orchestration, then internal/runtime/frontend cleanups. |

## Errors Encountered
| Error | Attempt | Resolution |
|-------|---------|------------|
| Initial Claude dispatch prompt used shell-interpreted backticks, causing accidental `npm` command substitutions from the prompt text. | 1 | Stopped the Claude process and will re-dispatch with shell-safe single-quoted prompt text. |

## Notes
- Pre-existing worktree changes must be preserved unless explicitly requested otherwise.
- Prefer CodeGraph for structural code questions and native `rg`/file reads for literal config/test details.

## Phase 14: GitHub Pages Documentation Site
- [x] Inventory the existing documentation, application architecture, screenshots, and CI conventions.
- [x] Select VitePress and define the site information architecture.
- [x] Implement the VitePress site, custom theme, content, and local npm scripts.
- [x] Add SHA-pinned GitHub Actions validation and Pages deployment workflows.
- [x] Review the implementation diff for correctness, scope, accessibility, and maintainability.
- [x] Build the site, check links/workflow invariants, and inspect the rendered result.
- **Status:** complete

### Documentation Site Acceptance Criteria
- The site has a polished responsive home page plus navigable user, architecture, development, CI/release, and operations sections.
- Existing long-lived Markdown and screenshots are reused or linked without deleting source documentation.
- The VitePress base path works at `https://ashllll.github.io/log-analyzer_rust/` and can be overridden for preview/custom-domain builds.
- Local scripts support development, production build, and preview from a clean install.
- Pull requests validate the docs build; pushes to `main` deploy through the official GitHub Pages artifact workflow.
- All third-party GitHub Actions are pinned to full commit SHAs, matching repository CI policy.
- The site respects reduced-motion and keyboard accessibility, and mobile navigation remains usable.
- Claude Code must not commit, push, deploy, or modify unrelated user-owned files.
