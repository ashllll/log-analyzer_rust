# LogAnalyzer Apple Calm Instrument Panel PRD

- **Status**: Implemented and verified
- **Owner**: Frontend
- **Design source**: `log-analyzer/src/prototypes/frontend-redesign-prototype.html?variant=A`
- **Technical plan**: `docs/design/frontend-redesign-migration-plan.md`
- **Baseline commit**: `676af52fd3d98bbc04a73c32d3f04477f0417679`

## 1. Problem

The current desktop UI is functionally complete but visually fragmented: Zinc/Teal/Emerald tokens compete for meaning, frequent navigation and list surfaces carry decorative motion, overlays implement their own focus and animation behavior, and the core Search workspace does not yet provide the spatial hierarchy or direct manipulation expected from a polished macOS application.

## 2. Goal

Ship the approved Variant A as the production React UI while preserving all existing log import, search, pagination, export, keyword, task, settings, Tauri Event, and IPC behavior. The result should feel calm, immediate, legible, and physically coherent under both light and dark appearance.

## 3. Users and jobs

- Investigators need to scan dense log results for hours without visual fatigue.
- Operators need immediate navigation and reliable status feedback during long-running tasks.
- Developers need reusable keyword groups and precise filters without losing search context.
- All users need predictable keyboard focus, reduced-motion behavior, and readable high-contrast states.

## 4. Product requirements

### PR-1 — Application shell

- Render a 224px structural sidebar and a 52px workspace toolbar.
- Use Apple System Blue for action/focus/selection and independent system semantic colors for status.
- Navigation and keyboard route changes are immediate; no page enter/exit motion.
- Preserve routes, skip link, lazy loading, ErrorBoundary, active workspace status, and existing navigation test IDs.

### PR-2 — Appearance

- Provide System, Light, and Dark appearance modes.
- System mode reacts to OS changes while the app is open.
- Persist the explicit user choice locally.
- Components consume semantic tokens rather than hard-coded theme colors.
- Support reduced motion, reduced transparency, and increased contrast independently.

### PR-3 — UI foundations

- Button, Input, Card, NavItem, Skeleton, EmptyState, focus ring, status badge, and section surfaces share one visual language.
- Pointer press feedback uses `scale(.97)` with a 120–160ms strong ease-out.
- No `transition-all`, `scale(0)`, UI `ease-in`, decorative glow, or high-frequency stagger.
- System font is primary; log content, paths, queries, and timestamps use the platform monospace stack.

### PR-4 — Search cockpit

- First row: query, Keyword Groups, Export, Search.
- Second row: Level, Time range, File pattern, Reset.
- Keep query parsing, orchestration, pagination, event projection, export, and virtual scrolling behavior unchanged.
- Keyword Groups opens an origin-aware popover; Escape/outside click closes it and focus returns to the trigger.
- Log details open in a right inspector, default 420px, directly resizable between 320px and 640px with pointer capture.
- Log rows never receive enter/stagger animation.

### PR-5 — Supporting pages

- Workspaces uses the approved card hierarchy without changing import, switch, watch, refresh, or delete behavior.
- Keywords uses a group-to-rules master/detail hierarchy without changing persistence behavior.
- Tasks uses a compact operational table/list; only active progress/status indicators move.
- Settings uses grouped navigation and a sticky save surface without changing validation or config persistence.

### PR-6 — Overlays and feedback

- Keyword and file-filter dialogs share one dialog module for scrim, Escape, focus trap, focus return, and motion.
- Modal entry is 240ms and exit 180ms; background is dimmed and subtly pushed back.
- Popover entry is 180ms and exit 130ms from its trigger origin.
- Reduced motion removes translation/scale while preserving brief opacity/color feedback.

## 5. Non-goals

- No Rust, IPC, Tauri Event, query semantic, storage, archive, or task-lifecycle changes.
- No production route for the prototype or Variant B/C.
- No new UI/motion dependency unless an existing platform primitive cannot satisfy an approved requirement.
- No celebratory, parallax, rubber-band, animated log-row, or page-transition effects.

## 6. Test seams

Tests observe behavior through these interfaces:

1. Appearance provider: mode selection, system resolution, persistence, document theme.
2. UI primitives: semantic variants and native interaction/accessibility behavior.
3. Dialog/popover: open/close, Escape, outside click, focus containment and restoration.
4. Inspector resize: pointer tracking and 320–640px clamping.
5. Page flows: existing user-visible Workspaces, Search, Keywords, Tasks, and Settings actions.

Tests do not assert implementation-only class lists or internal state.

## 7. Acceptance criteria

- All six product requirements are visible in the production React route tree.
- Existing business tests pass without weakened assertions.
- New tests cover the five agreed seams.
- `npm run type-check`, `npm run lint`, `npm test -- --runInBand`, and `npm run build` pass.
- Browser verification passes at 1280×720 and 1440×900 in Light and Dark.
- Keyboard navigation, Escape behavior, focus restoration, 200% text zoom, reduced motion, and reduced transparency are manually verified.
- Motion review returns Approve and general code review has no blocking Standards or Spec finding.

## 8. Rollout

Implement in vertical slices: appearance/foundation, shell, Search, supporting pages, overlays/feedback, cleanup. Each slice remains buildable and testable. The semantic Tailwind class interface remains stable during migration so a page can be rolled back independently.

## 9. Implementation record

- Completed the seven migration phases on 2026-07-20 without changing Rust, IPC, search semantics, storage, or task lifecycle behavior.
- Added automated seams for appearance, UI foundations, dialog/popover focus behavior, reduced motion, inspector pointer/keyboard resizing, log-row focus restoration, and page flows.
- Verified 441 tests passing (17 skipped by the existing suite), TypeScript, ESLint, production build, and `git diff --check`.
- Verified Variant A at 1280×720 Light and 1440×900 Dark with no horizontal overflow and exact 224px sidebar, 52px toolbar, and 420px inspector dimensions.
- Launched the production React route in Tauri dev; backend initialization and state synchronization completed successfully.
- Motion review and the final Standards and Spec reviews returned Approve with no blocking findings.
