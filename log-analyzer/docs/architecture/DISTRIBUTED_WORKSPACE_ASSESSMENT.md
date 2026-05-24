# Distributed / Remote Workspace Assessment

> **Status:** Assessment Complete — Decision: DEFER
>
> **Date:** 2026-05-23
>
> **Author:** Architecture review, Phase 3

## 1. Current Workspace Architecture

Workspaces are currently **local-only** constructs managed via:

- **Storage:** `config.json` in Tauri's `app_config_dir()` — a JSON file storing `StoredWorkspaceConfig { id, name, path }` arrays.
- **Runtime state:** `RuntimeWorkspaceRepository` — in-memory cache of active workspace metadata (file counts, status, last-indexed timestamps).
- **Data layer:** All logs, CAS blobs, search indices, and metadata live on local disk under the workspace path.

```text
┌─────────────────────────────────────┐
│  Tauri Desktop App (single instance) │
│                                     │
│  config.json ← workspaces[]         │
│  ┌─────────────────────────────┐    │
│  │ RuntimeWorkspaceRepository  │    │
│  │  (in-memory, list/get/del)  │    │
│  └──────────┬──────────────────┘    │
│             │                       │
│  ┌──────────▼──────────────────┐    │
│  │ Workspace on local disk     │    │
│  │  ├── logs/                  │    │
│  │  ├── .cas/                  │    │
│  │  ├── .index/                │    │
│  │  └── metadata.db            │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

## 2. What "Distributed / Remote Workspace" Could Mean

Three interpretations, ordered by complexity:

### A. Shared Workspace Registry (轻量)
Multiple users/devices share a **central registry** of workspace metadata (name, path, status). Each instance still operates on its own local copies of the log files and indices.

### B. Remote Workspace Access (中等)
A single "server" instance hosts the workspace data. Other instances connect remotely to query/search the server's workspace. Requires a client/server transport layer (HTTP/gRPC/Tauri IPC tunneling).

### C. Full Distributed Sync (重量)
Multiple instances collaboratively index and search the same log corpus. Requires CRDT-based state sync, conflict resolution for metadata/index writes, and distributed search aggregation.

## 3. Technical Feasibility Assessment

| Dimension | Interpretation A | Interpretation B | Interpretation C |
|-----------|-----------------|-----------------|-----------------|
| **Effort estimate** | ~2-3 days | ~2-4 weeks | ~2-3 months |
| **Architecture impact** | New `WorkspaceRegistry` trait + HTTP adapter | Network transport + remote query protocol | Distributed consensus, CRDT indexes |
| **User-facing value** | Low (saves one `config.json` copy-paste) | Medium (team log analysis) | High (collaborative analysis at scale) |
| **Risk to existing code** | Minimal | Moderate (query API changes) | High (fundamental index format changes) |
| **Product-market fit** | Unclear — who needs a shared config without shared data? | Potential for DevOps/on-call teams | Requires enterprise-scale log corpus |

## 4. Dependencies & Blockers

- **A/B/C:** Server infrastructure (self-hosted or cloud). The project has no server component today.
- **B/C:** Authentication/authorization layer — accessing remote workspaces requires auth.
- **B/C:** Network transport — Tauri's IPC is local-only; needs HTTP/gRPC client integration.
- **C:** Conflict resolution for concurrent writes to `metadata.db` and search indices — non-trivial distributed systems problem.

## 5. Recommendation: DEFER

**Rationale:**

1. **No clear user demand.** The project's primary use case is single-user desktop log analysis. No issues/requests for remote/multi-user features have been filed.

2. **Architecture mismatch.** The Tauri desktop model is inherently local-first. Adding a server component significantly shifts the architectural boundary — it would make the app a **hybrid desktop-server** product rather than a pure desktop tool.

3. **Opportunity cost.** The remaining Phase 3 effort (P3-01 plugin handlers) is already complete. The engineering time that would go into distributed workspaces is better invested in:
   - Search performance optimization
   - Archive format support expansion
   - UI/UX improvements
   - Plugin system for custom log parsers

4. **If demand emerges**, the clean architecture established in Phase 1-2 makes it straightforward to:
   - Add a `RemoteWorkspaceRepository` implementing the existing `WorkspaceRepository` trait
   - Create a `RemoteLogSearcher` implementing `LogSearcher`
   - Add a small HTTP/gRPC adapter for network transport

   The domain layer requires **zero changes** — it's already abstracted behind traits.

## 6. Trigger for Re-evaluation

Revisit this assessment when:
- ≥3 users request shared/remote workspace features
- A team deployment use case is documented
- A prototype of the server component exists

## 7. Related Documents

- [Clean Architecture ADR](./ADR_CLEAN_ARCHITECTURE.md) — trait-based domain design
- [Workspace UseCase](../../src-tauri/src/application/workspace.rs) — current workspace operations
- [Workspace Domain Trait](../../src-tauri/crates/la-core/src/domain/workspace.rs) — `WorkspaceRepository` abstract interface
