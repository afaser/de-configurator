use kdl::KdlDocument;
use std::str::FromStr;
use global_hotkey::hotkey::HotKey;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct VpnStateConfig {
    pub id: String,
    pub hotkey_str: String,
    pub hotkey: HotKey,
    pub display_name: String,
    pub interface: Option<String>,
    pub up_cmd: Vec<String>,
    pub down_cmd: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub states: Vec<VpnStateConfig>,
}

impl Config {
    pub fn load_from_str(s: &str) -> Result<Self, String> {
        let doc: KdlDocument = s.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
        let mut states = Vec::new();

        for node in doc.nodes() {
            if node.name().value() == "state" {
                let id = node.entries().get(0)
                    .and_then(|e| e.value().as_string())
                    .ok_or_else(|| "У узла state должен быть строковый идентификатор".to_string())?
                    .to_string();

                let mut hotkey_str = String::new();
                let mut display_name = id.clone();
                let mut interface = None;
                let mut up_cmd = Vec::new();
                let mut down_cmd = Vec::new();

                if let Some(children) = node.children() {
                    for child in children.nodes() {
                        match child.name().value() {
                            "hotkey" => {
                                hotkey_str = child.entries().get(0)
                                    .and_then(|e| e.value().as_string())
                                    .unwrap_or("")
                                    .to_string();
                            }
                            "display-name" => {
                                display_name = child.entries().get(0)
                                    .and_then(|e| e.value().as_string())
                                    .unwrap_or(&id)
                                    .to_string();
                            }
                            "interface" => {
                                interface = child.entries().get(0)
                                    .and_then(|e| e.value().as_string())
                                    .map(String::from);
                            }
                            "up-cmd" => {
                                up_cmd = child.entries().iter()
                                    .filter_map(|e| e.value().as_string().map(String::from))
                                    .collect();
                            }
                            "down-cmd" => {
                                down_cmd = child.entries().iter()
                                    .filter_map(|e| e.value().as_string().map(String::from))
                                    .collect();
                            }
                            _ => {}
                        }
                    }
                }

                if hotkey_str.is_empty() {
                    return Err(format!("У узла state '{}' должен быть указан hotkey", id));
                }

                let clean_hotkey_str = hotkey_str.to_lowercase();
                let hotkey = HotKey::from_str(&clean_hotkey_str)
                    .map_err(|e| format!("Не удалось распарсить хоткей '{}' для состояния '{}': {:?}", hotkey_str, id, e))?;

                states.push(VpnStateConfig {
                    id,
                    hotkey_str,
                    hotkey,
                    display_name,
                    interface,
                    up_cmd,
                    down_cmd,
                });
            }
        }

        Ok(Config { states })
    }
}

pub fn get_config() -> Result<(Config, PathBuf), String> {
    // 1. Попробуем прочитать локальный ./config.kdl
    let local_path = PathBuf::from("config.kdl");
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| format!("Не удалось прочитать local config.kdl: {}", e))?;
        let config = Config::load_from_str(&content)?;
        return Ok((config, local_path));
    }

    // 2. Попробуем прочитать ~/.config/de-configurator/config.kdl
    let home = std::env::var("HOME").ok();
    if let Some(h) = home {
        let config_path = PathBuf::from(h).join(".config/de-configurator/config.kdl");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Не удалось прочитать ~/.config/de-configurator/config.kdl: {}", e))?;
            let config = Config::load_from_str(&content)?;
            return Ok((config, config_path));
        }
    }

    // 3. Создаем дефолтный конфиг в текущей папке
    let default_content = r#"// Конфигурационный файл для de-configurator

state "off" {
    hotkey "ctrl+shift+f9"
    display-name "VPN выключен"
}

state "warp" {
    hotkey "ctrl+shift+f10"
    display-name "Warp"
    interface "w"
    up-cmd "doas" "wg-quick" "up" "w"
    down-cmd "doas" "wg-quick" "down" "w"
}

state "finland" {
    hotkey "ctrl+shift+f11"
    display-name "Финляндия"
    interface "fl"
    up-cmd "doas" "wg-quick" "up" "fl"
    down-cmd "doas" "wg-quick" "down" "fl"
}
"#;
    std::fs::write(&local_path, default_content)
        .map_err(|e| format!("Не удалось создать дефолтный config.kdl: {}", e))?;
    let config = Config::load_from_str(default_content)?;
    Ok((config, local_path))
}
