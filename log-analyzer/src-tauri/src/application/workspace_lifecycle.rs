//! Serialize workspace creation, import teardown and deletion. Long-lived
//! search readers additionally use the CAS activity lease.
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
pub struct WorkspaceLifecycle {
    pub(crate) gate: Arc<Mutex<()>>,
    generation: AtomicU64,
    deleting: AtomicBool,
}

impl WorkspaceLifecycle {
    pub fn is_deleting(&self) -> bool {
        self.deleting.load(Ordering::SeqCst)
    }

    pub async fn enter(self: &Arc<Self>) -> Result<OwnedMutexGuard<()>, String> {
        let generation = self.generation.load(Ordering::SeqCst);
        if self.deleting.load(Ordering::SeqCst) {
            return Err("Workspace is being deleted".into());
        }
        let permit = self.gate.clone().lock_owned().await;
        if self.deleting.load(Ordering::SeqCst)
            || self.generation.load(Ordering::SeqCst) != generation
        {
            return Err("Workspace was deleted while this operation was waiting".into());
        }
        Ok(permit)
    }

    pub async fn begin_delete(self: &Arc<Self>) -> WorkspaceDeletionGuard {
        let permit = self.gate.clone().lock_owned().await;
        self.deleting.store(true, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
        WorkspaceDeletionGuard {
            lifecycle: self.clone(),
            _permit: permit,
        }
    }
}

pub struct WorkspaceDeletionGuard {
    lifecycle: Arc<WorkspaceLifecycle>,
    _permit: OwnedMutexGuard<()>,
}

impl Drop for WorkspaceDeletionGuard {
    fn drop(&mut self) {
        self.lifecycle.deleting.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;

    #[tokio::test]
    async fn deletion_waits_for_import_and_rejects_queued_stale_work() {
        let lifecycle = Arc::new(WorkspaceLifecycle::default());
        let import = lifecycle.enter().await.unwrap();
        let deletion = lifecycle.begin_delete();
        tokio::pin!(deletion);
        assert!(deletion.as_mut().now_or_never().is_none());
        let stale = lifecycle.enter();
        tokio::pin!(stale);
        assert!(stale.as_mut().now_or_never().is_none());
        drop(import);
        let deletion = deletion.await;
        assert!(lifecycle.enter().await.is_err());
        drop(deletion);
        assert!(stale.await.is_err());
        // A deliberate new operation can reuse an ID after deletion finishes.
        assert!(lifecycle.enter().await.is_ok());
    }
}
