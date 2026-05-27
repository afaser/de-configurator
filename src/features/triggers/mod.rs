pub mod config;

use std::sync::Arc;
use crate::core::event_bus::{EventBus, SystemEvent, EventContext, Initiator};
pub use config::{TriggersConfig, TriggerType, TriggerConfig};

pub struct TriggersFeature;

impl TriggersFeature {
    pub async fn start(
        doc: &kdl::KdlDocument,
        event_bus: EventBus,
    ) -> Result<(), String> {
        let config = TriggersConfig::parse_from_root_doc(doc)?;
        let triggers = Arc::new(config.triggers);

        println!("Фича Triggers инициализирована. Зарегистрировано триггеров: {}", triggers.len());

        let mut wm_triggers = Vec::new();

        for trigger in triggers.iter() {
            match &trigger.trigger_type {
                TriggerType::Interval { duration } => {
                    let duration = *duration;
                    let cmd = trigger.command.clone();
                    let bus = event_bus.clone();
                    let trigger_id = trigger.id.clone();
                    
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
                TriggerType::Wm { event_type, .. } => {
                    println!("  trigger '{}' (wm: {})", trigger.id, event_type);
                    wm_triggers.push(trigger.clone());
                }
            }
        }

        if !wm_triggers.is_empty() {
            let wm_triggers = Arc::new(wm_triggers);
            let bus = event_bus.clone();
            let mut rx = event_bus.subscribe();

            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        SystemEvent::WmMonitorFocused { monitor_name, context } => {
                            for trigger in wm_triggers.iter() {
                                if let TriggerType::Wm { event_type, monitor, .. } = &trigger.trigger_type {
                                    if event_type == "monitor_focus" && matches_value(monitor, &monitor_name) {
                                        fire_trigger(&bus, trigger, &context);
                                    }
                                }
                            }
                        }
                        SystemEvent::WmDesktopFocused { monitor_name, desktop_name, context } => {
                            for trigger in wm_triggers.iter() {
                                if let TriggerType::Wm { event_type, monitor, desktop, .. } = &trigger.trigger_type {
                                    if event_type == "desktop_focus" 
                                        && matches_value(monitor, &monitor_name)
                                        && matches_value(desktop, &desktop_name) {
                                        fire_trigger(&bus, trigger, &context);
                                    }
                                }
                            }
                        }
                        SystemEvent::WmNodeFocused { monitor_name, desktop_name, class_name, context, .. } => {
                            for trigger in wm_triggers.iter() {
                                if let TriggerType::Wm { event_type, monitor, desktop, class } = &trigger.trigger_type {
                                    if event_type == "node_focus"
                                        && matches_value(monitor, &monitor_name)
                                        && matches_value(desktop, &desktop_name)
                                        && matches_opt_value(class, &class_name) {
                                        fire_trigger(&bus, trigger, &context);
                                    }
                                }
                            }
                        }
                        SystemEvent::WmNodeAdded { monitor_name, desktop_name, class_name, context, .. } => {
                            for trigger in wm_triggers.iter() {
                                if let TriggerType::Wm { event_type, monitor, desktop, class } = &trigger.trigger_type {
                                    if event_type == "node_add"
                                        && matches_value(monitor, &monitor_name)
                                        && matches_value(desktop, &desktop_name)
                                        && matches_opt_value(class, &class_name) {
                                        fire_trigger(&bus, trigger, &context);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });
        }

        Ok(())
    }
}

fn matches_value(pattern: &Option<String>, value: &str) -> bool {
    if let Some(pat) = pattern {
        pat == value
    } else {
        true
    }
}

fn matches_opt_value(pattern: &Option<String>, value: &Option<String>) -> bool {
    if let Some(pat) = pattern {
        value.as_ref() == Some(pat)
    } else {
        true
    }
}

fn fire_trigger(bus: &EventBus, trigger: &TriggerConfig, context: &EventContext) {
    println!("Trigger fired: {}", trigger.id);
    bus.publish(SystemEvent::RequestCommandExecute {
        command: trigger.command.clone(),
        context: EventContext {
            initiator: Initiator::Trigger,
            silent: context.silent,
        },
    });
}
