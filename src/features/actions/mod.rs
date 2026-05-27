pub mod config;

pub use config::ActionsConfig;
use std::sync::Arc;
use std::collections::HashMap;
use crate::core::event_bus::{EventBus, SystemEvent, CommandAction, DaemonAction, EventContext, Initiator};

pub trait Command: Send + Sync {
    fn execute<'a>(
        &'a self,
        context: &'a EventContext,
        event_bus: &'a EventBus,
        registry: &'a ActionRegistry,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;
}

pub struct RunCommand {
    pub parts: Vec<String>,
}

impl Command for RunCommand {
    fn execute<'a>(
        &'a self,
        _context: &'a EventContext,
        _event_bus: &'a EventBus,
        _registry: &'a ActionRegistry,
        _depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            if !self.parts.is_empty() {
                let mut command = tokio::process::Command::new(&self.parts[0]);
                command.args(&self.parts[1..]);
                command.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;
            }
            Ok(())
        })
    }
}

pub struct VpnSwitchCommand {
    pub state_id: String,
}

impl Command for VpnSwitchCommand {
    fn execute<'a>(
        &'a self,
        context: &'a EventContext,
        event_bus: &'a EventBus,
        _registry: &'a ActionRegistry,
        _depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            event_bus.publish(SystemEvent::RequestVpnSwitch {
                state_id: self.state_id.clone(),
                context: context.clone(),
            });
            Ok(())
        })
    }
}

pub struct DaemonControlCommand {
    pub daemon_id: String,
    pub action: DaemonAction,
}

impl Command for DaemonControlCommand {
    fn execute<'a>(
        &'a self,
        context: &'a EventContext,
        event_bus: &'a EventBus,
        _registry: &'a ActionRegistry,
        _depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            event_bus.publish(SystemEvent::RequestDaemonControl {
                daemon_id: self.daemon_id.clone(),
                action: self.action,
                context: context.clone(),
            });
            Ok(())
        })
    }
}

pub struct ActionReferenceCommand {
    pub action_id: String,
}

impl Command for ActionReferenceCommand {
    fn execute<'a>(
        &'a self,
        context: &'a EventContext,
        event_bus: &'a EventBus,
        registry: &'a ActionRegistry,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            registry.execute_with_depth(&self.action_id, depth + 1, context, event_bus).await
        })
    }
}

pub struct WorkspaceFocusCommand {
    pub workspace_id: String,
    pub monitor_name: Option<String>,
}

impl Command for WorkspaceFocusCommand {
    fn execute<'a>(
        &'a self,
        context: &'a EventContext,
        event_bus: &'a EventBus,
        _registry: &'a ActionRegistry,
        _depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            event_bus.publish(SystemEvent::RequestWorkspaceFocus {
                workspace_id: self.workspace_id.clone(),
                monitor_name: self.monitor_name.clone(),
                context: context.clone(),
            });
            Ok(())
        })
    }
}

pub struct ActionExpression {
    pub id: String,
    pub export: bool,
    pub commands: Vec<Box<dyn Command>>,
}

pub struct ActionRegistry {
    pub actions: HashMap<String, ActionExpression>,
}

impl ActionRegistry {
    pub fn new(config: ActionsConfig) -> Self {
        let mut actions = HashMap::new();
        for act in config.actions {
            let mut commands: Vec<Box<dyn Command>> = Vec::new();
            for cmd in act.commands {
                let box_cmd: Box<dyn Command> = match cmd {
                    CommandAction::Run(parts) => Box::new(RunCommand { parts }),
                    CommandAction::Vpn(state_id) => Box::new(VpnSwitchCommand { state_id }),
                    CommandAction::Daemon(daemon_id, action) => Box::new(DaemonControlCommand { daemon_id, action }),
                    CommandAction::Action(action_id) => Box::new(ActionReferenceCommand { action_id }),
                    CommandAction::Workspace(workspace_id, monitor_name) => Box::new(WorkspaceFocusCommand { workspace_id, monitor_name }),
                };
                commands.push(box_cmd);
            }
            actions.insert(act.id.clone(), ActionExpression {
                id: act.id,
                export: act.export,
                commands,
            });
        }
        Self { actions }
    }

    pub async fn execute(
        &self,
        action_id: &str,
        context: &EventContext,
        event_bus: &EventBus,
    ) -> Result<(), String> {
        self.execute_with_depth(action_id, 0, context, event_bus).await
    }

    pub fn execute_with_depth<'a>(
        &'a self,
        action_id: &'a str,
        depth: usize,
        context: &'a EventContext,
        event_bus: &'a EventBus,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            if depth > 10 {
                return Err(format!("Max recursion depth exceeded for action: {}", action_id));
            }

            let action = self.actions.get(action_id)
                .ok_or_else(|| format!("Action not found: {}", action_id))?;

            println!("Выполнение экшена '{}' (глубина {})", action_id, depth);

            for cmd in &action.commands {
                cmd.execute(context, event_bus, self, depth).await?;
            }
            Ok(())
        })
    }
}

pub struct ActionsFeature;

impl ActionsFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = ActionsConfig::parse_from_root_doc(doc)?;
        let registry = ActionRegistry::new(config);
        let registry = Arc::new(registry);
        let mut rx = event_bus.subscribe();

        tokio::spawn(async move {
            println!("Фича Actions инициализирована. Зарегистрировано действий: {}", registry.actions.len());
            for action in registry.actions.values() {
                println!("  action '{}' (шагов: {})", action.id, action.commands.len());
            }

            while let Ok(event) = rx.recv().await {
                let registry = registry.clone();
                let bus = event_bus.clone();

                match event {
                    SystemEvent::RequestActionExecute { action_id, context } => {
                        tokio::spawn(async move {
                            let inner_context = EventContext {
                                initiator: Initiator::Action,
                                silent: true,
                            };
                            match registry.execute(&action_id, &inner_context, &bus).await {
                                Ok(_) => {
                                    if !context.silent {
                                        let _ = notify_rust::Notification::new()
                                            .summary("Действие выполнено")
                                            .body(&format!("Режим {} успешно применен", action_id))
                                            .appname("de-configurator")
                                            .show();
                                    }
                                }
                                Err(e) => {
                                    let _ = notify_rust::Notification::new()
                                        .summary("Ошибка действия")
                                        .body(&format!("Не удалось применить {}: {}", action_id, e))
                                        .appname("de-configurator")
                                        .urgency(notify_rust::Urgency::Critical)
                                        .show();
                                }
                            }
                        });
                    }
                    SystemEvent::RequestActionExecuteExported { action_id, context } => {
                        tokio::spawn(async move {
                            let action = registry.actions.get(&action_id);
                            if let Some(act) = action {
                                if act.export {
                                    let inner_context = EventContext {
                                        initiator: Initiator::Action,
                                        silent: true,
                                    };
                                    match registry.execute(&action_id, &inner_context, &bus).await {
                                        Ok(_) => {
                                            if !context.silent {
                                                let _ = notify_rust::Notification::new()
                                                    .summary("Действие выполнено")
                                                    .body(&format!("Режим {} успешно применен", action_id))
                                                    .appname("de-configurator")
                                                    .show();
                                            }
                                        }
                                        Err(e) => {
                                            let _ = notify_rust::Notification::new()
                                                .summary("Ошибка действия")
                                                .body(&format!("Не удалось применить {}: {}", action_id, e))
                                                .appname("de-configurator")
                                                .urgency(notify_rust::Urgency::Critical)
                                                .show();
                                        }
                                    }
                                } else {
                                    eprintln!("Ошибка безопасности: экшен '{}' не экспортирован", action_id);
                                    let _ = notify_rust::Notification::new()
                                        .summary("Ошибка безопасности")
                                        .body(&format!("Экшен '{}' не экспортирован для внешнего вызова", action_id))
                                        .show();
                                }
                            }
                        });
                    }
                    SystemEvent::RequestCommandExecute { command, context } => {
                        tokio::spawn(async move {
                            let cmd_obj: Box<dyn Command> = match command {
                                CommandAction::Run(parts) => Box::new(RunCommand { parts }),
                                CommandAction::Vpn(state_id) => Box::new(VpnSwitchCommand { state_id }),
                                CommandAction::Daemon(daemon_id, action) => Box::new(DaemonControlCommand { daemon_id, action }),
                                CommandAction::Action(action_id) => Box::new(ActionReferenceCommand { action_id }),
                                CommandAction::Workspace(workspace_id, monitor_name) => Box::new(WorkspaceFocusCommand { workspace_id, monitor_name }),
                            };
                            let _ = cmd_obj.execute(&context, &bus, &registry, 0).await;
                        });
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
