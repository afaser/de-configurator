use kdl::KdlDocument;
use std::str::FromStr;
use global_hotkey::hotkey::HotKey;

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
pub struct VpnConfig {
    pub states: Vec<VpnStateConfig>,
}

impl VpnConfig {
    /// Парсит секцию `feature "vpn"` из общего KDL-документа
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let vpn_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("vpn")
        })
        .ok_or_else(|| "Секция feature \"vpn\" не найдена в конфигурации".to_string())?;

        let mut states = Vec::new();

        if let Some(children) = vpn_node.children() {
            for node in children.nodes() {
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

                    if let Some(state_children) = node.children() {
                        for child in state_children.nodes() {
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
        }

        Ok(VpnConfig { states })
    }
}
