use tokio::fs;
use crate::config::VpnStateConfig;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct IpInfo {
    pub query: String, // IP-адрес
    pub country: String,
    pub city: String,
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
    pub async fn detect_state(&self, states: &[VpnStateConfig]) -> VpnStateConfig {
        for state in states {
            if let Some(ref iface) = state.interface {
                if self.is_interface_up(iface).await {
                    return state.clone();
                }
            }
        }
        
        // Если ни один интерфейс не поднят, возвращаем первое состояние без интерфейса (обычно "off")
        for state in states {
            if state.interface.is_none() {
                return state.clone();
            }
        }

        // Резервный вариант, если в конфиге вообще нет состояния без интерфейса
        VpnStateConfig {
            id: "off".to_string(),
            hotkey_str: "".to_string(),
            hotkey: global_hotkey::hotkey::HotKey::new(None, global_hotkey::hotkey::Code::F9),
            display_name: "VPN выключен".to_string(),
            interface: None,
            up_cmd: vec![],
            down_cmd: vec![],
        }
    }

    /// Переключает VPN в целевое состояние
    pub async fn switch_to(&self, target: &VpnStateConfig, all_states: &[VpnStateConfig]) -> Result<Option<IpInfo>, String> {
        // 1. Опускаем все другие интерфейсы, которые сейчас подняты
        for state in all_states {
            if state.id != target.id {
                if let Some(ref iface) = state.interface {
                    if self.is_interface_up(iface).await {
                        if !state.down_cmd.is_empty() {
                            Self::run_cmd(&state.down_cmd).await?;
                        }
                    }
                }
            }
        }

        // 2. Поднимаем целевой интерфейс, если он ещё не поднят
        if let Some(ref iface) = target.interface {
            if !self.is_interface_up(iface).await {
                if !target.up_cmd.is_empty() {
                    Self::run_cmd(&target.up_cmd).await?;
                }
            }
            // Получаем новый IP
            let ip_info = self.retry_fetch_ip_info().await?;
            Ok(Some(ip_info))
        } else {
            // Если у целевого состояния нет интерфейса (например, "off"), 
            // то все остальные интерфейсы уже потушены на первом шаге
            Ok(None)
        }
    }

    /// Запуск внешней команды
    async fn run_cmd(cmd_parts: &[String]) -> Result<(), String> {
        if cmd_parts.is_empty() {
            return Ok(());
        }
        let cmd = &cmd_parts[0];
        let args = &cmd_parts[1..];

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
        // Делаем до 4 попыток с увеличивающимся интервалом, чтобы дать маршрутизации стабилизироваться
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
