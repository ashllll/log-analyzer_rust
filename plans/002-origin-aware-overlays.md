# 002 — Make overlays origin-aware and accessible

- **Status**: TODO
- **Commit**: 676af52
- **Severity**: MEDIUM
- **Category**: Physicality, origin, interruptibility, accessibility
- **Estimated scope**: 5 files

## Problem

FilterPalette, KeywordModal, and FileFilterSettings independently manage overlay behavior. FilterPalette enters in 100ms but has no coordinated exit; dialog focus and Escape logic is duplicated.

## Target

- Popover: origin-aware `translateY(-4px) scale(.96)` + opacity + blur, 180ms enter and 130ms exit with `cubic-bezier(0.23, 1, 0.32, 1)`.
- Modal: centered `translateY(8px) scale(.965)` + opacity, 240ms enter and 180ms exit with `cubic-bezier(0.32, 0.72, 0, 1)`.
- Reduced motion uses opacity only.
- Shared modules own Escape, outside click, focus trap, and focus restore.

## Steps

1. Add DialogSurface and PopoverSurface modules with small open/onClose interfaces.
2. Add behavior tests before migrating consumers.
3. Migrate FilterPalette, KeywordModal, and FileFilterSettings without changing form/business behavior.

## Boundaries

- Do not change keyword schemas, file-filter validation, save callbacks, or translations.
- Do not animate scale from zero or use keyframes for rapidly toggled state.

## Verification

- Exercise Escape, outside click, tab loop, and focus restoration.
- Inspect at 10% playback and verify popover origin and modal center origin.

