mod config;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use kdl::KdlDocument;
use global_hotkey::GlobalHotKeyManager;
use crate::core::hotkey_dispatcher::HotkeyDispatcher;
use crate::features::vpn::event::VpnInputEvent;
use crate::features::daemons::event::DaemonInputEvent;

use config::{KeybindsConfig, KeybindAction};

pub struct KeybindsFeature;

impl KeybindsFeature {
    /// Запуск фичи Keybinds на основе KDL-документа конфигурации
    pub async fn start(
        doc: &KdlDocument,
        manager: Arc<GlobalHotKeyManager>,
        dispatcher: &mut HotkeyDispatcher,
        vpn_tx: Option<mpsc::Sender<VpnInputEvent>>,
        daemons_tx: Option<mpsc::Sender<DaemonInputEvent>>,
    ) -> Result<(), String> {
        // 1. Парсим конфигурацию keybinds
        let config = KeybindsConfig::parse_from_root_doc(doc)?;
        let config = Arc::new(config);

        // 2. Создаем асинхронный канал для получения событий от диспетчера
        let (local_tx, mut local_rx) = mpsc::channel::<u32>(100);

        // 3. Регистрируем клавиши в глобальном менеджере и диспетчере
        for bind in &config.binds {
            manager
                .register(bind.hotkey)
                .map_err(|e| format!("Не удалось зарегистрировать хоткей '{}': {:?}", bind.hotkey_str, e))?;
            
            dispatcher.register(bind.hotkey.id(), local_tx.clone());
        }

        // 4. Запускаем фоновый цикл обработки нажатий
        tokio::spawn(async move {
            let mut last_triggers = HashMap::new();
            for bind in &config.binds {
                last_triggers.insert(bind.hotkey.id(), tokio::time::Instant::now() - std::time::Duration::from_secs(5));
            }

            println!("Фича Keybinds инициализирована. Зарегистрировано биндов: {}", config.binds.len());
            for bind in &config.binds {
                match &bind.action {
                    KeybindAction::Run(cmd) => println!("  {:18} -> run {:?}", bind.hotkey_str, cmd),
                    KeybindAction::Vpn(state) => println!("  {:18} -> vpn '{}'", bind.hotkey_str, state),
                    KeybindAction::Daemon(daemon_id, action) => println!("  {:18} -> daemon '{}' {:?}", bind.hotkey_str, daemon_id, action),
                }
            }

            while let Some(event_id) = local_rx.recv().await {
                let bind = config.binds.iter().find(|b| b.hotkey.id() == event_id);
                if let Some(b) = bind {
                    let now = tokio::time::Instant::now();
                    
                    // Подавление дребезга кнопок (throttle) на основе ID хоткея
                    if let Some(last_trigger) = last_triggers.get_mut(&event_id) {
                        if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                            continue;
                        }
                        *last_trigger = now;
                    }

                    match &b.action {
                        KeybindAction::Run(cmd_parts) => {
                            if !cmd_parts.is_empty() {
                                let mut cmd = tokio::process::Command::new(&cmd_parts[0]);
                                cmd.args(&cmd_parts[1..]);
                                if let Err(e) = cmd.spawn() {
                                    eprintln!("Ошибка запуска команды: {}", e);
                                    let _ = notify_rust::Notification::new()
                                        .summary("Ошибка запуска")
                                        .body(&format!("Команда {:?} завершилась с ошибкой: {}", cmd_parts, e))
                                        .show();
                                }
                            }
                        }
                        KeybindAction::Vpn(state_id) => {
                            if let Some(ref tx) = vpn_tx {
                                let _ = tx.send(VpnInputEvent::RequestStateSwitch(state_id.clone())).await;
                            } else {
                                eprintln!("Ошибка: Запрошено действие vpn '{}', но фича VPN отключена в конфигурации", state_id);
                                let _ = notify_rust::Notification::new()
                                    .summary("Ошибка VPN")
                                    .body(&format!("Не удалось переключить на '{}': VPN фича отключена в конфигурации", state_id))
                                    .show();
                            }
                        }
                        KeybindAction::Daemon(daemon_id, action) => {
                            if let Some(ref tx) = daemons_tx {
                                let _ = tx.send(DaemonInputEvent::Control {
                                    daemon_id: daemon_id.clone(),
                                    action: *action,
                                    silent: false,
                                }).await;
                            } else {
                                eprintln!("Ошибка: Запрошено управление демоном '{}', но фича Daemons отключена в конфигурации", daemon_id);
                                let _ = notify_rust::Notification::new()
                                    .summary("Ошибка демонов")
                                    .body(&format!("Не удалось выполнить {:?} для '{}': фича Daemons отключена", action, daemon_id))
                                    .show();
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
