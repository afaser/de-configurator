use std::path::PathBuf;
use kdl::KdlDocument;

pub fn is_feature_enabled(doc: &KdlDocument, feature_name: &str) -> bool {
    doc.nodes().iter().find(|n| {
        n.name().value() == "feature"
        && n.entries().get(0).and_then(|e| e.value().as_string()) == Some(feature_name)
    })
    .and_then(|n| {
        n.entries().iter().find(|e| e.name().map(|id| id.value()) == Some("enabled"))
    })
    .and_then(|e| e.value().as_bool())
    .unwrap_or(false)
}

pub fn get_config_doc() -> Result<(KdlDocument, PathBuf), String> {
    let local_path = PathBuf::from("config.kdl");
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| format!("Не удалось прочитать local config.kdl: {}", e))?;
        let doc = content.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
        return Ok((doc, local_path));
    }

    let home = std::env::var("HOME").ok();
    if let Some(h) = home {
        let config_path = PathBuf::from(h).join(".config/de-configurator/config.kdl");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Не удалось прочитать ~/.config/de-configurator/config.kdl: {}", e))?;
            let doc = content.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
            return Ok((doc, config_path));
        }
    }

    let default_content = r#"feature "vpn" enabled=true {
    state "off" {
        display-name "VPN выключен"
    }

    state "warp" {
        display-name "Warp"
        interface "w"
        up-cmd "doas" "wg-quick" "up" "w"
        down-cmd "doas" "wg-quick" "down" "w"
    }

    state "finland" {
        display-name "Финляндия"
        interface "fl"
        up-cmd "doas" "wg-quick" "up" "fl"
        down-cmd "doas" "wg-quick" "down" "fl"
    }
}

feature "keybinds" enabled=true {
    bind "ctrl+shift+t" {
        run "alacritty"
    }
    
    bind "ctrl+shift+w" {
        vpn "warp"
    }

    bind "ctrl+shift+o" {
        vpn "off"
    }
}
"#;
    std::fs::write(&local_path, default_content)
        .map_err(|e| format!("Не удалось создать дефолтный config.kdl: {}", e))?;
    let doc = default_content.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
    Ok((doc, local_path))
}
