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

pub fn get_pin(port: &str, pin: GpioPin) -> Result<OutputState, AppError> {
    #[cfg(windows)]
    {
        windows::get_pin(port, pin)
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_pin(port, pin)
    }

    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        unsupported::get_pin(port, pin)
    }
}
