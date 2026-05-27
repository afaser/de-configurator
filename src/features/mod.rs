pub mod vpn;
pub mod keybinds;
pub mod daemons;
pub mod actions;
pub mod triggers;
pub mod ipc;

use std::sync::Arc;
use kdl::KdlDocument;
use global_hotkey::GlobalHotKeyManager;
use crate::core::hotkey_dispatcher::HotkeyDispatcher;
use crate::core::event_bus::EventBus;
use crate::core::config::is_feature_enabled;

pub async fn start_all(
    doc: &KdlDocument,
    event_bus: EventBus,
    hotkey_manager: Arc<GlobalHotKeyManager>,
    dispatcher: &mut HotkeyDispatcher,
) -> Result<(), String> {
    if is_feature_enabled(doc, "vpn") {
        vpn::VpnFeature::start(doc, event_bus.clone()).await?;
    } else {
        println!("Фича 'vpn' отключена в конфигурации.");
    }

    if is_feature_enabled(doc, "daemons") {
        daemons::DaemonsFeature::start(doc, event_bus.clone()).await?;
    } else {
        println!("Фича 'daemons' отключена в конфигурации.");
    }

    if is_feature_enabled(doc, "actions") {
        actions::ActionsFeature::start(doc, event_bus.clone()).await?;
    } else {
        println!("Фича 'actions' отключена в конфигурации.");
    }

    if is_feature_enabled(doc, "triggers") {
        triggers::TriggersFeature::start(doc, event_bus.clone()).await?;
    } else {
        println!("Фича 'triggers' отключена в конфигурации.");
    }

    if is_feature_enabled(doc, "ipc") {
        ipc::IpcFeature::start(doc, event_bus.clone()).await?;
    } else {
        println!("Фича 'ipc' отключена в конфигурации.");
    }

    if is_feature_enabled(doc, "keybinds") {
        keybinds::KeybindsFeature::start(doc, hotkey_manager, dispatcher, event_bus).await?;
    } else {
        println!("Фича 'keybinds' отключена в конфигурации.");
    }

    Ok(())
}
