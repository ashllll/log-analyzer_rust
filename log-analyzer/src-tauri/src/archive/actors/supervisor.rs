use super::messages::{ExtractorMessage, SupervisorMessage};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

/// Actor handle for supervision
pub struct ActorHandle {
    pub id: String,
    pub sender: mpsc::UnboundedSender<ExtractorMessage>,
    pub restart_count: usize,
    pub last_seen: std::time::Instant,
}

/// The Supervisor Actor monitors health and handles restarts
pub struct SupervisorActor {
    receiver: mpsc::UnboundedReceiver<SupervisorMessage>,
    actors: Arc<DashMap<String, ActorHandle>>,
    max_restarts: usize,
}

impl SupervisorActor {
    pub fn spawn(
        receiver: mpsc::UnboundedReceiver<SupervisorMessage>,
        actors: Arc<DashMap<String, ActorHandle>>,
    ) -> tokio::task::JoinHandle<()> {
        let mut actor = Self {
            receiver,
            actors,
            max_restarts: 3,
        };

        tokio::spawn(async move {
            actor.run().await;
        })
    }

    async fn run(&mut self) {
        info!("Supervisor Actor started");
        let mut health_interval = interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        SupervisorMessage::WatchActor { actor_id } => {
                            debug!("Supervisor watching actor: {}", actor_id);
                        }
                        SupervisorMessage::ActorPanicked { actor_id, reason } => {
                            warn!(actor_id = %actor_id, reason = %reason, "Actor panicked, evaluating restart");
                            let _ = self.handle_restart(&actor_id).await;
                        }
                    }
                }
                _ = health_interval.tick() => {
                    self.check_health().await;
                }
            }
        }
    }

    async fn check_health(&self) {
        for mut actor in self.actors.iter_mut() {
            let (tx, rx) = oneshot::channel();

            // Try pinging the actor with a timeout
            if actor
                .sender
                .send(ExtractorMessage::Ping { response: tx })
                .is_err()
            {
                warn!(
                    "Failed to send ping to actor {}, it might be dead",
                    actor.id
                );
                continue;
            }

            match timeout(Duration::from_secs(2), rx).await {
                Ok(Ok(())) => {
                    actor.last_seen = std::time::Instant::now();
                }
                _ => {
                    let elapsed = actor.last_seen.elapsed();
                    if elapsed > Duration::from_secs(15) {
                        warn!(actor_id = %actor.id, "Actor heartbeat timeout ({}s), initiating restart", elapsed.as_secs());
                        // Trigger restart logic (simplified for this example)
                    }
                }
            }
        }
    }

    async fn handle_restart(&mut self, actor_id: &str) -> Result<(), String> {
        if let Some(mut actor) = self.actors.get_mut(actor_id) {
            if actor.restart_count < self.max_restarts {
                actor.restart_count += 1;
                info!(actor_id = %actor_id, retry = actor.restart_count, "Restarting actor");
                // Actual restart logic would involve re-spawning the task and updating the handle
                Ok(())
            } else {
                error!(actor_id = %actor_id, "Max restarts reached, giving up");
                Err("Max restarts reached".to_string())
            }
        } else {
            Err("Actor not found".to_string())
        }
    }
}
