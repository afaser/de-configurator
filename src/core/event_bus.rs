use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonAction {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    Run(Vec<String>),
    Vpn(String),
    Daemon(String, DaemonAction),
    Action(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Initiator {
    Direct,
    Keybind,
    Ipc,
    Trigger,
    Action,
}

#[derive(Debug, Clone)]
pub struct EventContext {
    pub initiator: Initiator,
    pub silent: bool,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    RequestVpnSwitch {
        state_id: String,
        context: EventContext,
    },
    RequestDaemonControl {
        daemon_id: String,
        action: DaemonAction,
        context: EventContext,
    },
    RequestActionExecute {
        action_id: String,
        context: EventContext,
    },
    RequestActionExecuteExported {
        action_id: String,
        context: EventContext,
    },
    RequestCommandExecute {
        command: CommandAction,
        context: EventContext,
    },

    VpnTransitionStarted {
        state_id: String,
        display_name: String,
        has_interface: bool,
        context: EventContext,
    },
    VpnStateChanged {
        state_id: String,
        display_name: String,
        ip_info: Option<String>,
        context: EventContext,
    },
    VpnStateTransitionFailed {
        state_id: String,
        display_name: String,
        error: String,
        context: EventContext,
    },
    DaemonStateChanged {
        daemon_id: String,
        display_name: String,
        is_running: bool,
        context: EventContext,
    },
    DaemonStateTransitionFailed {
        daemon_id: String,
        display_name: String,
        action: DaemonAction,
        error: String,
        context: EventContext,
    },
}

impl SystemEvent {
    #[allow(dead_code)]
    pub fn context(&self) -> &EventContext {
        match self {
            SystemEvent::RequestVpnSwitch { context, .. } => context,
            SystemEvent::RequestDaemonControl { context, .. } => context,
            SystemEvent::RequestActionExecute { context, .. } => context,
            SystemEvent::RequestActionExecuteExported { context, .. } => context,
            SystemEvent::RequestCommandExecute { context, .. } => context,
            SystemEvent::VpnTransitionStarted { context, .. } => context,
            SystemEvent::VpnStateChanged { context, .. } => context,
            SystemEvent::VpnStateTransitionFailed { context, .. } => context,
            SystemEvent::DaemonStateChanged { context, .. } => context,
            SystemEvent::DaemonStateTransitionFailed { context, .. } => context,
        }
    }
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(128);
        Self { sender }
    }

    pub fn publish(&self, event: SystemEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}
