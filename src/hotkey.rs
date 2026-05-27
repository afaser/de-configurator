use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use tokio::sync::mpsc;
use crate::config::VpnStateConfig;
use crate::event::InputEvent;

pub struct HotkeyListener {
    manager: GlobalHotKeyManager,
    hotkeys: Vec<(u32, String)>, // (hotkey_id, state_id)
}

impl HotkeyListener {
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Не удалось инициализировать HotKeyManager: {:?}", e))?;
        Ok(Self {
            manager,
            hotkeys: Vec::new(),
        })
    }

    /// Регистрирует горячую клавишу для конкретного состояния
    pub fn register_state(&mut self, state: &VpnStateConfig) -> Result<(), String> {
        self.manager
            .register(state.hotkey)
            .map_err(|e| format!("Не удалось зарегистрировать хоткей '{}' для {}: {:?}", state.hotkey_str, state.id, e))?;
        
        self.hotkeys.push((state.hotkey.id(), state.id.clone()));
        Ok(())
    }

    /// Запускает прослушивание событий клавиш в фоновом потоке
    pub fn start(self, tx: mpsc::Sender<InputEvent>) {
        let receiver = GlobalHotKeyEvent::receiver();
        let hotkeys = self.hotkeys;
        let manager = self.manager; // Забираем владение менеджером

        std::thread::spawn(move || {
            // Держим менеджер живым в течение всего времени работы потока
            let _keep_alive = manager;
            
            loop {
                if let Ok(event) = receiver.recv() {
                    // Ищем соответствие ID хоткея с ID нашего состояния в конфиге
                    if let Some((_, state_id)) = hotkeys.iter().find(|(id, _)| *id == event.id) {
                        let input_event = InputEvent::RequestStateSwitch(state_id.clone());
                        if tx.blocking_send(input_event).is_err() {
                            break; // Канал закрыт, завершаем работу потока
                        }
                    }
                }
            }
        });
    }
}
