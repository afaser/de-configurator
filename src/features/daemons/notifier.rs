use notify_rust::Notification;
use tokio::sync::mpsc;
use super::event::DaemonDomainEvent;

const NOTIFICATION_ID: u32 = 4225; // Отдельный ID, чтобы не перебивать уведомления VPN

fn notify_info(title: &str, body: &str) {
    if let Err(e) = Notification::new()
        .id(NOTIFICATION_ID)
        .summary(title)
        .body(body)
        .appname("de-configurator")
        .show()
    {
        eprintln!("Не удалось отправить уведомление: {}", e);
    }
}

fn notify_error(title: &str, body: &str) {
    if let Err(e) = Notification::new()
        .id(NOTIFICATION_ID)
        .summary(title)
        .body(body)
        .appname("de-configurator")
        .urgency(notify_rust::Urgency::Critical)
        .show()
    {
        eprintln!("Не удалось отправить уведомление: {}", e);
    }
}

pub struct DaemonNotificationService;

impl DaemonNotificationService {
    /// Запуск обработчика доменных событий для вывода уведомлений демонов
    pub async fn run(mut rx: mpsc::Receiver<DaemonDomainEvent>) {
        while let Some(event) = rx.recv().await {
            match event {
                DaemonDomainEvent::StateChanged { display_name, is_running, .. } => {
                    let title = format!("Демон: {}", display_name);
                    let body = if is_running {
                        format!("Процесс {} запущен", display_name)
                    } else {
                        format!("Процесс {} остановлен", display_name)
                    };
                    notify_info(&title, &body);
                }
                DaemonDomainEvent::TransitionFailed { display_name, action, error, .. } => {
                    let title = format!("Ошибка демона: {}", display_name);
                    let body = format!("Не удалось выполнить '{}' для {}: {}", action, display_name, error);
                    notify_error(&title, &body);
                }
            }
        }
    }
}
