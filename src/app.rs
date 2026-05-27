use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::config::Config;
use crate::vpn::VpnManager;
use crate::event::{InputEvent, DomainEvent};

pub struct App {
    config: Arc<Config>,
    vpn_mgr: VpnManager,
    is_transitioning: Arc<AtomicBool>,
    last_triggers: HashMap<String, tokio::time::Instant>, // Мапа (state_id -> время) для троттлинга
    event_tx: mpsc::Sender<DomainEvent>,                 // Передатчик доменных событий
}

impl App {
    pub fn new(config: Config, event_tx: mpsc::Sender<DomainEvent>) -> Self {
        let mut last_triggers = HashMap::new();
        for state in &config.states {
            last_triggers.insert(state.id.clone(), tokio::time::Instant::now() - std::time::Duration::from_secs(5));
        }

        App {
            config: Arc::new(config),
            vpn_mgr: VpnManager::new(),
            is_transitioning: Arc::new(AtomicBool::new(false)),
            last_triggers,
            event_tx,
        }
    }

    /// Запуск основного цикла обработки входящих событий
    pub async fn run(mut self, mut rx: mpsc::Receiver<InputEvent>) {
        let current_state = self.vpn_mgr.detect_state(&self.config.states).await;
        println!("Текущий статус VPN: {}", current_state.display_name);

        println!("Зарегистрированные горячие клавиши:");
        for state in &self.config.states {
            println!("  {:18} -> {}", state.hotkey_str, state.display_name);
        }

        while let Some(event) = rx.recv().await {
            self.handle_event(event).await;
        }
    }

    /// Обработка поступившего входящего события
    async fn handle_event(&mut self, event: InputEvent) {
        match event {
            InputEvent::RequestStateSwitch(state_id) => {
                let target_state = self.config.states.iter().find(|s| s.id == state_id);

                if let Some(state) = target_state {
                    let now = tokio::time::Instant::now();
                    
                    // Подавление дребезга кнопок (throttle) на основе state_id
                    if let Some(last_trigger) = self.last_triggers.get_mut(&state_id) {
                        if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                            return; // Игнорируем частые нажатия
                        }
                        *last_trigger = now;
                    }

                    if self.is_transitioning.load(Ordering::SeqCst) {
                        println!("Игнорируем запрос на {}, так как процесс переключения уже запущен", state_id);
                        return;
                    }

                    let is_transitioning_clone = Arc::clone(&self.is_transitioning);
                    is_transitioning_clone.store(true, Ordering::SeqCst);

                    let state_clone = state.clone();
                    let config_clone = Arc::clone(&self.config);
                    let event_tx_clone = self.event_tx.clone();

                    tokio::spawn(async move {
                        let vpn_mgr = VpnManager::new();

                        // 1. Эмитим событие о начале перехода
                        let _ = event_tx_clone.send(DomainEvent::TransitionStarted {
                            state_id: state_clone.id.clone(),
                            display_name: state_clone.display_name.clone(),
                            has_interface: state_clone.interface.is_some(),
                        }).await;

                        // 2. Выполняем переход
                        match vpn_mgr.switch_to(&state_clone, &config_clone.states).await {
                            Ok(maybe_ip_info) => {
                                // 3. Эмитим событие об успешном завершении
                                let _ = event_tx_clone.send(DomainEvent::TransitionCompleted {
                                    state_id: state_clone.id.clone(),
                                    display_name: state_clone.display_name.clone(),
                                    ip_info: maybe_ip_info,
                                }).await;
                            }
                            Err(err_msg) => {
                                // 4. Эмитим событие о сбое переключения
                                let _ = event_tx_clone.send(DomainEvent::TransitionFailed {
                                    state_id: state_clone.id.clone(),
                                    display_name: state_clone.display_name.clone(),
                                    error: err_msg,
                                }).await;
                            }
                        }

                        is_transitioning_clone.store(false, Ordering::SeqCst);
                    });
                }
            }
        }
    }
}
