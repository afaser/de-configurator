use kdl::KdlDocument;
use crate::features::actions::event::CommandAction;
use crate::features::daemons::event::DaemonAction;

#[derive(Debug, Clone)]
pub enum TriggerType {
    Interval { duration: std::time::Duration },
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
    /// Парсит секцию `feature "triggers"` из KDL-конфигурации
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

                    if type_str != "interval" {
                        return Err(format!("Неподдерживаемый тип триггера '{}' у '{}'. В триггерах поддерживается только type=\"interval\"", type_str, id));
                    }

                    let mut duration = None;
                    let mut trigger_command = None;

                    if let Some(trigger_children) = trigger_node.children() {
                        for child in trigger_children.nodes() {
                            match child.name().value() {
                                "interval" => {
                                    let duration_str = child.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У триггера '{}' должно быть указано время интервала", id))?;
                                    duration = Some(parse_duration(duration_str)?);
                                }
                                "run" => {
                                    let parts: Vec<String> = child.entries().iter()
                                        .filter_map(|e| e.value().as_string().map(String::from))
                                        .collect();
                                    trigger_command = Some(CommandAction::Run(parts));
                                }
                                "vpn" => {
                                    let state = child.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У триггера '{}' не указано состояние vpn", id))?
                                        .to_string();
                                    trigger_command = Some(CommandAction::Vpn(state));
                                }
                                "daemon" => {
                                    let daemon_id = child.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У триггера '{}' не указан ID демона", id))?
                                        .to_string();
                                    let action_str = child.entries().get(1)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У триггера '{}' не указано действие демона", id))?;
                                    let daemon_action = match action_str {
                                        "on" => DaemonAction::On,
                                        "off" => DaemonAction::Off,
                                        "toggle" => DaemonAction::Toggle,
                                        other => return Err(format!("Неподдерживаемое действие демона '{}' в триггере '{}'", other, id)),
                                    };
                                    trigger_command = Some(CommandAction::Daemon(daemon_id, daemon_action));
                                }
                                "action" => {
                                    let action_name = child.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .ok_or_else(|| format!("У триггера '{}' не указано действие для вызова", id))?
                                        .to_string();
                                    trigger_command = Some(CommandAction::Action(action_name));
                                }
                                other => return Err(format!("Неподдерживаемый узел '{}' в триггере '{}'", other, id)),
                            }
                        }
                    }

                    let dur = duration.ok_or_else(|| format!("У интервального триггера '{}' должен быть указан интервал (например, interval \"5m\")", id))?;
                    let cmd = trigger_command.ok_or_else(|| format!("У интервального триггера '{}' должно быть указано выполняемое действие (run, vpn, daemon или action)", id))?;

                    triggers.push(TriggerConfig {
                        id,
                        trigger_type: TriggerType::Interval { duration: dur },
                        command: cmd,
                    });
                }
            }
        }

        Ok(TriggersConfig { triggers })
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
