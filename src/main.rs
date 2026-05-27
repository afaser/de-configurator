mod notifier;
mod vpn;
mod config;
mod event;
mod hotkey;
mod app;

use app::App;
use hotkey::HotkeyListener;
use notifier::NotificationService;

#[tokio::main]
async fn main() {
    // 1. Загружаем конфигурацию
    let (config, config_path) = match config::get_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Критическая ошибка конфигурации: {}", e);
            let _ = notify_rust::Notification::new()
                .summary("Ошибка конфигурации")
                .body(&e)
                .show();
            return;
        }
    };

    println!("Конфигурация успешно загружена из: {}", config_path.display());

    // 2. Инициализируем слушатель горячих клавиш
    let mut listener = match HotkeyListener::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Ошибка инициализации hotkeys: {}", e);
            return;
        }
    };

    // Регистрируем клавиши для всех состояний из конфига
    for state in &config.states {
        if let Err(e) = listener.register_state(state) {
            eprintln!("Ошибка регистрации клавиши: {}", e);
            return;
        }
    }

    // 3. Создаем каналы событий (Event-Driven)
    let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
    let (domain_tx, domain_rx) = tokio::sync::mpsc::channel(100);

    // 4. Запускаем фоновую службу уведомлений, реагирующую на доменные события
    tokio::spawn(NotificationService::run(domain_rx));

    // 5. Запускаем асинхронное прослушивание хоткеев и трансляцию в InputEvent
    listener.start(input_tx);

    // 6. Запускаем ядро приложения, передавая ему каналы
    let app = App::new(config, domain_tx);
    app.run(input_rx).await;
}
