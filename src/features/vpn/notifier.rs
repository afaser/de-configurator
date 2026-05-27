use notify_rust::Notification;
use tokio::sync::broadcast;
use crate::core::event_bus::SystemEvent;

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
    pub async fn run(mut rx: broadcast::Receiver<SystemEvent>) {
        while let Ok(event) = rx.recv().await {
            match event {
                SystemEvent::VpnTransitionStarted { display_name, has_interface, context, .. } => {
                    if !context.silent {
                        let title = format!("Переключение: {}", display_name);
                        let body = if has_interface {
                            format!("Установка соединения с {}...", display_name)
                        } else {
                            "Отключение всех VPN туннелей...".to_string()
                        };
                        notify_info(&title, &body);
                    }
                }
                SystemEvent::VpnStateChanged { display_name, ip_info, context, .. } => {
                    if !context.silent {
                        let title = format!("Успешно: {}", display_name);
                        let body = match ip_info {
                            Some(info) => format!("Соединение установлено\n{}", info),
                            None => "Все туннели успешно остановлены".to_string(),
                        };
                        notify_success(&title, &body);
                    }
                }
                SystemEvent::VpnStateTransitionFailed { display_name, error, context, .. } => {
                    if !context.silent {
                        let title = format!("Ошибка: {}", display_name);
                        notify_error(&title, &error);
                    }
                }
                _ => {}
            }
        }
    }
}
