pub mod config;
pub mod manager;
pub mod notifier;

use std::sync::Arc;
use std::collections::HashMap;
use kdl::KdlDocument;
use crate::core::event_bus::{EventBus, SystemEvent, EventContext, Initiator};
use config::VpnConfig;
use manager::VpnManager;
use notifier::VpnNotificationService;

pub trait VpnState: Send + Sync {
    #[allow(dead_code)]
    fn state_id(&self) -> &str;

    #[allow(dead_code)]
    fn display_name(&self) -> &str;

    fn is_transitioning(&self) -> bool {
        false
    }
}

pub struct Disconnected;

impl VpnState for Disconnected {
    fn state_id(&self) -> &str {
        "off"
    }

    fn display_name(&self) -> &str {
        "VPN выключен"
    }
}

pub struct Connected {
    pub id: String,
    pub display_name: String,
}

impl VpnState for Connected {
    fn state_id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }
}

pub struct Transitioning {
    pub target_id: String,
    pub display_name: String,
}

impl VpnState for Transitioning {
    fn state_id(&self) -> &str {
        &self.target_id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn is_transitioning(&self) -> bool {
        true
    }
}

pub struct VpnFeature;

impl VpnFeature {
    pub async fn start(
        doc: &KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let vpn_config = VpnConfig::parse_from_root_doc(doc)?;
        let vpn_config = Arc::new(vpn_config);

        tokio::spawn(VpnNotificationService::run(event_bus.subscribe()));

        let app = VpnApp::new(vpn_config, event_bus.clone());
        tokio::spawn(app.run());

        Ok(())
    }
}

struct VpnApp {
    config: Arc<VpnConfig>,
    vpn_mgr: VpnManager,
    state: Box<dyn VpnState>,
    last_triggers: HashMap<String, tokio::time::Instant>,
    event_bus: EventBus,
}

impl VpnApp {
    fn new(config: Arc<VpnConfig>, event_bus: EventBus) -> Self {
        let mut last_triggers = HashMap::new();
        for state in &config.states {
            last_triggers.insert(state.id.clone(), tokio::time::Instant::now() - std::time::Duration::from_secs(5));
        }

        VpnApp {
            config,
            vpn_mgr: VpnManager::new(),
            state: Box::new(Disconnected),
            last_triggers,
            event_bus,
        }
    }

    async fn run(mut self) {
        let current_state = self.vpn_mgr.detect_state(&self.config.states).await;
        println!("Фича VPN инициализирована. Текущий статус: {}", current_state.display_name);

        let initial_context = EventContext {
            initiator: Initiator::Direct,
            silent: true,
        };

        self.event_bus.publish(SystemEvent::VpnStateChanged {
            state_id: current_state.id.clone(),
            display_name: current_state.display_name.clone(),
            ip_info: None,
            context: initial_context,
        });

        if current_state.interface.is_some() {
            self.state = Box::new(Connected {
                id: current_state.id.clone(),
                display_name: current_state.display_name.clone(),
            });
        } else {
            self.state = Box::new(Disconnected);
        }

        let mut rx = self.event_bus.subscribe();
        while let Ok(event) = rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: SystemEvent) {
        match event {
            SystemEvent::RequestVpnSwitch { state_id, context } => {
                let target_state = self.config.states.iter().find(|s| s.id == state_id);

                if let Some(state) = target_state {
                    let now = tokio::time::Instant::now();
                    
                    if let Some(last_trigger) = self.last_triggers.get_mut(&state_id) {
                        if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                            return;
                        }
                        *last_trigger = now;
                    }

                    if self.state.is_transitioning() {
                        println!("Игнорируем запрос на {}, так как процесс переключения уже запущен", state_id);
                        return;
                    }

                    self.state = Box::new(Transitioning {
                        target_id: state.id.clone(),
                        display_name: state.display_name.clone(),
                    });

                    let state_clone = state.clone();
                    let config_clone = Arc::clone(&self.config);
                    let event_bus_clone = self.event_bus.clone();
                    let context_clone = context.clone();

                    tokio::spawn(async move {
                        let vpn_mgr = VpnManager::new();

                        event_bus_clone.publish(SystemEvent::VpnTransitionStarted {
                            state_id: state_clone.id.clone(),
                            display_name: state_clone.display_name.clone(),
                            has_interface: state_clone.interface.is_some(),
                            context: context_clone.clone(),
                        });

                        match vpn_mgr.switch_to(&state_clone, &config_clone.states).await {
                            Ok(maybe_ip_info) => {
                                let ip_info_str = maybe_ip_info.map(|info| {
                                    format!("IP: {} ({}, {})", info.query, info.city, info.country)
                                });
                                event_bus_clone.publish(SystemEvent::VpnStateChanged {
                                    state_id: state_clone.id.clone(),
                                    display_name: state_clone.display_name.clone(),
                                    ip_info: ip_info_str,
                                    context: context_clone,
                                });
                            }
                            Err(err_msg) => {
                                event_bus_clone.publish(SystemEvent::VpnStateTransitionFailed {
                                    state_id: state_clone.id.clone(),
                                    display_name: state_clone.display_name.clone(),
                                    error: err_msg,
                                    context: context_clone,
                                });
                            }
                        }
                    });
                }
            }
            SystemEvent::VpnStateChanged { state_id, display_name, .. } => {
                if state_id == "off" {
                    self.state = Box::new(Disconnected);
                } else {
                    self.state = Box::new(Connected {
                        id: state_id,
                        display_name,
                    });
                }
            }
            SystemEvent::VpnStateTransitionFailed { .. } => {
                let current_state = self.vpn_mgr.detect_state(&self.config.states).await;
                if current_state.interface.is_some() {
                    self.state = Box::new(Connected {
                        id: current_state.id,
                        display_name: current_state.display_name,
                    });
                } else {
                    self.state = Box::new(Disconnected);
                }
            }
            _ => {}
        }
    }
}
