use crate::features::daemons::event::DaemonAction;

#[derive(Debug, Clone)]
pub enum CommandAction {
    Run(Vec<String>),
    Vpn(String),
    Daemon(String, DaemonAction),
    Action(String),
}

#[derive(Debug, Clone)]
pub enum ActionInputEvent {
    ExecuteAction(String),
    ExecuteExportedAction(String),
    ExecuteCommand(CommandAction),
}
