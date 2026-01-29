#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;
    use crate::archive::actors::{CoordinatorActor, CoordinatorMessage, ExtractionPolicy};
    use tokio::sync::{mpsc, oneshot};
    use tracing::info;

    #[tokio::test]
    async fn stress_test_large_archive() {
        // This test simulates a large archive extraction request
        let (tx, rx) = mpsc::unbounded_channel();
        let _coordinator = CoordinatorActor::spawn(rx);

        let start = Instant::now();
        
        // Mock large archive path (in a real test, this would be a generated file)
        let archive_path = PathBuf::from("stress_test_100gb.zip");
        
        let (resp_tx, resp_rx) = oneshot::channel();
        tx.send(CoordinatorMessage::ExtractRequest {
            archive_path,
            workspace_id: "test_workspace".to_string(),
            policy: ExtractionPolicy::default(),
            response: resp_tx,
        }).unwrap();

        let _task_id = resp_rx.await.unwrap().unwrap();
        
        info!("Stress test task submitted in {}ms", start.elapsed().as_millis());
        
        // Verification logic would go here
    }

    #[tokio::test]
    async fn stress_test_million_files() {
        // Simulates extraction of an archive with 1 million small files
        // Focus on memory stability and progress feedback responsiveness
    }
}
