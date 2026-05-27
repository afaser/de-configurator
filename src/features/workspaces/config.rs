use kdl::KdlDocument;

#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    pub id: String,
    pub class: String,
}

#[derive(Debug, Clone)]
pub struct WorkspacesConfig {
    pub workspaces: Vec<WorkspaceConfig>,
}

impl WorkspacesConfig {
    pub fn parse_from_root_doc(doc: &KdlDocument) -> Result<Self, String> {
        let node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("workspaces")
        })
        .ok_or_else(|| "Секция feature \"workspaces\" не найдена в конфигурации".to_string())?;

        let mut workspaces = Vec::new();

        if let Some(children) = node.children() {
            for child in children.nodes() {
                if child.name().value() == "workspace" {
                    let id = child.entries().get(0)
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| "У узла workspace должен быть идентификатор".to_string())?
                        .to_string();

                    let class = child.entries().iter()
                        .find(|e| e.name().map(|i| i.value()) == Some("class"))
                        .and_then(|e| e.value().as_string())
                        .ok_or_else(|| format!("У воркспейса '{}' должен быть указан класс class", id))?
                        .to_string();

                    workspaces.push(WorkspaceConfig { id, class });
                }
            }
        }

        Ok(WorkspacesConfig { workspaces })
    }
}
