use kdl::KdlDocument;

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub path: Option<String>,
}

impl IpcConfig {
    /// Парсит секцию `feature "ipc"` из общего KDL-документа
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let ipc_node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("ipc")
        });

        let mut path = None;

        let node = match ipc_node {
            Some(n) => n,
            None => return Ok(IpcConfig { path }),
        };

        if let Some(children) = node.children() {
            for child in children.nodes() {
                match child.name().value() {
                    "path" => {
                        path = child.entries().get(0)
                            .and_then(|e| e.value().as_string())
                            .map(String::from);
                    }
                    _ => {}
                }
            }
        }

        Ok(IpcConfig { path })
       }
}
