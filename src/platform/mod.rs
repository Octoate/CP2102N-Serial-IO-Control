use crate::{
    error::AppError,
    gpio::{GpioPin, OutputState},
};

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(all(not(windows), not(target_os = "linux")))]
mod unsupported;

pub fn set_pin(port: &str, pin: GpioPin, state: OutputState) -> Result<(), AppError> {
    #[cfg(windows)]
    {
        windows::set_pin(port, pin, state)
    }

    #[cfg(target_os = "linux")]
    {
        linux::set_pin(port, pin, state)
    }

    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        unsupported::set_pin(port, pin, state)
    }
}
