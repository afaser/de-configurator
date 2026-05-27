pub mod config;
pub mod event;
pub mod manager;
pub mod notifier;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::mpsc;
use kdl::KdlDocument;

use config::VpnConfig;
use event::{VpnInputEvent, VpnDomainEvent};
use manager::VpnManager;
use notifier::VpnNotificationService;

pub struct VpnFeature;

impl VpnFeature {
    /// Запуск фичи VPN на основе переданного KDL-документа конфигурации
    pub async fn start(
        doc: &KdlDocument,
    ) -> Result<mpsc::Sender<VpnInputEvent>, String> {
        // 1. Парсим настройки VPN
        let vpn_config = VpnConfig::parse_from_root_doc(doc)?;
        let vpn_config = Arc::new(vpn_config);

        // 2. Создаем каналы событий (Event-Driven)
        let (input_tx, input_rx) = mpsc::channel::<VpnInputEvent>(100);
        let (domain_tx, domain_rx) = mpsc::channel::<VpnDomainEvent>(100);

        // 3. Запускаем службу уведомлений
        tokio::spawn(VpnNotificationService::run(domain_rx));

        // 4. Запускаем основное асинхронное приложение фичи в фоне
        let app = VpnApp::new(vpn_config, domain_tx);
        tokio::spawn(app.run(input_rx));

        Ok(input_tx)
    }
}

struct VpnApp {
    config: Arc<VpnConfig>,
    vpn_mgr: VpnManager,
    is_transitioning: Arc<AtomicBool>,
    last_triggers: HashMap<String, tokio::time::Instant>,
    event_tx: mpsc::Sender<VpnDomainEvent>,
}

impl VpnApp {
    fn new(config: Arc<VpnConfig>, event_tx: mpsc::Sender<VpnDomainEvent>) -> Self {
        let mut last_triggers = HashMap::new();
        for state in &config.states {
            last_triggers.insert(state.id.clone(), tokio::time::Instant::now() - std::time::Duration::from_secs(5));
        }

        VpnApp {
            config,
            vpn_mgr: VpnManager::new(),
            is_transitioning: Arc::new(AtomicBool::new(false)),
            last_triggers,
            event_tx,
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<VpnInputEvent>) {
        let current_state = self.vpn_mgr.detect_state(&self.config.states).await;
        println!("Фича VPN инициализирована. Текущий статус: {}", current_state.display_name);

        while let Some(event) = rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn handle_event(&mut self, event: VpnInputEvent) {
        match event {
            VpnInputEvent::RequestStateSwitch(state_id) => {
                let target_state = self.config.states.iter().find(|s| s.id == state_id);

                if let Some(state) = target_state {
                    let now = tokio::time::Instant::now();
                    
                    // Подавление дребезга (throttle) по ID состояния
                    if let Some(last_trigger) = self.last_triggers.get_mut(&state_id) {
                        if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                            return;
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

                        // Эмитим начало перехода
                        let _ = event_tx_clone.send(VpnDomainEvent::TransitionStarted {
                            state_id: state_clone.id.clone(),
                            display_name: state_clone.display_name.clone(),
                            has_interface: state_clone.interface.is_some(),
                        }).await;

                        match vpn_mgr.switch_to(&state_clone, &config_clone.states).await {
                            Ok(maybe_ip_info) => {
                                // Эмитим успех перехода
                                let _ = event_tx_clone.send(VpnDomainEvent::TransitionCompleted {
                                    state_id: state_clone.id.clone(),
                                    display_name: state_clone.display_name.clone(),
                                    ip_info: maybe_ip_info,
                                }).await;
                            }
                            Err(err_msg) => {
                                // Эмитим провал
                                let _ = event_tx_clone.send(VpnDomainEvent::TransitionFailed {
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
