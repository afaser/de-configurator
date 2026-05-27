pub mod config;
pub mod event;

use tokio::sync::mpsc;
use crate::features::vpn::event::VpnInputEvent;
use crate::features::daemons::event::DaemonInputEvent;
use event::{ActionInputEvent, CommandAction};
pub use config::ActionsConfig;

pub struct ActionsFeature;

impl ActionsFeature {
    /// Запуск фичи Actions
    pub async fn start(
        doc: &kdl::KdlDocument,
        vpn_tx: Option<mpsc::Sender<VpnInputEvent>>,
        daemons_tx: Option<mpsc::Sender<DaemonInputEvent>>,
    ) -> Result<mpsc::Sender<ActionInputEvent>, String> {
        let config = ActionsConfig::parse_from_root_doc(doc)?;
        let (tx, mut rx) = mpsc::channel::<ActionInputEvent>(100);

        tokio::spawn(async move {
            println!("Фича Actions инициализирована. Зарегистрировано действий: {}", config.actions.len());
            for action in &config.actions {
                println!("  action '{}' (шагов: {})", action.id, action.commands.len());
            }

            while let Some(event) = rx.recv().await {
                match event {
                    ActionInputEvent::ExecuteAction(action_id) => {
                        execute_action_recursive(&action_id, 0, &config, &vpn_tx, &daemons_tx).await;
                    }
                    ActionInputEvent::ExecuteCommand(cmd) => {
                        execute_single_command(&cmd, &config, &vpn_tx, &daemons_tx).await;
                    }
                }
            }
        });

        Ok(tx)
    }
}

/// Асинхронное рекурсивное выполнение экшена
fn execute_action_recursive<'a>(
    action_id: &'a str,
    depth: usize,
    config: &'a ActionsConfig,
    vpn_tx: &'a Option<mpsc::Sender<VpnInputEvent>>,
    daemons_tx: &'a Option<mpsc::Sender<DaemonInputEvent>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        if depth > 10 {
            eprintln!("Ошибка: превышена максимальная глубина рекурсии (10) при вызове экшена '{}'", action_id);
            let _ = notify_rust::Notification::new()
                .summary("Ошибка вызова экшена")
                .body(&format!("Превышена глубина рекурсии при вызове '{}'", action_id))
                .show();
            return;
        }

        let action = match config.actions.iter().find(|a| a.id == action_id) {
            Some(a) => a,
            None => {
                eprintln!("Ошибка: Экшен '{}' не найден", action_id);
                return;
            }
        };

        println!("Выполнение экшена '{}' (глубина {})", action_id, depth);

        for cmd in &action.commands {
            match cmd {
                CommandAction::Run(parts) => {
                    if !parts.is_empty() {
                        let mut command = tokio::process::Command::new(&parts[0]);
                        command.args(&parts[1..]);
                        match command.spawn() {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Ошибка запуска внешней команды в экшене: {}", e);
                                let _ = notify_rust::Notification::new()
                                    .summary("Ошибка запуска")
                                    .body(&format!("Команда {:?} завершилась с ошибкой: {}", parts, e))
                                    .show();
                            }
                        }
                    }
                }
                CommandAction::Vpn(state) => {
                    if let Some(tx) = vpn_tx {
                        let _ = tx.send(VpnInputEvent::RequestStateSwitch(state.clone())).await;
                    } else {
                        eprintln!("Ошибка: экшен запросил VPN '{}', но фича VPN отключена", state);
                    }
                }
                CommandAction::Daemon(daemon_id, daemon_action) => {
                    if let Some(tx) = daemons_tx {
                        let _ = tx.send(DaemonInputEvent::Control {
                            daemon_id: daemon_id.clone(),
                            action: *daemon_action,
                            silent: false,
                        }).await;
                    } else {
                        eprintln!("Ошибка: экшен запросил управление демоном '{}', но фича Daemons отключена", daemon_id);
                    }
                }
                CommandAction::Action(target) => {
                    execute_action_recursive(target, depth + 1, config, vpn_tx, daemons_tx).await;
                }
            }
        }
    })
}

/// Выполнение одиночной команды
async fn execute_single_command(
    cmd: &CommandAction,
    config: &ActionsConfig,
    vpn_tx: &Option<mpsc::Sender<VpnInputEvent>>,
    daemons_tx: &Option<mpsc::Sender<DaemonInputEvent>>,
) {
    match cmd {
        CommandAction::Run(parts) => {
            if !parts.is_empty() {
                let mut command = tokio::process::Command::new(&parts[0]);
                command.args(&parts[1..]);
                let _ = command.spawn();
            }
        }
        CommandAction::Vpn(state) => {
            if let Some(tx) = vpn_tx {
                let _ = tx.send(VpnInputEvent::RequestStateSwitch(state.clone())).await;
            }
        }
        CommandAction::Daemon(daemon_id, daemon_action) => {
            if let Some(tx) = daemons_tx {
                let _ = tx.send(DaemonInputEvent::Control {
                    daemon_id: daemon_id.clone(),
                    action: *daemon_action,
                    silent: false,
                }).await;
            }
        }
        CommandAction::Action(target) => {
            execute_action_recursive(target, 0, config, vpn_tx, daemons_tx).await;
        }
    }
}
