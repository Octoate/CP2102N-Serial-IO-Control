use std::{fmt, str::FromStr};

use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

use crate::error::PortSelectorError;

pub const DEFAULT_VENDOR_ID: u16 = 0x10C4;
pub const DEFAULT_PRODUCT_ID: u16 = 0xEA60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortSelector(String);

impl PortSelector {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, reported_port: &str) -> bool {
        if self.0.starts_with("COM") {
            self.0.eq_ignore_ascii_case(reported_port)
        } else {
            self.0 == reported_port
        }
    }
}

impl FromStr for PortSelector {
    type Err = PortSelectorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_whitespace) {
            return Err(PortSelectorError::EmptyOrWhitespace);
        }

        if value
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("COM"))
        {
            let number = &value[3..];
            if is_positive_number(number) {
                return Ok(Self(format!("COM{number}")));
            }

            return Err(PortSelectorError::InvalidWindowsSelector);
        }

        for prefix in ["/dev/ttyUSB", "/dev/ttyACM"] {
            if let Some(number) = value.strip_prefix(prefix) {
                if is_non_negative_number(number) {
                    return Ok(Self(value.to_owned()));
                }

                return Err(PortSelectorError::InvalidLinuxSelector { prefix });
            }
        }

        Err(PortSelectorError::UnsupportedFormat)
    }
}

fn is_positive_number(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| character.is_ascii_digit())
        && !value.starts_with('0')
}

fn is_non_negative_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cp2102nDevice {
    port_name: String,
    vendor_id: u16,
    product_id: u16,
    usb_serial_number: Option<String>,
    manufacturer: Option<String>,
    product: Option<String>,
}

impl Cp2102nDevice {
    pub fn from_port_info(info: &SerialPortInfo) -> Option<Self> {
        let SerialPortType::UsbPort(usb_info) = &info.port_type else {
            return None;
        };

        Self::from_usb_info(&info.port_name, usb_info)
    }

    pub fn from_usb_info(port_name: &str, info: &UsbPortInfo) -> Option<Self> {
        if !Self::matches_identity(info.vid, info.pid) {
            return None;
        }

        Some(Self {
            port_name: port_name.to_owned(),
            vendor_id: info.vid,
            product_id: info.pid,
            usb_serial_number: normalize_optional(info.serial_number.as_deref()),
            manufacturer: normalize_optional(info.manufacturer.as_deref()),
            product: normalize_optional(info.product.as_deref()),
        })
    }

    pub const fn matches_identity(vendor_id: u16, product_id: u16) -> bool {
        vendor_id == DEFAULT_VENDOR_ID && product_id == DEFAULT_PRODUCT_ID
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn usb_serial_number(&self) -> Option<&str> {
        self.usb_serial_number.as_deref()
    }

    pub fn manufacturer(&self) -> Option<&str> {
        self.manufacturer.as_deref()
    }

    pub fn product(&self) -> Option<&str> {
        self.product.as_deref()
    }
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

impl fmt::Display for Cp2102nDevice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "port={} vid=0x{:04X} pid=0x{:04X} serial={} manufacturer={} product={}",
            self.port_name,
            self.vendor_id,
            self.product_id,
            self.usb_serial_number().unwrap_or("<none>"),
            self.manufacturer().unwrap_or("<none>"),
            self.product().unwrap_or("<none>"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Cp2102nDevice, DEFAULT_PRODUCT_ID, DEFAULT_VENDOR_ID, PortSelector};
    use serialport::UsbPortInfo;

    fn usb_info(vid: u16, pid: u16) -> UsbPortInfo {
        UsbPortInfo {
            vid,
            pid,
            serial_number: Some("  CP2102N-123  ".to_owned()),
            manufacturer: Some(" Silicon Labs ".to_owned()),
            product: Some(" CP2102N USB to UART Bridge ".to_owned()),
        }
    }

    #[test]
    fn matches_the_default_cp2102n_identity() {
        assert!(Cp2102nDevice::matches_identity(
            DEFAULT_VENDOR_ID,
            DEFAULT_PRODUCT_ID
        ));
        assert!(!Cp2102nDevice::matches_identity(DEFAULT_VENDOR_ID, 0xEA61));
        assert!(!Cp2102nDevice::matches_identity(0x10C5, DEFAULT_PRODUCT_ID));
    }

    #[test]
    fn rejects_non_matching_usb_devices() {
        assert!(Cp2102nDevice::from_usb_info("COM3", &usb_info(0x1234, 0x5678)).is_none());
    }

    #[test]
    fn preserves_and_normalizes_usb_metadata() {
        let device =
            Cp2102nDevice::from_usb_info("COM3", &usb_info(DEFAULT_VENDOR_ID, DEFAULT_PRODUCT_ID))
                .unwrap();

        assert_eq!(device.port_name(), "COM3");
        assert_eq!(device.usb_serial_number(), Some("CP2102N-123"));
        assert_eq!(device.manufacturer(), Some("Silicon Labs"));
        assert_eq!(device.product(), Some("CP2102N USB to UART Bridge"));
    }

    #[test]
    fn represents_missing_metadata_deterministically() {
        let mut info = usb_info(DEFAULT_VENDOR_ID, DEFAULT_PRODUCT_ID);
        info.serial_number = Some("  ".to_owned());
        info.manufacturer = None;
        info.product = None;

        let device = Cp2102nDevice::from_usb_info("/dev/ttyUSB0", &info).unwrap();

        assert_eq!(device.usb_serial_number(), None);
        assert_eq!(
            device.to_string(),
            "port=/dev/ttyUSB0 vid=0x10C4 pid=0xEA60 serial=<none> manufacturer=<none> product=<none>"
        );
    }

    #[test]
    fn validates_and_normalizes_port_selectors() {
        assert_eq!("cOm12".parse::<PortSelector>().unwrap().as_str(), "COM12");
        assert_eq!(
            "/dev/ttyACM0".parse::<PortSelector>().unwrap().as_str(),
            "/dev/ttyACM0"
        );
        assert!("COM".parse::<PortSelector>().is_err());
        assert!("/dev/ttyUSB".parse::<PortSelector>().is_err());
        assert!("/dev/ttyS0".parse::<PortSelector>().is_err());
    }
}
