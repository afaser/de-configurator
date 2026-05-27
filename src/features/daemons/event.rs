#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonAction {
    On,
    Off,
    Toggle,
}

impl std::fmt::Display for DaemonAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonAction::On => write!(f, "включение"),
            DaemonAction::Off => write!(f, "выключение"),
            DaemonAction::Toggle => write!(f, "переключение"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DaemonInputEvent {
    Control {
        daemon_id: String,
        action: DaemonAction,
        silent: bool,
    },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DaemonDomainEvent {
    StateChanged {
        daemon_id: String,
        display_name: String,
        is_running: bool,
    },
    TransitionFailed {
        daemon_id: String,
        display_name: String,
        action: DaemonAction,
        error: String,
    },
}
