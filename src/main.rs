mod features {
    pub mod vpn;
}

use std::path::PathBuf;
use kdl::KdlDocument;

#[tokio::main]
async fn main() {
    // 1. Загружаем общий KDL-конфиг
    let (config_doc, config_path) = match get_config_doc() {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Критическая ошибка конфигурации: {}", e);
            let _ = notify_rust::Notification::new()
                .summary("Ошибка конфигурации")
                .body(&e)
                .show();
            return;
        }
    };

    println!("Конфигурация успешно загружена из: {}", config_path.display());

    // 2. Проверяем и запускаем фичу VPN
    let vpn_enabled = is_feature_enabled(&config_doc, "vpn");
    if vpn_enabled {
        if let Err(e) = features::vpn::VpnFeature::start(&config_doc).await {
            eprintln!("Ошибка запуска VPN: {}", e);
            let _ = notify_rust::Notification::new()
                .summary("Ошибка запуска VPN")
                .body(&e)
                .show();
        }
    } else {
        println!("Фича 'vpn' отключена в конфигурации (пропишите enabled=true для запуска).");
    }

    // Здесь в будущем можно добавить запуск других фич аналогично:
    // if is_feature_enabled(&config_doc, "brightness") { ... }

    // 3. Удерживаем программу запущенной (ждем Ctrl+C)
    println!("Приложение запущено. Нажмите Ctrl+C для выхода.");
    let _ = tokio::signal::ctrl_c().await;
    println!("Завершение работы...");
}

/// Проверяет, включена ли фича с указанным именем в конфигурации
fn is_feature_enabled(doc: &KdlDocument, feature_name: &str) -> bool {
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

/// Находит и считывает файл конфигурации, при отсутствии создает дефолтный
fn get_config_doc() -> Result<(KdlDocument, PathBuf), String> {
    // 1. Попробуем прочитать локальный ./config.kdl
    let local_path = PathBuf::from("config.kdl");
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| format!("Не удалось прочитать local config.kdl: {}", e))?;
        let doc = content.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
        return Ok((doc, local_path));
    }

    // 2. Попробуем прочитать ~/.config/de-configurator/config.kdl
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

    // 3. Создаем дефолтный конфиг в текущей папке
    let default_content = r#"// Конфигурационный файл для de-configurator

feature "vpn" enabled=true {
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
}
"#;
    std::fs::write(&local_path, default_content)
        .map_err(|e| format!("Не удалось создать дефолтный config.kdl: {}", e))?;
    let doc = default_content.parse().map_err(|e| format!("Ошибка парсинга KDL: {}", e))?;
    Ok((doc, local_path))
}
