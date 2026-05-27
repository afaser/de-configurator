pub async fn get_window_class(window_id: &str) -> Option<String> {
    let output = tokio::process::Command::new("xprop")
        .args(&["-id", window_id, "WM_CLASS"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = s.find('=') {
            let val = &s[pos + 1..];
            let parts: Vec<&str> = val.split(',').collect();
            let class_part = parts.last()?;
            let clean = class_part.trim().replace('"', "");
            if !clean.is_empty() {
                return Some(clean);
            }
        }
    }
    None
}
