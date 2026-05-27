use kdl::KdlDocument;
use std::str::FromStr;
use global_hotkey::hotkey::HotKey;
use crate::features::daemons::event::DaemonAction;
use crate::features::actions::event::CommandAction;

#[derive(Debug, Clone)]
pub struct KeybindConfig {
    pub hotkey_str: String,
    pub hotkey: HotKey,
    pub action: CommandAction,
}

#[derive(Debug, Clone)]
pub struct KeybindsConfig {
    pub binds: Vec<KeybindConfig>,
}

impl KeybindsConfig {
    /// Парсит секцию `feature "keybinds"` из общего KDL-документа
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let keybinds_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("keybinds")
        })
        .ok_or_else(|| "Секция feature \"keybinds\" не найдена в конфигурации".to_string())?;

        let mut binds = Vec::new();

        if let Some(children) = keybinds_node.children() {
            for node in children.nodes() {
                if node.name().value() == "bind" {
                    let hotkey_str = node.entries().get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| "У узла bind должно быть указано сочетание клавиш (строка)".to_string())?
                        .to_string();

                    // Ищем первый дочерний узел, который является действием
                    let child_node = node.children()
                        .and_then(|c| c.nodes().first())
                        .ok_or_else(|| format!("У бинда '{}' должно быть указано действие (run, vpn, daemon или action)", hotkey_str))?;

                    let action = match child_node.name().value() {
                        "run" => {
                            let cmd_parts: Vec<String> = child_node.entries().iter()
                                .filter_map(|e| e.value().as_string().map(String::from))
                                .collect();
                            if cmd_parts.is_empty() {
                                return Err(format!("У действия run для бинда '{}' не указана команда", hotkey_str));
                            }
                            CommandAction::Run(cmd_parts)
                        }
                        "vpn" => {
                            let state_id = child_node.entries().get(0)
                                .and_then(|e| e.value().as_string())
                                .ok_or_else(|| format!("У действия vpn для бинда '{}' не указано целевое состояние", hotkey_str))?
                                .to_string();
                            CommandAction::Vpn(state_id)
                        }
                        "daemon" => {
                            let daemon_id = child_node.entries().get(0)
                                .and_then(|e| e.value().as_string())
                                .ok_or_else(|| format!("У действия daemon для бинда '{}' не указан идентификатор демона", hotkey_str))?
                                .to_string();
                            let action_str = child_node.entries().get(1)
                                .and_then(|e| e.value().as_string())
                                .ok_or_else(|| format!("У действия daemon для бинда '{}' не указано действие (on, off, toggle)", hotkey_str))?;
                            
                            let daemon_action = match action_str {
                                "on" => DaemonAction::On,
                                "off" => DaemonAction::Off,
                                "toggle" => DaemonAction::Toggle,
                                other => {
                                    return Err(format!("Неподдерживаемое действие демона '{}' для бинда '{}' (разрешены: on, off, toggle)", other, hotkey_str));
                                }
                            };
                            CommandAction::Daemon(daemon_id, daemon_action)
                        }
                        "action" => {
                            let action_id = child_node.entries().get(0)
                                .and_then(|e| e.value().as_string())
                                .ok_or_else(|| format!("У действия action для бинда '{}' не указано целевое действие", hotkey_str))?
                                .to_string();
                            CommandAction::Action(action_id)
                        }
                        other => {
                            return Err(format!("Неподдерживаемое действие '{}' для бинда '{}'", other, hotkey_str));
                        }
                    };

                    let clean_hotkey_str = hotkey_str.to_lowercase();
                    let hotkey = HotKey::from_str(&clean_hotkey_str)
                        .map_err(|e| format!("Не удалось распарсить хоткей '{}': {:?}", hotkey_str, e))?;

                    binds.push(KeybindConfig {
                        hotkey_str,
                        hotkey,
                        action,
                    });
                }
            }
        }

        Ok(KeybindsConfig { binds })
    }
}
