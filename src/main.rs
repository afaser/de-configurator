mod notifier;
mod vpn;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use vpn::{VpnManager, VpnState};

#[tokio::main]
async fn main() {
    let manager = GlobalHotKeyManager::new().unwrap();

    // Создаём 3 горячих клавиши
    let hotkey_off = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F9);
    let hotkey_warp = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F10);
    let hotkey_fin = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::F11);

    // Регистрируем клавиши
    manager
        .register(hotkey_off)
        .expect("Не удалось зарегистрировать хоткей для отключения VPN");
    manager
        .register(hotkey_warp)
        .expect("Не удалось зарегистрировать хоткей для Warp");
    manager
        .register(hotkey_fin)
        .expect("Не удалось зарегистрировать хоткей для Финляндии");

    let vpn_mgr = VpnManager::new();
    let current_state = vpn_mgr.detect_state().await;

    println!("Глобальные кейбинды зарегистрированы (текущий статус VPN: {}):", current_state);
    println!("  Ctrl + Shift + F9  -> Отключить VPN");
    println!("  Ctrl + Shift + F10 -> Подключить Warp");
    println!("  Ctrl + Shift + F11 -> Подключить Финляндию");

    // Канал для передачи событий из блокирующего потока в async-цикл
    let (tx, mut rx) = tokio::sync::mpsc::channel::<GlobalHotKeyEvent>(100);
    let receiver = GlobalHotKeyEvent::receiver();

    // Запускаем выделенный поток для прослушивания событий клавиш
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = receiver.recv() {
                if tx.blocking_send(event).is_err() {
                    break; // Канал закрыт, завершаем поток
                }
            }
        }
    });

    // Флаг для предотвращения параллельного запуска переключения
    let is_transitioning = Arc::new(AtomicBool::new(false));

    let mut last_off = tokio::time::Instant::now() - std::time::Duration::from_secs(5);
    let mut last_warp = tokio::time::Instant::now() - std::time::Duration::from_secs(5);
    let mut last_fin = tokio::time::Instant::now() - std::time::Duration::from_secs(5);

    // Основной асинхронный цикл обработки событий
    while let Some(event) = rx.recv().await {
        let (target_state, last_trigger) = if event.id == hotkey_off.id() {
            (Some(VpnState::Off), &mut last_off)
        } else if event.id == hotkey_warp.id() {
            (Some(VpnState::Warp), &mut last_warp)
        } else if event.id == hotkey_fin.id() {
            (Some(VpnState::Finland), &mut last_fin)
        } else {
            (None, &mut last_off)
        };

        if let Some(state) = target_state {
            let now = tokio::time::Instant::now();
            if now.duration_since(*last_trigger) < std::time::Duration::from_secs(1) {
                // Игнорируем дребезг клавиши в течение 1 секунды
                continue;
            }
            *last_trigger = now;

            if is_transitioning.load(Ordering::SeqCst) {
                notifier::notify_info("VPN", "Процесс переключения VPN уже запущен...");
                continue;
            }

            let is_transitioning_clone = Arc::clone(&is_transitioning);
            is_transitioning_clone.store(true, Ordering::SeqCst);

            tokio::spawn(async move {
                let vpn_mgr = VpnManager::new();

                // 1. Показываем начальное уведомление о подключении
                let (connecting_title, connecting_body) = match state {
                    VpnState::Off => ("Отключение VPN", "Выключение всех активных туннелей..."),
                    VpnState::Warp => ("Подключение к Warp", "Инициализация соединения с Warp..."),
                    VpnState::Finland => ("Подключение к Финляндии", "Инициализация соединения с Финляндией..."),
                };
                notifier::notify_info(connecting_title, connecting_body);

                // 2. Выполняем переход
                match vpn_mgr.switch_to(state).await {
                    Ok(maybe_ip_info) => {
                        let success_title = match state {
                            VpnState::Off => "VPN успешно отключен",
                            VpnState::Warp => "Подключено к Warp",
                            VpnState::Finland => "Подключено к Финляндии",
                        };

                        let success_body = match maybe_ip_info {
                            Some(ip_info) => format!("Соединение установлено\nIP: {} ({})", ip_info.query, ip_info.country),
                            None => "Все туннели успешно остановлены".to_string(),
                        };

                        notifier::notify_success(success_title, &success_body);
                    }
                    Err(err_msg) => {
                        let err_title = match state {
                            VpnState::Off => "Ошибка отключения VPN",
                            VpnState::Warp => "Ошибка подключения к Warp",
                            VpnState::Finland => "Ошибка подключения к Финляндии",
                        };
                        notifier::notify_error(err_title, &err_msg);
                    }
                }

                is_transitioning_clone.store(false, Ordering::SeqCst);
            });
        }
    }
}
