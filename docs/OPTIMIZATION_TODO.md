# Optimization TODO

This checklist is the current implementation plan for aligning the codebase, public docs, and CI/CD with the real business path.

## Sources of truth

- Tauri distribution pipeline guide: [GitHub](https://tauri.app/distribute/pipelines/github/)
- Tauri prerequisites: [Start / Prerequisites](https://v2.tauri.app/start/prerequisites/)
- Vite guide: [Getting Started](https://vite.dev/guide/)
- React upgrade guide: [React 19 Upgrade Guide](https://react.dev/blog/2024/04/25/react-19-upgrade-guide)
- GitHub Actions workflow dispatch API: [Create a workflow dispatch event](https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#create-a-workflow-dispatch-event)

## P0: Truthful runtime and docs

- [x] Align frontend settings schema with backend config schema.
- [x] Make runtime search/cache behavior read persisted config values.
- [x] Fix cache metrics so dashboard values are based on real snapshots.
- [x] Update public environment requirements to Node 22.12+ and Tauri 2 docs.
- [ ] Rewrite remaining public README claims that still describe aspirational Tantivy / FTS5 / plugin / OpenTelemetry capabilities as shipped behavior.

## P0: Search path convergence

- [x] Confirm the active interactive path is `search_logs + fetch_search_page`.
- [ ] Mark `search_logs_paged` and `register_search_session` as legacy in authoritative docs.
- [ ] Add regression tests that exercise the active search path instead of legacy session helpers.
- [ ] Decide whether the interactive path should stay CAS scan based or switch to Tantivy / FTS5 for user-facing search.

## P0: CI/CD release train

- [x] Add a Linux desktop smoke build to CI.
- [x] Add a desktop smoke build to PR validation.
- [x] Make automated version bump update all workspace crate versions, not just the root package.
- [x] Replace implicit tag-push chaining with an explicit `workflow_dispatch` call to `release.yml`.
- [ ] Validate the first end-to-end release run in GitHub Actions and confirm required repository permissions / signing secrets are configured.

## P1: Measured performance baseline

- [ ] Add benchmark fixtures for large import, search latency, and damaged archives.
- [ ] Replace fixed public performance numbers with benchmark-backed release notes or CI artifacts.
- [ ] Add a release checklist item that links the current benchmark run.

## P1: Dependency and architecture cleanup

- [ ] Audit unused or not-yet-activated dependencies such as `libloading` and OpenTelemetry-related crates.
- [ ] Remove or feature-gate dependencies that are not part of the shipped path.
- [ ] Split oversized modules only after tests and ownership boundaries are in place.
