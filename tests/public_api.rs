use cp2102n_serial_io_control::{Cp2102nDevice, Error, GpioPin, OutputState, PortSelector, Result};

#[test]
fn exposes_cli_independent_domain_types() {
    let selector: PortSelector = "com12".parse().unwrap();

    assert_eq!(selector.as_str(), "COM12");
    assert_eq!(selector.as_ref(), "COM12");
    assert_eq!(selector.to_string(), "COM12");
    assert_eq!(GpioPin::try_from(2).unwrap().number(), 2);
    assert_eq!(OutputState::High.to_string(), "high");
}

#[test]
fn exposes_device_operations_without_executing_them() {
    let _get: fn(&Cp2102nDevice, GpioPin) -> Result<OutputState> = Cp2102nDevice::get_pin;
    let _set: fn(&Cp2102nDevice, GpioPin, OutputState) -> Result<()> = Cp2102nDevice::set_pin;

    fn assert_std_error<T: std::error::Error>() {}
    assert_std_error::<Error>();
}
