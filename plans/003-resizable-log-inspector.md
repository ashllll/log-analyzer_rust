# 003 — Add a directly manipulated log inspector

- **Status**: TODO
- **Commit**: 676af52
- **Severity**: MEDIUM
- **Category**: Spatial consistency, direct manipulation, performance
- **Estimated scope**: 3 files

## Problem

`LogDetailPanel.tsx` is fixed at 450px and only has an entry keyframe. It cannot be resized, has no symmetric exit, and does not preserve the spatial relationship between the selected row and its details.

## Target

- Default 420px, clamped to 320–640px.
- Resize handle uses pointer capture and updates width 1:1 without transition.
- Panel entrance uses transform/opacity for 260ms with `cubic-bezier(0.32, 0.72, 0, 1)`; exit uses 180ms along the same path.
- Reduced motion uses opacity only.

## Steps

1. Write a behavior test for pointer capture and width clamping.
2. Implement a local `useResizableInspector` hook behind a small handle-props/width interface.
3. Integrate the handle and spatial motion into LogDetailPanel.
4. Restore focus to the selected log row when the inspector closes.

## Boundaries

- Do not change log selection, query highlighting, virtualization, or copy behavior.
- Do not persist inspector width in this slice.

## Verification

- Drag beyond both limits and confirm 320/640px clamping.
- Drag quickly outside the handle and confirm tracking continues.
- Confirm virtual scrolling remains smooth while the inspector is open.

