pub mod config;

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::io::AsyncReadExt;
use crate::features::actions::event::ActionInputEvent;
pub use config::{TriggersConfig, TriggerType};

pub struct TriggersFeature;

impl TriggersFeature {
    /// Запуск фичи Triggers
    pub async fn start(
        doc: &kdl::KdlDocument,
        actions_tx: mpsc::Sender<ActionInputEvent>,
    ) -> Result<(), String> {
        let config = TriggersConfig::parse_from_root_doc(doc)?;
        let triggers = Arc::new(config.triggers);

        println!("Фича Triggers инициализирована. Зарегистрировано триггеров: {}", triggers.len());

        for trigger in triggers.iter() {
            let actions_tx_clone = actions_tx.clone();
            let trigger_id = trigger.id.clone();

            match &trigger.trigger_type {
                TriggerType::Interval { duration } => {
                    let duration = *duration;
                    let cmd = trigger.command.clone().ok_or_else(|| {
                        format!("У интервального триггера '{}' должна быть указана команда", trigger_id)
                    })?;
                    
                    println!("  trigger '{}' (интервал: {:?})", trigger_id, duration);

                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(duration);
                        // Пропускаем первый немедленный тик
                        interval.tick().await;
                        loop {
                            interval.tick().await;
                            let _ = actions_tx_clone.send(ActionInputEvent::ExecuteCommand(cmd.clone())).await;
                        }
                    });
                }
                TriggerType::Socket { path } => {
                    let socket_path = path.clone().unwrap_or_else(|| "/tmp/de-configurator.sock".to_string());
                    println!("  trigger '{}' (Unix socket: {})", trigger_id, socket_path);

                    // Создаем bash-скрипт de-action
                    generate_bash_script(&socket_path)?;

                    // Удаляем старый файл сокета, если он остался
                    let _ = std::fs::remove_file(&socket_path);

                    let listener = tokio::net::UnixListener::bind(&socket_path)
                        .map_err(|e| format!("Не удалось привязаться к Unix socket '{}': {}", socket_path, e))?;

                    let triggers_list = triggers.clone();

                    tokio::spawn(async move {
                        loop {
                            if let Ok((mut socket, _)) = listener.accept().await {
                                let tx = actions_tx_clone.clone();
                                let list = triggers_list.clone();
                                tokio::spawn(async move {
                                    let mut buf = vec![0; 1024];
                                    match socket.read(&mut buf).await {
                                        Ok(n) if n > 0 => {
                                            let msg = String::from_utf8_lossy(&buf[..n]);
                                            let target = msg.trim().to_string();
                                            if target.is_empty() {
                                                return;
                                            }
                                            
                                            // 1. Проверяем, есть ли триггер с таким именем
                                            let matched_trigger = list.iter().find(|t| t.id == target);
                                            if let Some(t) = matched_trigger {
                                                if let Some(ref cmd) = t.command {
                                                    let _ = tx.send(ActionInputEvent::ExecuteCommand(cmd.clone())).await;
                                                }
                                            } else {
                                                // 2. Иначе трактуем как прямое название экшена
                                                let _ = tx.send(ActionInputEvent::ExecuteAction(target)).await;
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            }
                        }
                    });
                }
            }
        }

        Ok(())
    }
}

/// Генерирует bash-скрипт ~/.local/bin/de-action для вызова действий через Unix-сокет
fn generate_bash_script(socket_path: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var is not set".to_string())?;
    let bin_dir = std::path::PathBuf::from(home).join(".local/bin");
    
    // Создаем директорию, если ее нет
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Не удалось создать директорию ~/.local/bin: {}", e))?;

    let script_path = bin_dir.join("de-action");
    let script_content = format!(
        r#"#!/bin/sh
# Этот скрипт сгенерирован автоматически de-configurator.
# Он передает переданный аргумент в Unix domain socket.

if [ -z "$1" ]; then
    echo "Usage: de-action <action_or_trigger_name>"
    exit 1
fi

SOCKET_PATH="{}"

if [ -S "$SOCKET_PATH" ]; then
    if command -v socat >/dev/null 2>&1; then
        echo "$1" | socat - "UNIX-CONNECT:$SOCKET_PATH"
    elif command -v nc >/dev/null 2>&1; then
        echo "$1" | nc -U "$SOCKET_PATH"
    else
        echo "Error: Neither 'socat' nor 'nc' (netcat) with UNIX socket support is installed." >&2
        exit 1
    fi
else
    echo "de-configurator is not running or socket $SOCKET_PATH is missing." >&2
    exit 1
fi
"#,
        socket_path
    );

    std::fs::write(&script_path, script_content)
        .map_err(|e| format!("Не удалось записать скрипт de-action: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&script_path)
            .map_err(|e| format!("Не удалось прочитать права de-action: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)
            .map_err(|e| format!("Не удалось выставить права на исполнение для de-action: {}", e))?;
    }

    Ok(())
}
