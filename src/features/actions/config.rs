use crate::core::event_bus::{CommandAction, DaemonAction};
use kdl::KdlDocument;

#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub id: String,
    pub export: bool,
    pub commands: Vec<CommandAction>,
}

#[derive(Debug, Clone)]
pub struct ActionsConfig {
    pub actions: Vec<ActionConfig>,
}

impl ActionsConfig {
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let actions_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
                && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("actions")
        });

        let mut actions = Vec::new();

        let node = match actions_node {
            Some(n) => n,
            None => return Ok(ActionsConfig { actions }),
        };

        if let Some(children) = node.children() {
            for action_node in children.nodes() {
                if action_node.name().value() == "action" {
                    let id = action_node
                        .entries()
                        .get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| {
                            "У узла action должен быть строковый идентификатор".to_string()
                        })?
                        .to_string();

                    let export = action_node.entries().iter()
                        .find(|e| e.name().map(|i| i.value()) == Some("export"))
                        .and_then(|e| e.value().as_bool())
                        .unwrap_or(false);

                    let mut commands = Vec::new();

                    if let Some(cmd_children) = action_node.children() {
                        for cmd_node in cmd_children.nodes() {
                            let cmd = match cmd_node.name().value() {
                                "run" => {
                                    let parts: Vec<String> = cmd_node.entries().iter()
                                        .filter_map(|e| e.value().as_string().map(String::from))
                                        .collect();
                                    if parts.is_empty() {
                                        return Err(format!("У действия run в экшене '{}' не указана команда", id));
                                    }
                                    CommandAction::Run(parts)
                                }
                                "vpn" => {
                                    let state = cmd_node.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У действия vpn в экшене '{}' не указано состояние", id))?
                                        .to_string();
                                    CommandAction::Vpn(state)
                                }
                                "daemon" => {
                                    let daemon_id = cmd_node.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| {
                                            format!(
                                                "У действия daemon в экшене '{}' не указан ID",
                                                id
                                            )
                                        })?
                                        .to_string();
                                    let action_str = cmd_node.entries().get(1)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У действия daemon в экшене '{}' не указано действие (on, off, toggle)", id))?;

                                    let daemon_action = match action_str {
                                        "on" => DaemonAction::On,
                                        "off" => DaemonAction::Off,
                                        "toggle" => DaemonAction::Toggle,
                                        other => {
                                            return Err(format!(
                                                "Неподдерживаемое действие демона '{}' в экшене '{}'",
                                                other, id
                                            ));
                                        }
                                    };
                                    CommandAction::Daemon(daemon_id, daemon_action)
                                }
                                "action" => {
                                    let target_action = cmd_node.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У действия action в экшене '{}' не указано целевое действие", id))?
                                        .to_string();
                                    CommandAction::Action(target_action)
                                }
                                other => {
                                    return Err(format!(
                                        "Неизвестная команда '{}' в экшене '{}'",
                                        other, id
                                    ));
                                }
                            };
                            commands.push(cmd);
                        }
                    }

                    actions.push(ActionConfig { id, export, commands });
                }
            }
        }

        Ok(ActionsConfig { actions })
    }
}
