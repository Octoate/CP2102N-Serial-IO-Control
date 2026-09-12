use serialport::available_ports;

use crate::{
    device::{Cp2102nDevice, PortSelector},
    error::AppError,
};

pub fn discover_devices() -> Result<Vec<Cp2102nDevice>, AppError> {
    let mut devices = available_ports()?
        .iter()
        .filter_map(Cp2102nDevice::from_port_info)
        .collect::<Vec<_>>();

    devices.sort_by(|left, right| left.port_name().cmp(right.port_name()));
    Ok(devices)
}

pub fn select_device<'a>(
    devices: &'a [Cp2102nDevice],
    selector: &PortSelector,
) -> Result<&'a Cp2102nDevice, AppError> {
    let matches = devices
        .iter()
        .filter(|device| selector.matches(device.port_name()))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(AppError::PortNotFound {
            port: selector.as_str().to_owned(),
        }),
        [device] => Ok(device),
        _ => Err(AppError::AmbiguousPort {
            port: selector.as_str().to_owned(),
            matches: matches.len(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cp2102nDevice, select_device};
    use crate::device::{DEFAULT_PRODUCT_ID, DEFAULT_VENDOR_ID, PortSelector};
    use serialport::UsbPortInfo;

    fn device(port_name: &str) -> Cp2102nDevice {
        Cp2102nDevice::from_usb_info(
            port_name,
            &UsbPortInfo {
                vid: DEFAULT_VENDOR_ID,
                pid: DEFAULT_PRODUCT_ID,
                serial_number: None,
                manufacturer: None,
                product: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn selects_a_matching_port_exactly() {
        let devices = vec![device("COM3"), device("COM12")];
        let selected = select_device(&devices, &"COM12".parse().unwrap()).unwrap();

        assert_eq!(selected.port_name(), "COM12");
    }

    #[test]
    fn reports_a_missing_port() {
        let error = select_device(&[device("COM3")], &"COM4".parse().unwrap()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "no matching CP2102N serial device was found for port 'COM4'"
        );
    }

    #[test]
    fn reports_ambiguous_port_matches() {
        let devices = vec![device("COM3"), device("COM3")];
        let error = select_device(&devices, &"com3".parse::<PortSelector>().unwrap()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "multiple matching CP2102N serial devices were found for port 'COM3' (2 matches)"
        );
    }
}
