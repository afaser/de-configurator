use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};

pub struct HotkeyDispatcher {
    manager: Arc<GlobalHotKeyManager>,
    routes: HashMap<u32, mpsc::Sender<u32>>,
}

impl HotkeyDispatcher {
    pub fn new(manager: Arc<GlobalHotKeyManager>) -> Self {
        Self {
            manager,
            routes: HashMap::new(),
        }
    }

    pub fn register(&mut self, hotkey_id: u32, tx: mpsc::Sender<u32>) {
        self.routes.insert(hotkey_id, tx);
    }

    pub fn start(self) {
        let receiver = GlobalHotKeyEvent::receiver();
        let routes = self.routes;
        let manager = self.manager;

        std::thread::spawn(move || {
            let _keep_alive = manager;
            
            loop {
                if let Ok(event) = receiver.recv() {
                    if let Some(tx) = routes.get(&event.id) {
                        let _ = tx.blocking_send(event.id);
                    }
                }
            }
        });
    }
}
