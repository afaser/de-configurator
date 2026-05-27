pub mod bspwm;

use std::sync::{Arc, OnceLock};
use crate::core::event_bus::EventBus;

pub trait WindowManager: Send + Sync {
    fn get_monitors<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>;

    fn get_focused_monitor<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>;

    fn get_desktop_windows<'a>(
        &'a self,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>;

    fn get_app_windows<'a>(
        &'a self,
        app_class: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>>;

    fn move_window_to_desktop<'a>(
        &'a self,
        window_id: &'a str,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;

    fn focus_desktop<'a>(
        &'a self,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;

    fn set_spawn_rule<'a>(
        &'a self,
        app_class: &'a str,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;

    fn remove_spawn_rule<'a>(
        &'a self,
        app_class: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;

    fn ensure_desktop_exists<'a>(
        &'a self,
        monitor_name: &'a str,
        desktop_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;
}

static ACTIVE_WM: OnceLock<Arc<dyn WindowManager>> = OnceLock::new();

pub fn register_wm(wm: Arc<dyn WindowManager>) {
    let _ = ACTIVE_WM.set(wm);
}

pub fn get_wm() -> Option<Arc<dyn WindowManager>> {
    ACTIVE_WM.get().cloned()
}

pub struct WmConfig {
    pub driver: String,
}

impl WmConfig {
    pub fn parse_from_root_doc(doc: &kdl::KdlDocument) -> Result<Self, String> {
        let node = doc.nodes().iter().find(|n| {
            n.name().value() == "feature"
            && n.entries().get(0).and_then(|e| e.value().as_string()) == Some("wm")
        })
        .ok_or_else(|| "Секция feature \"wm\" не найдена".to_string())?;

        let mut driver = None;
        if let Some(children) = node.children() {
            for child in children.nodes() {
                if child.name().value() == "driver" {
                    driver = child.entries().get(0)
                        .and_then(|e| e.value().as_string())
                        .map(|s| s.to_string());
                }
            }
        }

        let drv = driver.unwrap_or_else(|| {
            if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
                "hyprland".to_string()
            } else {
                "bspwm".to_string()
            }
        });

        Ok(WmConfig { driver: drv })
    }
}

pub struct WmFeature;

impl WmFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = WmConfig::parse_from_root_doc(doc)?;
        println!("Фича WM инициализирована. Выбран драйвер: {}", config.driver);

        match config.driver.as_str() {
            "bspwm" => {
                bspwm::start_bspwm(event_bus).await?;
            }
            "hyprland" => {
                return Err("Драйвер hyprland ещё не реализован".to_string());
            }
            other => {
                return Err(format!("Неподдерживаемый драйвер оконного менеджера: {}", other));
            }
        }

        Ok(())
    }
}
