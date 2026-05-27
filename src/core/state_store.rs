use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use crate::core::event_bus::SystemEvent;

#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub active_vpn_state: String,
    pub running_daemons: HashSet<String>,
}

#[derive(Clone)]
pub struct StateStore {
    state: Arc<RwLock<AppState>>,
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
        }
    }

    #[allow(dead_code)]
    pub async fn get_vpn_state(&self) -> String {
        self.state.read().await.active_vpn_state.clone()
    }

    #[allow(dead_code)]
    pub async fn is_daemon_running(&self, daemon_id: &str) -> bool {
        self.state.read().await.running_daemons.contains(daemon_id)
    }

    pub async fn update_vpn_state(&self, state_id: String) {
        self.state.write().await.active_vpn_state = state_id;
    }

    pub async fn add_daemon(&self, daemon_id: String) {
        self.state.write().await.running_daemons.insert(daemon_id);
    }

    pub async fn remove_daemon(&self, daemon_id: &str) {
        self.state.write().await.running_daemons.remove(daemon_id);
    }

    pub fn start_sync(&self, mut rx: broadcast::Receiver<SystemEvent>) {
        let store = self.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    SystemEvent::VpnStateChanged { state_id, .. } => {
                        store.update_vpn_state(state_id).await;
                    }
                    SystemEvent::DaemonStateChanged { daemon_id, is_running, .. } => {
                        if is_running {
                            store.add_daemon(daemon_id).await;
                        } else {
                            store.remove_daemon(&daemon_id).await;
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}
