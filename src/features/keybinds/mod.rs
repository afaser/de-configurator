mod config;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use kdl::KdlDocument;
use global_hotkey::GlobalHotKeyManager;
use crate::core::hotkey_dispatcher::HotkeyDispatcher;
use crate::core::event_bus::{EventBus, SystemEvent, CommandAction, EventContext, Initiator};
use config::KeybindsConfig;

pub struct KeybindsFeature;

impl KeybindsFeature {
    pub async fn start(
        doc: &KdlDocument,
        manager: Arc<GlobalHotKeyManager>,
        dispatcher: &mut HotkeyDispatcher,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = KeybindsConfig::parse_from_root_doc(doc)?;
        let config = Arc::new(config);

        let (local_tx, mut local_rx) = mpsc::channel::<u32>(100);

        for bind in &config.binds {
            manager
                .register(bind.hotkey)
                .map_err(|e| format!("Не удалось зарегистрировать хоткей '{}': {:?}", bind.hotkey_str, e))?;
            
            dispatcher.register(bind.hotkey.id(), local_tx.clone());
        }

        tokio::spawn(async move {
            let mut last_triggers = HashMap::new();
            for bind in &config.binds {
                last_triggers.insert(bind.hotkey.id(), tokio::time::Instant::now() - std::time::Duration::from_secs(5));
            }

            println!("Фича Keybinds инициализирована. Зарегистрировано биндов: {}", config.binds.len());
            for bind in &config.binds {
                match &bind.action {
                    CommandAction::Run(cmd) => println!("  {:18} -> run {:?}", bind.hotkey_str, cmd),
                    CommandAction::Vpn(state) => println!("  {:18} -> vpn '{}'", bind.hotkey_str, state),
                    CommandAction::Daemon(daemon_id, action) => println!("  {:18} -> daemon '{}' {:?}", bind.hotkey_str, daemon_id, action),
                    CommandAction::Action(name) => println!("  {:18} -> action '{}'", bind.hotkey_str, name),
                }
            }

            while let Some(event_id) = local_rx.recv().await {
                let bind = config.binds.iter().find(|b| b.hotkey.id() == event_id);
                if let Some(b) = bind {
                    let now = tokio::time::Instant::now();
                    
                    if let Some(last_trigger) = last_triggers.get_mut(&event_id) {
                        if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                            continue;
                        }
                        *last_trigger = now;
                    }

                    let context = EventContext {
                        initiator: Initiator::Keybind,
                        silent: false,
                    };

                    match &b.action {
                        CommandAction::Run(cmd_parts) => {
                            event_bus.publish(SystemEvent::RequestCommandExecute {
                                command: CommandAction::Run(cmd_parts.clone()),
                                context,
                            });
                        }
                        CommandAction::Vpn(state_id) => {
                            event_bus.publish(SystemEvent::RequestVpnSwitch {
                                state_id: state_id.clone(),
                                context,
                            });
                        }
                        CommandAction::Daemon(daemon_id, action) => {
                            event_bus.publish(SystemEvent::RequestDaemonControl {
                                daemon_id: daemon_id.clone(),
                                action: *action,
                                context,
                            });
                        }
                        CommandAction::Action(action_id) => {
                            event_bus.publish(SystemEvent::RequestActionExecute {
                                action_id: action_id.clone(),
                                context,
                            });
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
