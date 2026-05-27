use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use crate::core::event_bus::{EventBus, SystemEvent, EventContext, Initiator};
use super::driver::BspwmDriver;
use super::xprop::get_window_class;

pub async fn start_listener(driver: Arc<BspwmDriver>, bus: EventBus) {
    let mut child = match tokio::process::Command::new("bspc")
        .args(&["subscribe", "monitor", "desktop", "node"])
        .stdout(std::process::Stdio::piped())
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to spawn bspc subscribe: {}", e);
                return;
            }
        };

    let stdout = child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let context = EventContext {
            initiator: Initiator::Direct,
            silent: true,
        };

        match parts[0] {
            "monitor_focus" if parts.len() >= 2 => {
                let monitor_name = driver.get_monitor_name(parts[1]).await;
                println!("bspwm: WmMonitorFocused: monitor_name={}", monitor_name);
                bus.publish(SystemEvent::WmMonitorFocused {
                    monitor_name,
                    context,
                });
            }
            "desktop_focus" if parts.len() >= 3 => {
                let monitor_name = driver.get_monitor_name(parts[1]).await;
                let desktop_name = driver.get_desktop_name(parts[2]).await;
                println!("bspwm: WmDesktopFocused: monitor_name={}, desktop_name={}", monitor_name, desktop_name);
                bus.publish(SystemEvent::WmDesktopFocused {
                    monitor_name,
                    desktop_name,
                    context,
                });
            }
            "node_focus" if parts.len() >= 4 => {
                let monitor_name = driver.get_monitor_name(parts[1]).await;
                let desktop_name = driver.get_desktop_name(parts[2]).await;
                let node_id = parts[3].to_string();
                let class_name = get_window_class(&node_id).await;
                println!("bspwm: WmNodeFocused: monitor_name={}, desktop_name={}, node_id={}, class={:?}", monitor_name, desktop_name, node_id, class_name);
                bus.publish(SystemEvent::WmNodeFocused {
                    monitor_name,
                    desktop_name,
                    node_id,
                    class_name,
                    context,
                });
            }
            "node_add" if parts.len() >= 5 => {
                let monitor_name = driver.get_monitor_name(parts[1]).await;
                let desktop_name = driver.get_desktop_name(parts[2]).await;
                let node_id = parts[4].to_string();
                let class_name = get_window_class(&node_id).await;
                println!("bspwm: WmNodeAdded: monitor_name={}, desktop_name={}, node_id={}, class={:?}", monitor_name, desktop_name, node_id, class_name);
                bus.publish(SystemEvent::WmNodeAdded {
                    monitor_name,
                    desktop_name,
                    node_id,
                    class_name,
                    context,
                });
            }
            _ => {}
        }
    }
}
