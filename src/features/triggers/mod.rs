pub mod config;

use std::sync::Arc;
use crate::core::event_bus::{EventBus, SystemEvent, EventContext, Initiator};
pub use config::{TriggersConfig, TriggerType};

pub struct TriggersFeature;

impl TriggersFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = TriggersConfig::parse_from_root_doc(doc)?;
        let triggers = Arc::new(config.triggers);

        println!("Фича Triggers инициализирована. Зарегистрировано триггеров: {}", triggers.len());

        for trigger in triggers.iter() {
            let bus = event_bus.clone();
            let trigger_id = trigger.id.clone();

            match &trigger.trigger_type {
                TriggerType::Interval { duration } => {
                    let duration = *duration;
                    let cmd = trigger.command.clone();
                    
                    println!("  trigger '{}' (интервал: {:?})", trigger_id, duration);

                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(duration);
                        interval.tick().await;
                        loop {
                            interval.tick().await;
                            bus.publish(SystemEvent::RequestCommandExecute {
                                command: cmd.clone(),
                                context: EventContext {
                                    initiator: Initiator::Trigger,
                                    silent: false,
                                },
                            });
                        }
                    });
                }
            }
        }

        Ok(())
    }
}
