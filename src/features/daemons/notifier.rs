use notify_rust::Notification;
use tokio::sync::broadcast;
use crate::core::event_bus::{SystemEvent, DaemonAction};

const NOTIFICATION_ID: u32 = 4225;

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
    pub async fn run(mut rx: broadcast::Receiver<SystemEvent>) {
        while let Ok(event) = rx.recv().await {
            match event {
                SystemEvent::DaemonStateChanged { display_name, is_running, context, .. } => {
                    if !context.silent {
                        let title = format!("Демон: {}", display_name);
                        let body = if is_running {
                            format!("Процесс {} запущен", display_name)
                        } else {
                            format!("Процесс {} остановлен", display_name)
                        };
                        notify_info(&title, &body);
                    }
                }
                SystemEvent::DaemonStateTransitionFailed { display_name, action, error, context, .. } => {
                    if !context.silent {
                        let title = format!("Ошибка демона: {}", display_name);
                        let action_str = match action {
                            DaemonAction::On => "включение",
                            DaemonAction::Off => "выключение",
                            DaemonAction::Toggle => "переключение",
                        };
                        let body = format!("Не удалось выполнить '{}' для {}: {}", action_str, display_name, error);
                        notify_error(&title, &body);
                    }
                }
                _ => {}
            }
        }
    }
}
