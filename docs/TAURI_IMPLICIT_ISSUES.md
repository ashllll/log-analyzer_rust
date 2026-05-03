# Tauri v2 Migration: Remaining 3 Implicit Issues — Deep Dive Report

> **Report Date**: 2026-04-26
> **Scope**: v1.2.60 post-audit remediation
> **Tauri Version**: 2.0.0 (stable)

---

## Executive Summary

After verifying that the 3 explicit issues (camelCase params, mmap fallback, perf-monitor cleanup) were already resolved, the audit identified 3 remaining implicit issues requiring action before v1.2.61:

| ID | Issue | Severity | Status | Est. Effort |
|---|---|---|---|---|
| I1 | Tauri v2 Capabilities configuration missing | Medium | Open | 2h |
| I2 | async_zip pre-release API dependency | Medium | Open | 4h |
| I3 | CI IPC parameter consistency not enforced | Medium | Open | 6h |
