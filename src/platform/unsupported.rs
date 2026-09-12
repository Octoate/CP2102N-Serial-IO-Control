use crate::{
    error::AppError,
    gpio::{GpioPin, OutputState},
};

pub fn set_pin(_port: &str, _pin: GpioPin, _state: OutputState) -> Result<(), AppError> {
    Err(AppError::UnsupportedPlatform)
}
