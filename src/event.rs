use crate::vpn::IpInfo;

#[derive(Debug, Clone)]
pub enum InputEvent {
    RequestStateSwitch(String), // Запрос на переключение стейта (state_id)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DomainEvent {
    TransitionStarted {
        state_id: String,
        display_name: String,
        has_interface: bool,
    },
    TransitionCompleted {
        state_id: String,
        display_name: String,
        ip_info: Option<IpInfo>,
    },
    TransitionFailed {
        state_id: String,
        display_name: String,
        error: String,
    },
}
