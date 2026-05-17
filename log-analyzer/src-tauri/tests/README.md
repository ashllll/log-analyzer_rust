# Rust Test Layout

This project keeps integration and end-to-end tests under `src-tauri/tests`.
Unit tests and property tests that need private module access stay next to the
source file that owns the behavior.

Use this split when adding tests:

- `src-tauri/tests`: public command, storage, archive, and cross-module flows.
- `src-tauri/src/**`: focused unit/property tests for private helpers and module-local invariants.
