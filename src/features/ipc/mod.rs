pub mod config;

use tokio::io::AsyncReadExt;
use crate::core::event_bus::{EventBus, SystemEvent, EventContext, Initiator};
pub use config::IpcConfig;

pub struct IpcFeature;

impl IpcFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = IpcConfig::parse_from_root_doc(doc)?;
        let socket_path = config.path.unwrap_or_else(|| "/tmp/de-configurator.sock".to_string());

        println!("Фича IPC инициализирована. Unix socket: {}", socket_path);

        generate_bash_script(&socket_path)?;

        let _ = std::fs::remove_file(&socket_path);

        let listener = tokio::net::UnixListener::bind(&socket_path)
            .map_err(|e| format!("Не удалось привязаться к Unix socket '{}': {}", socket_path, e))?;

        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let bus = event_bus.clone();
                    tokio::spawn(async move {
                        let mut buf = vec![0; 1024];
                        match socket.read(&mut buf).await {
                            Ok(n) if n > 0 => {
                                let msg = String::from_utf8_lossy(&buf[..n]);
                                let target = msg.trim().to_string();
                                if !target.is_empty() {
                                    bus.publish(SystemEvent::RequestActionExecuteExported {
                                        action_id: target,
                                        context: EventContext {
                                            initiator: Initiator::Ipc,
                                            silent: false,
                                        },
                                    });
                                }
                            }
                            _ => {}
                        }
                    });
                }
            }
        });

        Ok(())
    }
}

fn generate_bash_script(socket_path: &str) -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME env var is not set".to_string())?;
    let bin_dir = std::path::PathBuf::from(home).join(".local/bin");
    
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Не удалось создать директорию ~/.local/bin: {}", e))?;

    let script_path = bin_dir.join("de-action");
    let script_content = format!(
        r#"#!/bin/sh
if [ -z "$1" ]; then
    echo "Usage: de-action <action_name>"
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
