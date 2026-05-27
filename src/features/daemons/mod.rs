pub mod config;
pub mod event;
pub mod manager;
pub mod notifier;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use kdl::KdlDocument;

use config::DaemonsConfig;
use event::{DaemonInputEvent, DaemonDomainEvent, DaemonAction};
use manager::DaemonManager;
use notifier::DaemonNotificationService;

pub struct DaemonsFeature;

impl DaemonsFeature {
    /// Запуск фичи Daemons на основе KDL-конфигурации
    pub async fn start(doc: &KdlDocument) -> Result<mpsc::Sender<DaemonInputEvent>, String> {
        // 1. Парсим конфигурацию демонов
        let config = DaemonsConfig::parse_from_root_doc(doc)?;
        let config = Arc::new(config);

        // 2. Создаем каналы событий (Event-Driven)
        let (input_tx, input_rx) = mpsc::channel::<DaemonInputEvent>(100);
        let (domain_tx, domain_rx) = mpsc::channel::<DaemonDomainEvent>(100);

        // 3. Запускаем службу уведомлений
        tokio::spawn(DaemonNotificationService::run(domain_rx));

        // 4. Запускаем фоновый цикл обработки управления демонами
        let app = DaemonApp::new(config.clone(), domain_tx);
        tokio::spawn(app.run(input_rx));

        // 5. Обработка автозапуска (autostart = true)
        for daemon in &config.daemons {
            if daemon.autostart {
                let input_tx_clone = input_tx.clone();
                let daemon_id = daemon.id.clone();
                tokio::spawn(async move {
                    // Даем системе немного времени для полной инициализации
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let _ = input_tx_clone.send(DaemonInputEvent::Control {
                        daemon_id,
                        action: DaemonAction::On,
                        silent: true,
                    }).await;
                });
            }
        }

        Ok(input_tx)
    }
}

struct DaemonApp {
    config: Arc<DaemonsConfig>,
    manager: DaemonManager,
    children: HashMap<String, tokio::process::Child>, // Храним запущенные дочерние процессы
    event_tx: mpsc::Sender<DaemonDomainEvent>,
}

impl DaemonApp {
    fn new(config: Arc<DaemonsConfig>, event_tx: mpsc::Sender<DaemonDomainEvent>) -> Self {
        Self {
            config,
            manager: DaemonManager::new(),
            children: HashMap::new(),
            event_tx,
        }
    }

    async fn run(mut self, mut rx: mpsc::Receiver<DaemonInputEvent>) {
        println!("Фича Daemons инициализирована. Зарегистрировано демонов: {}", self.config.daemons.len());
        for daemon in &self.config.daemons {
            println!("  {:18} (autostart={})", daemon.id, daemon.autostart);
        }

        while let Some(event) = rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn is_daemon_running(&mut self, daemon: &config::DaemonConfig) -> bool {
        // 1. Сначала проверяем, есть ли запущенный дочерний процесс у нас в памяти, и живой ли он
        let mut is_alive = false;
        if let Some(child) = self.children.get_mut(&daemon.id) {
            if let Ok(None) = child.try_wait() {
                is_alive = true;
            }
        }
        
        if is_alive {
            return true;
        } else {
            // Если дочерний процесс завершился, убираем его из HashMap
            self.children.remove(&daemon.id);
        }

        // 2. Если дочернего процесса нет или он завершился, проверяем в системе по имени процесса
        let process_name = std::path::Path::new(&daemon.start_cmd[0])
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&daemon.id);

        self.manager.is_running(process_name).await
    }

    async fn handle_event(&mut self, event: DaemonInputEvent) {
        match event {
            DaemonInputEvent::Control { daemon_id, action, silent } => {
                let target_daemon = self.config.daemons.iter().find(|d| d.id == daemon_id).cloned();

                if let Some(daemon) = target_daemon {
                    let is_running = self.is_daemon_running(&daemon).await;

                    match action {
                        DaemonAction::On => {
                            if is_running {
                                if !silent {
                                    println!("Запрос на запуск '{}' проигнорирован, процесс уже запущен", daemon.id);
                                }
                                return;
                            }
                            self.start_daemon_process(&daemon, silent).await;
                        }
                        DaemonAction::Off => {
                            if !is_running {
                                println!("Запрос на остановку '{}' проигнорирован, процесс не запущен", daemon.id);
                                return;
                            }
                            self.stop_daemon_process(&daemon).await;
                        }
                        DaemonAction::Toggle => {
                            if is_running {
                                self.stop_daemon_process(&daemon).await;
                            } else {
                                self.start_daemon_process(&daemon, silent).await;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Вспомогательный метод для запуска демона
    async fn start_daemon_process(&mut self, daemon: &config::DaemonConfig, silent: bool) {
        match self.manager.start_daemon(&daemon.start_cmd) {
            Ok(child) => {
                // Сохраняем child handle
                self.children.insert(daemon.id.clone(), child);
                
                // Отправляем уведомление об успешном запуске
                if !silent {
                    let _ = self.event_tx.send(DaemonDomainEvent::StateChanged {
                        daemon_id: daemon.id.clone(),
                        display_name: daemon.display_name.clone(),
                        is_running: true,
                    }).await;
                }
            }
            Err(e) => {
                let _ = self.event_tx.send(DaemonDomainEvent::TransitionFailed {
                    daemon_id: daemon.id.clone(),
                    display_name: daemon.display_name.clone(),
                    action: DaemonAction::On,
                    error: e,
                }).await;
            }
        }
    }

    /// Вспомогательный метод для остановки демона
    async fn stop_daemon_process(&mut self, daemon: &config::DaemonConfig) {
        // 1. Пытаемся остановить дочерний процесс, если он хранится у нас
        if let Some(mut child) = self.children.remove(&daemon.id) {
            let _ = child.kill().await;
        }

        // 2. Ищем и убиваем по PID (для надежности и на случай перезапуска приложения)
        let process_name = std::path::Path::new(&daemon.start_cmd[0])
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&daemon.id);

        let pids = self.manager.get_pids(process_name).await;
        if !pids.is_empty() {
            if let Err(e) = self.manager.kill_pids(&pids).await {
                let _ = self.event_tx.send(DaemonDomainEvent::TransitionFailed {
                    daemon_id: daemon.id.clone(),
                    display_name: daemon.display_name.clone(),
                    action: DaemonAction::Off,
                    error: e,
                }).await;
                return;
            }
        }

        // 3. Если в конфиге была прописана специальная stop-cmd, также выполняем ее
        if let Some(ref stop_cmd) = daemon.stop_cmd {
            let mut cmd = tokio::process::Command::new(&stop_cmd[0]);
            cmd.args(&stop_cmd[1..]);
            let _ = cmd.status().await;
        }

        // Отправляем уведомление о выключении
        let _ = self.event_tx.send(DaemonDomainEvent::StateChanged {
            daemon_id: daemon.id.clone(),
            display_name: daemon.display_name.clone(),
            is_running: false,
        }).await;
    }
}
