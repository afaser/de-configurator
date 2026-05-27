pub mod config;

use std::sync::Arc;
use tokio::sync::mpsc;
use crate::features::actions::event::ActionInputEvent;
pub use config::{TriggersConfig, TriggerType};

pub struct TriggersFeature;

impl TriggersFeature {
    /// Запуск фичи Triggers
    pub async fn start(
        doc: &kdl::KdlDocument,
        actions_tx: mpsc::Sender<ActionInputEvent>,
    ) -> Result<(), String> {
        let config = TriggersConfig::parse_from_root_doc(doc)?;
        let triggers = Arc::new(config.triggers);

        println!("Фича Triggers инициализирована. Зарегистрировано триггеров: {}", triggers.len());

        for trigger in triggers.iter() {
            let actions_tx_clone = actions_tx.clone();
            let trigger_id = trigger.id.clone();

            match &trigger.trigger_type {
                TriggerType::Interval { duration } => {
                    let duration = *duration;
                    let cmd = trigger.command.clone();
                    
                    println!("  trigger '{}' (интервал: {:?})", trigger_id, duration);

                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(duration);
                        // Пропускаем первый немедленный тик
                        interval.tick().await;
                        loop {
                            interval.tick().await;
                            let _ = actions_tx_clone.send(ActionInputEvent::ExecuteCommand(cmd.clone())).await;
                        }
                    });
                }
            }
        }

        Ok(())
    }
}
