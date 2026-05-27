pub mod config;

pub use config::WorkspacesConfig;
use std::sync::Arc;
use crate::core::event_bus::{EventBus, SystemEvent};

pub struct WorkspacesFeature;

impl WorkspacesFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = WorkspacesConfig::parse_from_root_doc(doc)?;
        let config = Arc::new(config);
        
        let mut rx = event_bus.subscribe();
        let cfg = config.clone();
        
        tokio::spawn(async move {
            let wm = loop {
                if let Some(w) = crate::features::wm::get_wm() {
                    break w;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            };

            if let Ok(monitors) = wm.get_monitors().await {
                for monitor in &monitors {
                    for ws in &cfg.workspaces {
                        let desktop_name = format!("{}_{}", monitor, ws.id);
                        let _ = wm.ensure_desktop_exists(monitor, &desktop_name).await;
                    }
                }
            }

            while let Ok(event) = rx.recv().await {
                match event {
                    SystemEvent::RequestWorkspaceFocus { workspace_id, monitor_name, .. } => {
                        let ws = cfg.workspaces.iter().find(|w| w.id == workspace_id);
                        if let Some(w_config) = ws {
                            let active_mon = match monitor_name {
                                Some(m) => m,
                                None => match wm.get_focused_monitor().await {
                                    Ok(m) => m,
                                    Err(e) => {
                                        eprintln!("Failed to get focused monitor: {}", e);
                                        continue;
                                    }
                                }
                            };

                            let desktop_name = format!("{}_{}", active_mon, workspace_id);
                            
                            let windows = match wm.get_app_windows(&w_config.class).await {
                                Ok(w) => w,
                                Err(e) => {
                                    eprintln!("Failed to query windows for class {}: {}", w_config.class, e);
                                    vec![]
                                }
                            };

                            println!("workspaces: Focusing workspace '{}' (class '{}') on monitor '{}' -> desktop '{}' (found {} windows)", workspace_id, w_config.class, active_mon, desktop_name, windows.len());

                            for win in &windows {
                                let _ = wm.move_window_to_desktop(win, &desktop_name).await;
                            }

                            let _ = wm.set_spawn_rule(&w_config.class, &desktop_name).await;

                            if let Err(e) = wm.focus_desktop(&desktop_name).await {
                                eprintln!("Failed to focus desktop {}: {}", desktop_name, e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }
}
