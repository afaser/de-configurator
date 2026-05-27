#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod core {
    pub mod config;
    pub mod event_bus;
    pub mod hotkey_dispatcher;
    pub mod state_store;
}
mod features;

use std::sync::Arc;
use global_hotkey::GlobalHotKeyManager;
use core::hotkey_dispatcher::HotkeyDispatcher;
use core::event_bus::EventBus;
use core::state_store::StateStore;

#[tokio::main]
async fn main() {
    let (config_doc, config_path) = match core::config::get_config_doc() {
        Ok(res) => res,
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

    let hotkey_manager = Arc::new(GlobalHotKeyManager::new().unwrap());
    let mut dispatcher = HotkeyDispatcher::new(hotkey_manager.clone());

    let event_bus = EventBus::new();
    let state_store = StateStore::new();
    state_store.start_sync(event_bus.subscribe());

    if let Err(e) = features::start_all(&config_doc, event_bus, hotkey_manager, &mut dispatcher).await {
        eprintln!("Ошибка инициализации компонентов: {}", e);
        let _ = notify_rust::Notification::new()
            .summary("Ошибка инициализации")
            .body(&e)
            .show();
        return;
    }

    dispatcher.start();

    println!("Приложение запущено. Нажмите Ctrl+C для выхода.");
    let _ = tokio::signal::ctrl_c().await;
    println!("Завершение работы...");
}
