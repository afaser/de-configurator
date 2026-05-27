mod core {
    pub mod hotkey_dispatcher;
}
mod features {
    pub mod vpn;
    pub mod keybinds;
    pub mod daemons;
}

use std::path::PathBuf;
use std::sync::Arc;
use kdl::KdlDocument;
use global_hotkey::GlobalHotKeyManager;
use core::hotkey_dispatcher::HotkeyDispatcher;

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

    // 2. Инициализируем глобальный HotKeyManager и HotkeyDispatcher
    let hotkey_manager = Arc::new(GlobalHotKeyManager::new().unwrap());
    let mut dispatcher = HotkeyDispatcher::new(hotkey_manager.clone());

    // 3. Проверяем и запускаем фичу VPN (теперь она чисто реактивная, без хоткеев)
    let vpn_enabled = is_feature_enabled(&config_doc, "vpn");
    let mut vpn_tx = None;
    if vpn_enabled {
        match features::vpn::VpnFeature::start(&config_doc).await {
            Ok(tx) => {
                vpn_tx = Some(tx);
            }
            Err(e) => {
                eprintln!("Ошибка запуска VPN: {}", e);
                let _ = notify_rust::Notification::new()
                    .summary("Ошибка запуска VPN")
                    .body(&e)
                    .show();
            }
        }
    } else {
        println!("Фича 'vpn' отключена в конфигурации.");
    }

    // 3.5. Проверяем и запускаем фичу Daemons (менеджер демонов)
    let daemons_enabled = is_feature_enabled(&config_doc, "daemons");
    let mut daemons_tx = None;
    if daemons_enabled {
        match features::daemons::DaemonsFeature::start(&config_doc).await {
            Ok(tx) => {
                daemons_tx = Some(tx);
            }
            Err(e) => {
                eprintln!("Ошибка запуска Daemons: {}", e);
                let _ = notify_rust::Notification::new()
                    .summary("Ошибка запуска Daemons")
                    .body(&e)
                    .show();
            }
        }
    } else {
        println!("Фича 'daemons' отключена в конфигурации.");
    }

    // 4. Проверяем и запускаем фичу Keybinds (она регистрирует все хоткеи)
    let keybinds_enabled = is_feature_enabled(&config_doc, "keybinds");
    if keybinds_enabled {
        if let Err(e) = features::keybinds::KeybindsFeature::start(
            &config_doc,
            hotkey_manager.clone(),
            &mut dispatcher,
            vpn_tx,
            daemons_tx,
        ).await {
            eprintln!("Ошибка запуска Keybinds: {}", e);
            let _ = notify_rust::Notification::new()
                .summary("Ошибка запуска Keybinds")
                .body(&e)
                .show();
        }
    } else {
        println!("Фича 'keybinds' отключена в конфигурации.");
    }

    // 5. Запускаем диспетчер событий клавиатуры
    dispatcher.start();

    // 6. Удерживаем программу запущенной (ждем Ctrl+C)
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
    // Открыть терминал
    bind "ctrl+shift+t" {
        run "alacritty"
    }
    
    // Переключить VPN на Warp
    bind "ctrl+shift+w" {
        vpn "warp"
    }

    // Выключить VPN
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
