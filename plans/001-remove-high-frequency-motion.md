# 001 — Remove high-frequency motion and centralize motion tokens

- **Status**: TODO
- **Commit**: 676af52
- **Severity**: HIGH
- **Category**: Purpose, frequency, cohesion, performance
- **Estimated scope**: 8 files

## Problem

`PageTransition.tsx` uses `AnimatePresence mode="wait"`; `NavItem.tsx` uses a shared layout spring; Workspaces, Keywords, and Tasks stagger frequently revisited lists; Button and Card contain `transition-all`. These motions delay core navigation or animate information-dense surfaces users inspect repeatedly.

## Target

- Route and navigation state change instantly.
- Lists render without enter/stagger motion.
- Press feedback remains: `transform: scale(0.97)` with `transition: transform 120ms cubic-bezier(0.23, 1, 0.32, 1)`.
- Shared tokens live in `index.css`: `--ease-out-ui`, `--ease-in-out-ui`, `--ease-drawer`.
- No `transition-all`.

## Steps

1. Add exact motion and duration tokens to `src/index.css`.
2. Remove AnimatePresence/motion wrappers from PageTransition, Sidebar/NavItem, Workspaces, Keywords, and Tasks.
3. Convert Button to native button and explicit CSS properties.
4. Replace Card and task progress `transition-all` with named properties.

## Boundaries

- Do not alter routes, event handlers, business hooks, list ordering, or test IDs.
- Do not add a motion dependency.

## Verification

- Run type-check, lint, Jest, and build.
- Switch routes repeatedly with mouse and keyboard: content must change immediately.
- Enable reduced motion: color/opacity feedback remains; no layout movement appears.

