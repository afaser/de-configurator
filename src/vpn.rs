use std::fmt;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnState {
    Off,
    Warp,
    Finland,
}

impl fmt::Display for VpnState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VpnState::Off => write!(f, "VPN выключен"),
            VpnState::Warp => write!(f, "Warp"),
            VpnState::Finland => write!(f, "Финляндия"),
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct IpInfo {
    pub query: String, // IP-адрес
    pub country: String,
}

pub struct VpnManager;

impl VpnManager {
    pub fn new() -> Self {
        Self
    }

    /// Проверяет, поднят ли сетевой интерфейс с указанным именем
    pub async fn is_interface_up(&self, name: &str) -> bool {
        let path = format!("/sys/class/net/{}", name);
        fs::metadata(&path).await.is_ok()
    }

    /// Определяет текущее состояние VPN на основе активных сетевых интерфейсов
    pub async fn detect_state(&self) -> VpnState {
        if self.is_interface_up("fl").await {
            VpnState::Finland
        } else if self.is_interface_up("w").await {
            VpnState::Warp
        } else {
            VpnState::Off
        }
    }

    /// Переключает VPN в целевое состояние
    pub async fn switch_to(&self, target: VpnState) -> Result<Option<IpInfo>, String> {
        match target {
            VpnState::Off => {
                // Если Warp или Финляндия подняты, опускаем их
                if self.is_interface_up("w").await {
                    Self::run_cmd("doas", &["wg-quick", "down", "w"]).await?;
                }
                if self.is_interface_up("fl").await {
                    Self::run_cmd("doas", &["wg-quick", "down", "fl"]).await?;
                }
                Ok(None)
            }
            VpnState::Warp => {
                // Отключаем Финляндию, если она была активна
                if self.is_interface_up("fl").await {
                    Self::run_cmd("doas", &["wg-quick", "down", "fl"]).await?;
                }
                // Подключаем Warp, если он ещё не подключен
                if !self.is_interface_up("w").await {
                    Self::run_cmd("doas", &["wg-quick", "up", "w"]).await?;
                }
                
                // Получаем новый IP
                let ip_info = self.retry_fetch_ip_info().await?;
                Ok(Some(ip_info))
            }
            VpnState::Finland => {
                // Отключаем Warp, если он был активен
                if self.is_interface_up("w").await {
                    Self::run_cmd("doas", &["wg-quick", "down", "w"]).await?;
                }
                // Подключаем Финляндию, если она ещё не подключена
                if !self.is_interface_up("fl").await {
                    Self::run_cmd("doas", &["wg-quick", "up", "fl"]).await?;
                }
                
                // Получаем новый IP
                let ip_info = self.retry_fetch_ip_info().await?;
                Ok(Some(ip_info))
            }
        }
    }

    /// Запуск внешней команды
    async fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
        let output = tokio::process::Command::new(cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Не удалось запустить команду '{} {}': {}", cmd, args.join(" "), e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!(
                "Команда '{} {}' завершилась ошибкой: {}",
                cmd,
                args.join(" "),
                stderr.trim()
            ))
        }
    }

    /// Повторные попытки получить информацию об IP-адресе с таймаутом
    async fn retry_fetch_ip_info(&self) -> Result<IpInfo, String> {
        let mut last_err = String::new();
        // Делаем до 4 попыток с увеличивающимся интервалом, чтобы дать маршрутизации WireGuard стабилизироваться
        for i in 0..4 {
            let sleep_ms = 400 * (i + 1);
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
            
            match self.fetch_ip_info().await {
                Ok(info) => return Ok(info),
                Err(e) => {
                    last_err = e;
                }
            }
        }
        Err(format!("Не удалось получить IP адрес после подключения: {}", last_err))
    }

    /// Запрос IP-адреса и страны
    async fn fetch_ip_info(&self) -> Result<IpInfo, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .map_err(|e| e.to_string())?;

        let res = client
            .get("http://ip-api.com/json")
            .send()
            .await
            .map_err(|e| format!("Сеть недоступна: {}", e))?;

        let info = res
            .json::<IpInfo>()
            .await
            .map_err(|e| format!("Ошибка разбора JSON: {}", e))?;

        Ok(info)
    }
}
