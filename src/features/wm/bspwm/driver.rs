use crate::features::wm::WindowManager;
use super::xprop::get_window_class;

pub struct BspwmDriver;

impl BspwmDriver {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_monitor_name(&self, monitor_id: &str) -> String {
        run_bspc(&["query", "-M", "-m", monitor_id, "--names"])
            .await
            .unwrap_or_else(|_| monitor_id.to_string())
    }

    pub async fn get_desktop_name(&self, desktop_id: &str) -> String {
        run_bspc(&["query", "-D", "-d", desktop_id, "--names"])
            .await
            .unwrap_or_else(|_| desktop_id.to_string())
    }
}

impl WindowManager for BspwmDriver {
    fn get_monitors<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let res = run_bspc(&["query", "-M", "--names"]).await?;
            Ok(res.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        })
    }

    fn get_focused_monitor<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            run_bspc(&["query", "-M", "-m", "focused", "--names"]).await
        })
    }

    fn get_desktop_windows<'a>(
        &'a self,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let res = run_bspc(&["query", "-N", "-d", desktop_id]).await;
            match res {
                Ok(stdout) => {
                    Ok(stdout.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                }
                Err(_) => Ok(vec![]),
            }
        })
    }

    fn get_app_windows<'a>(
        &'a self,
        app_class: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let res = run_bspc(&["query", "-N", "-n", ".window"]).await;
            match res {
                Ok(stdout) => {
                    let mut matched = Vec::new();
                    for window_id in stdout.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        if let Some(class) = get_window_class(window_id).await {
                            if class.to_lowercase() == app_class.to_lowercase() {
                                matched.push(window_id.to_string());
                            }
                        }
                    }
                    Ok(matched)
                }
                Err(_) => Ok(vec![]),
            }
        })
    }

    fn move_window_to_desktop<'a>(
        &'a self,
        window_id: &'a str,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let _ = run_bspc(&["node", window_id, "-d", desktop_id]).await?;
            Ok(())
        })
    }

    fn focus_desktop<'a>(
        &'a self,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let _ = run_bspc(&["desktop", "-f", desktop_id]).await?;
            Ok(())
        })
    }

    fn set_spawn_rule<'a>(
        &'a self,
        app_class: &'a str,
        desktop_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let _ = run_bspc(&["rule", "-r", app_class]).await;
            let _ = run_bspc(&["rule", "-a", app_class, &format!("desktop={}", desktop_id), "follow=on"]).await?;
            Ok(())
        })
    }

    fn remove_spawn_rule<'a>(
        &'a self,
        app_class: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let _ = run_bspc(&["rule", "-r", app_class]).await;
            Ok(())
        })
    }

    fn ensure_desktop_exists<'a>(
        &'a self,
        monitor_name: &'a str,
        desktop_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let desktops = run_bspc(&["query", "-D", "--names"]).await?;
            let exists = desktops.lines().any(|l| l.trim() == desktop_name);
            if !exists {
                let _ = run_bspc(&["monitor", monitor_name, "-a", desktop_name]).await?;
            }
            Ok(())
        })
    }
}

async fn run_bspc(args: &[&str]) -> Result<String, String> {
    let output = tokio::process::Command::new("bspc")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to execute bspc: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
