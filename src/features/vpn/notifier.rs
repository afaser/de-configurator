use notify_rust::Notification;
use tokio::sync::mpsc;
use super::event::VpnDomainEvent;

const NOTIFICATION_ID: u32 = 4224;

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

fn notify_success(title: &str, body: &str) {
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

pub struct VpnNotificationService;

impl VpnNotificationService {
    /// Запуск обработчика доменных событий для вывода уведомлений
    pub async fn run(mut rx: mpsc::Receiver<VpnDomainEvent>) {
        while let Some(event) = rx.recv().await {
            match event {
                VpnDomainEvent::TransitionStarted { display_name, has_interface, .. } => {
                    let title = format!("Переключение: {}", display_name);
                    let body = if has_interface {
                        format!("Установка соединения с {}...", display_name)
                    } else {
                        "Отключение всех VPN туннелей...".to_string()
                    };
                    notify_info(&title, &body);
                }
                VpnDomainEvent::TransitionCompleted { display_name, ip_info, .. } => {
                    let title = format!("Успешно: {}", display_name);
                    let body = match ip_info {
                        Some(info) => format!("Соединение установлено\nIP: {} ({}, {})", info.query, info.city, info.country),
                        None => "Все туннели успешно остановлены".to_string(),
                    };
                    notify_success(&title, &body);
                }
                VpnDomainEvent::TransitionFailed { display_name, error, .. } => {
                    let title = format!("Ошибка: {}", display_name);
                    notify_error(&title, &error);
                }
            }
        }
    }
}
