use kdl::KdlDocument;
use crate::core::event_bus::{CommandAction, DaemonAction};

#[derive(Debug, Clone)]
pub enum TriggerType {
    Interval {
        duration: std::time::Duration,
    },
    Wm {
        event_type: String,
        desktop: Option<String>,
        monitor: Option<String>,
        class: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct TriggerConfig {
    pub id: String,
    pub trigger_type: TriggerType,
    pub command: CommandAction,
}

#[derive(Debug, Clone)]
pub struct TriggersConfig {
    pub triggers: Vec<TriggerConfig>,
}

impl TriggersConfig {
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let triggers_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("triggers")
        });

        let mut triggers = Vec::new();

        let node = match triggers_node {
            Some(n) => n,
            None => return Ok(TriggersConfig { triggers }),
        };

        if let Some(children) = node.children() {
            for trigger_node in children.nodes() {
                if trigger_node.name().value() == "trigger" {
                    let id = trigger_node.entries().get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| "У узла trigger должен быть строковый идентификатор".to_string())?
                        .to_string();

                    let type_str = trigger_node.entries().iter()
                        .find(|e| e.name().map(|i| i.value()) == Some("type"))
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| format!("У триггера '{}' должен быть тип type", id))?;

                    let mut trigger_command = None;

                    let t_type = if type_str == "interval" {
                        let mut duration = None;
                        if let Some(trigger_children) = trigger_node.children() {
                            for child in trigger_children.nodes() {
                                match child.name().value() {
                                    "interval" => {
                                        let duration_str = child.entries().get(0)
                                            .and_then(|e| e.value().as_string())
                                            .ok_or_else(|| format!("У триггера '{}' должно быть указано время интервала", id))?;
                                        duration = Some(parse_duration(duration_str)?);
                                    }
                                    "run" | "vpn" | "daemon" | "action" | "workspace" => {
                                        trigger_command = Some(parse_command(child, id.as_str())?);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        let dur = duration.ok_or_else(|| format!("У интервального триггера '{}' должен быть указан интервал (например, interval \"5m\")", id))?;
                        TriggerType::Interval { duration: dur }
                    } else if type_str == "wm" {
                        let mut event_type = None;
                        let mut desktop = None;
                        let mut monitor = None;
                        let mut class = None;
                        if let Some(trigger_children) = trigger_node.children() {
                            for child in trigger_children.nodes() {
                                match child.name().value() {
                                    "event" => {
                                        event_type = child.entries().get(0)
                                            .and_then(|e| e.value().as_string())
                                            .map(|s| s.to_string());
                                    }
                                    "desktop" => {
                                        desktop = child.entries().get(0)
                                            .and_then(|e| e.value().as_string())
                                            .map(|s| s.to_string());
                                    }
                                    "monitor" => {
                                        monitor = child.entries().get(0)
                                            .and_then(|e| e.value().as_string())
                                            .map(|s| s.to_string());
                                    }
                                    "class" => {
                                        class = child.entries().get(0)
                                            .and_then(|e| e.value().as_string())
                                            .map(|s| s.to_string());
                                    }
                                    "run" | "vpn" | "daemon" | "action" | "workspace" => {
                                        trigger_command = Some(parse_command(child, id.as_str())?);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        let et = event_type.ok_or_else(|| format!("У WM триггера '{}' должно быть указано событие (например, event \"desktop_focus\")", id))?;
                        TriggerType::Wm { event_type: et, desktop, monitor, class }
                    } else {
                        return Err(format!("Неподдерживаемый тип триггера '{}' у '{}'", type_str, id));
                    };

                    let cmd = trigger_command.ok_or_else(|| format!("У триггера '{}' должно быть указано выполняемое действие (run, vpn, daemon, action или workspace)", id))?;

                    triggers.push(TriggerConfig {
                        id,
                        trigger_type: t_type,
                        command: cmd,
                    });
                }
            }
        }

        Ok(TriggersConfig { triggers })
    }
}

fn parse_command(child: &kdl::KdlNode, id: &str) -> Result<CommandAction, String> {
    match child.name().value() {
        "run" => {
            let parts: Vec<String> = child.entries().iter()
                .filter_map(|e| e.value().as_string().map(String::from))
                .collect();
            if parts.is_empty() {
                return Err(format!("У действия run в триггере '{}' не указана команда", id));
            }
            Ok(CommandAction::Run(parts))
        }
        "vpn" => {
            let state = child.entries().get(0)
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| format!("У действия vpn в триггере '{}' не указано состояние", id))?
                .to_string();
            Ok(CommandAction::Vpn(state))
        }
        "daemon" => {
            let daemon_id = child.entries().get(0)
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| format!("У действия daemon в триггере '{}' не указан ID", id))?
                .to_string();
            let action_str = child.entries().get(1)
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| format!("У действия daemon в триггере '{}' не указано действие", id))?;
            let daemon_action = match action_str {
                "on" => DaemonAction::On,
                "off" => DaemonAction::Off,
                "toggle" => DaemonAction::Toggle,
                other => return Err(format!("Неподдерживаемое действие демона '{}' в триггере '{}'", other, id)),
            };
            Ok(CommandAction::Daemon(daemon_id, daemon_action))
        }
        "action" => {
            let action_name = child.entries().get(0)
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| format!("У действия action в триггере '{}' не указано действие для вызова", id))?
                .to_string();
            Ok(CommandAction::Action(action_name))
        }
        "workspace" => {
            let ws_name = child.entries().get(0)
                .and_then(|e| e.value().as_string())
                .ok_or_else(|| format!("У действия workspace в триггере '{}' не указан ID воркспейса", id))?
                .to_string();
            let monitor = child.entries().get(1)
                .and_then(|e| e.value().as_string())
                .map(|s| s.to_string());
            Ok(CommandAction::Workspace(ws_name, monitor))
        }
        other => Err(format!("Неподдерживаемый узел '{}' в триггере '{}'", other, id)),
    }
}

fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Пустой интервал".to_string());
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num = num_str.parse::<u64>()
        .map_err(|e| format!("Неверное число в интервале '{}': {}", s, e))?;
    match unit {
        "s" => Ok(std::time::Duration::from_secs(num)),
        "m" => Ok(std::time::Duration::from_secs(num * 60)),
        "h" => Ok(std::time::Duration::from_secs(num * 3600)),
        other => Err(format!("Неверная единица времени '{}' в интервале '{}'", other, s)),
    }
}
