mod config;

use std::sync::Arc;
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
            match manager.register(bind.hotkey) {
                Ok(_) => {
                    dispatcher.register(bind.hotkey.id(), local_tx.clone());
                }
                Err(e) => {
                    eprintln!("Предупреждение: Не удалось зарегистрировать хоткей '{}': {:?}", bind.hotkey_str, e);
                }
            }
        }

        tokio::spawn(async move {
            println!("Фича Keybinds инициализирована. Зарегистрировано биндов: {}", config.binds.len());
            for bind in &config.binds {
                match &bind.action {
                    CommandAction::Run(cmd) => println!("  {:18} -> run {:?}", bind.hotkey_str, cmd),
                    CommandAction::Vpn(state) => println!("  {:18} -> vpn '{}'", bind.hotkey_str, state),
                    CommandAction::Daemon(daemon_id, action) => println!("  {:18} -> daemon '{}' {:?}", bind.hotkey_str, daemon_id, action),
                    CommandAction::Action(name) => println!("  {:18} -> action '{}'", bind.hotkey_str, name),
                    CommandAction::Workspace(ws_id, mon) => {
                        if let Some(m) = mon {
                            println!("  {:18} -> workspace '{}' on monitor '{}'", bind.hotkey_str, ws_id, m);
                        } else {
                            println!("  {:18} -> workspace '{}'", bind.hotkey_str, ws_id);
                        }
                    }
                }
            }

            while let Some(event_id) = local_rx.recv().await {
                let bind = config.binds.iter().find(|b| b.hotkey.id() == event_id);
                if let Some(b) = bind {
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
                        CommandAction::Workspace(ws_id, monitor_name) => {
                            event_bus.publish(SystemEvent::RequestWorkspaceFocus {
                                workspace_id: ws_id.clone(),
                                monitor_name: monitor_name.clone(),
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
