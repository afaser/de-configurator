use kdl::KdlDocument;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub id: String,
    pub display_name: String,
    pub autostart: bool,
    pub start_cmd: Vec<String>,
    pub stop_cmd: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DaemonsConfig {
    pub daemons: Vec<DaemonConfig>,
}

impl DaemonsConfig {
    /// Парсит секцию `feature "daemons"` из общего KDL-документа
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let daemons_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("daemons")
        })
        .ok_or_else(|| "Секция feature \"daemons\" не найдена в конфигурации".to_string())?;

        let mut daemons = Vec::new();

        if let Some(children) = daemons_node.children() {
            for node in children.nodes() {
                if node.name().value() == "daemon" {
                    let id = node.entries().get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| "У узла daemon должен быть строковый идентификатор".to_string())?
                        .to_string();

                    // Парсим свойство autostart=true/false (по умолчанию false)
                    let autostart = node.entries().iter()
                        .find(|e| e.name().map(|i| i.value()) == Some("autostart"))
                        .and_then(|e| e.value().as_bool())
                        .unwrap_or(false);

                    let mut display_name = id.clone();
                    let mut start_cmd = Vec::new();
                    let mut stop_cmd = None;

                    if let Some(daemon_children) = node.children() {
                        for child in daemon_children.nodes() {
                            match child.name().value() {
                                "display-name" => {
                                    display_name = child.entries().get(0)
                                        .and_then(|e| e.value().as_string())
                                        .unwrap_or(&id)
                                        .to_string();
                                }
                                "start-cmd" => {
                                    start_cmd = child.entries().iter()
                                        .filter_map(|e| e.value().as_string().map(String::from))
                                        .collect();
                                }
                                "stop-cmd" => {
                                    stop_cmd = Some(child.entries().iter()
                                        .filter_map(|e| e.value().as_string().map(String::from))
                                        .collect());
                                }
                                _ => {}
                            }
                        }
                    }

                    if start_cmd.is_empty() {
                        return Err(format!("У демона '{}' не указана команда запуска start-cmd", id));
                    }

                    daemons.push(DaemonConfig {
                        id,
                        display_name,
                        autostart,
                        start_cmd,
                        stop_cmd,
                    });
                }
            }
        }

        Ok(DaemonsConfig { daemons })
    }
}
