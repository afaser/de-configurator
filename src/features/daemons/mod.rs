pub mod config;
pub mod manager;
pub mod notifier;

use std::sync::Arc;
use std::collections::HashMap;
use kdl::KdlDocument;
use crate::core::event_bus::{EventBus, SystemEvent, DaemonAction, EventContext, Initiator};
use config::DaemonsConfig;
use manager::DaemonManager;
use notifier::DaemonNotificationService;

pub struct DaemonsFeature;

impl DaemonsFeature {
    pub async fn start(doc: &KdlDocument, event_bus: EventBus) -> Result<(), String> {
        let config = DaemonsConfig::parse_from_root_doc(doc)?;
        let config = Arc::new(config);

        tokio::spawn(DaemonNotificationService::run(event_bus.subscribe()));

        let app = DaemonApp::new(config.clone(), event_bus.clone());
        tokio::spawn(app.run());

        for daemon in &config.daemons {
            if daemon.autostart {
                let bus = event_bus.clone();
                let daemon_id = daemon.id.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    bus.publish(SystemEvent::RequestDaemonControl {
                        daemon_id,
                        action: DaemonAction::On,
                        context: EventContext {
                            initiator: Initiator::Direct,
                            silent: true,
                        },
                    });
                });
            }
        }

        Ok(())
    }
}

struct DaemonApp {
    config: Arc<DaemonsConfig>,
    manager: DaemonManager,
    children: HashMap<String, tokio::process::Child>,
    event_bus: EventBus,
}

impl DaemonApp {
    fn new(config: Arc<DaemonsConfig>, event_bus: EventBus) -> Self {
        Self {
            config,
            manager: DaemonManager::new(),
            children: HashMap::new(),
            event_bus,
        }
    }

    async fn run(mut self) {
        println!("Фича Daemons инициализирована. Зарегистрировано демонов: {}", self.config.daemons.len());
        for daemon in &self.config.daemons {
            println!("  {:18} (autostart={})", daemon.id, daemon.autostart);
        }

        let mut rx = self.event_bus.subscribe();
        while let Ok(event) = rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: SystemEvent) {
        match event {
            SystemEvent::RequestDaemonControl { daemon_id, action, context } => {
                let target_daemon = self.config.daemons.iter().find(|d| d.id == daemon_id).cloned();

                if let Some(daemon) = target_daemon {
                    let is_running = self.is_daemon_running(&daemon).await;

                    match action {
                        DaemonAction::On => {
                            if is_running {
                                if !context.silent {
                                    println!("Запрос на запуск '{}' проигнорирован, процесс уже запущен", daemon.id);
                                }
                                return;
                            }
                            self.start_daemon_process(&daemon, context).await;
                        }
                        DaemonAction::Off => {
                            if !is_running {
                                if !context.silent {
                                    println!("Запрос на остановку '{}' проигнорирован, процесс не запущен", daemon.id);
                                }
                                return;
                            }
                            self.stop_daemon_process(&daemon, context).await;
                        }
                        DaemonAction::Toggle => {
                            if is_running {
                                self.stop_daemon_process(&daemon, context).await;
                            } else {
                                self.start_daemon_process(&daemon, context).await;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    async fn is_daemon_running(&mut self, daemon: &config::DaemonConfig) -> bool {
        let mut is_alive = false;
        if let Some(child) = self.children.get_mut(&daemon.id) {
            if let Ok(None) = child.try_wait() {
                is_alive = true;
            }
        }
        
        if is_alive {
            return true;
        } else {
            self.children.remove(&daemon.id);
        }

        let process_name = std::path::Path::new(&daemon.start_cmd[0])
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&daemon.id);

        self.manager.is_running(process_name).await
    }

    async fn start_daemon_process(&mut self, daemon: &config::DaemonConfig, context: EventContext) {
        match self.manager.start_daemon(&daemon.start_cmd) {
            Ok(child) => {
                self.children.insert(daemon.id.clone(), child);
                
                self.event_bus.publish(SystemEvent::DaemonStateChanged {
                    daemon_id: daemon.id.clone(),
                    display_name: daemon.display_name.clone(),
                    is_running: true,
                    context,
                });
            }
            Err(e) => {
                self.event_bus.publish(SystemEvent::DaemonStateTransitionFailed {
                    daemon_id: daemon.id.clone(),
                    display_name: daemon.display_name.clone(),
                    action: DaemonAction::On,
                    error: e,
                    context,
                });
            }
        }
    }

    async fn stop_daemon_process(&mut self, daemon: &config::DaemonConfig, context: EventContext) {
        if let Some(mut child) = self.children.remove(&daemon.id) {
            let _ = child.kill().await;
        }

        let process_name = std::path::Path::new(&daemon.start_cmd[0])
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&daemon.id);

        let pids = self.manager.get_pids(process_name).await;
        if !pids.is_empty() {
            if let Err(e) = self.manager.kill_pids(&pids).await {
                self.event_bus.publish(SystemEvent::DaemonStateTransitionFailed {
                    daemon_id: daemon.id.clone(),
                    display_name: daemon.display_name.clone(),
                    action: DaemonAction::Off,
                    error: e,
                    context,
                });
                return;
            }
        }

        if let Some(ref stop_cmd) = daemon.stop_cmd {
            let mut cmd = tokio::process::Command::new(&stop_cmd[0]);
            cmd.args(&stop_cmd[1..]);
            let _ = cmd.status().await;
        }

        self.event_bus.publish(SystemEvent::DaemonStateChanged {
            daemon_id: daemon.id.clone(),
            display_name: daemon.display_name.clone(),
            is_running: false,
            context,
        });
    }
}
