pub mod xprop;
pub mod driver;
pub mod listener;

pub use driver::BspwmDriver;
use std::sync::Arc;
use crate::core::event_bus::EventBus;

pub async fn start_bspwm(event_bus: EventBus) -> Result<(), String> {
    let driver = Arc::new(BspwmDriver::new());
    crate::features::wm::register_wm(driver.clone());

    tokio::spawn(listener::start_listener(driver, event_bus));

    Ok(())
}
