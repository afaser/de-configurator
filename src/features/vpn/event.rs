use super::manager::IpInfo;

#[derive(Debug, Clone)]
pub enum VpnInputEvent {
    RequestStateSwitch(String), // Запрос на переключение стейта по его ID
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum VpnDomainEvent {
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
