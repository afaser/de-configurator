use std::process::Stdio;

pub struct DaemonManager;

impl DaemonManager {
    pub fn new() -> Self {
        Self
    }

    /// Проверяет, запущен ли процесс с указанным именем (с помощью pgrep -x)
    pub async fn is_running(&self, name: &str) -> bool {
        let status = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        status.map(|s| s.success()).unwrap_or(false)
    }

    /// Получает список PID-ов процессов по их точному имени
    pub async fn get_pids(&self, name: &str) -> Vec<u32> {
        let output = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await;

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                return stdout
                    .lines()
                    .filter_map(|l| l.trim().parse::<u32>().ok())
                    .collect();
            }
        }
        vec![]
    }

    /// Завершает процессы по PID
    pub async fn kill_pids(&self, pids: &[u32]) -> Result<(), String> {
        for &pid in pids {
            let status = tokio::process::Command::new("kill")
                .arg(pid.to_string())
                .status()
                .await
                .map_err(|e| format!("Не удалось выполнить kill для PID {}: {}", pid, e))?;
            
            if !status.success() {
                // Если обычный SIGTERM не помог, завершаем жестко через SIGKILL (-9)
                let _ = tokio::process::Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .status()
                    .await;
            }
        }
        Ok(())
    }

    /// Запускает демон в фоновом режиме и возвращает его Child handle
    pub fn start_daemon(&self, cmd_parts: &[String]) -> Result<tokio::process::Child, String> {
        if cmd_parts.is_empty() {
            return Err("Пустая команда запуска".to_string());
        }
        let cmd = &cmd_parts[0];
        let args = &cmd_parts[1..];

        tokio::process::Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Не удалось запустить процесс '{}': {}", cmd, e))
    }
}
