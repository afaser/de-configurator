use notify_rust::Notification;

const NOTIFICATION_ID: u32 = 4224;

pub fn notify_info(title: &str, body: &str) {
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

pub fn notify_success(title: &str, body: &str) {
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

pub fn notify_error(title: &str, body: &str) {
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
