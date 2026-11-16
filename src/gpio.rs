use crate::error::AppError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpioPin {
    Gpio2,
    Gpio3,
}

impl GpioPin {
    pub const fn number(self) -> u8 {
        match self {
            Self::Gpio2 => 2,
            Self::Gpio3 => 3,
        }
    }

    pub(crate) const fn mask(self) -> u8 {
        1 << self.number()
    }
}

impl TryFrom<u8> for GpioPin {
    type Error = AppError;

    fn try_from(number: u8) -> Result<Self, Self::Error> {
        match number {
            2 => Ok(Self::Gpio2),
            3 => Ok(Self::Gpio3),
            pin => Err(AppError::UnsupportedGpioPin { pin }),
        }
    }
}

impl std::fmt::Display for GpioPin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "GPIO{}", self.number())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputState {
    High,
    Low,
}

impl std::fmt::Display for OutputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::High => "high",
            Self::Low => "low",
        })
    }
}

pub(crate) const fn update_latch(latch: u8, pin: GpioPin, state: OutputState) -> u8 {
    match state {
        OutputState::High => latch | pin.mask(),
        OutputState::Low => latch & !pin.mask(),
    }
}

pub(crate) const fn state_from_latch(latch: u8, pin: GpioPin) -> OutputState {
    if latch & pin.mask() == 0 {
        OutputState::Low
    } else {
        OutputState::High
    }
}

#[cfg(any(target_os = "linux", test))]
pub(crate) const fn state_from_value(value: u8) -> Option<OutputState> {
    match value {
        0 => Some(OutputState::Low),
        1 => Some(OutputState::High),
        _ => None,
    }
}

pub(crate) trait GpioController {
    fn read_latch(&mut self, port: &str) -> Result<u8, AppError>;

    fn write_latch(&mut self, port: &str, mask: u8, latch: u8) -> Result<(), AppError>;
}

pub(crate) fn set_pin<C: GpioController>(
    controller: &mut C,
    port: &str,
    pin: GpioPin,
    state: OutputState,
) -> Result<(), AppError> {
    let current_latch = controller.read_latch(port)?;
    let new_latch = update_latch(current_latch, pin, state);

    // The runtime API accepts a mask, so only the selected GPIO is changed by
    // the device even though the complete read-modified latch is supplied.
    controller.write_latch(port, pin.mask(), new_latch)
}

pub(crate) fn read_pin<C: GpioController>(
    controller: &mut C,
    port: &str,
    pin: GpioPin,
) -> Result<OutputState, AppError> {
    controller
        .read_latch(port)
        .map(|latch| state_from_latch(latch, pin))
}

#[cfg(test)]
mod tests {
    use super::{
        GpioController, GpioPin, OutputState, read_pin, set_pin, state_from_latch,
        state_from_value, update_latch,
    };
    use crate::error::AppError;

    #[test]
    fn maps_only_the_selected_pin_high() {
        assert_eq!(
            update_latch(0b1010_0000, GpioPin::Gpio2, OutputState::High),
            0b1010_0100
        );
    }

    #[test]
    fn maps_only_the_selected_pin_low() {
        assert_eq!(
            update_latch(0b1111_1111, GpioPin::Gpio3, OutputState::Low),
            0b1111_0111
        );
    }

    #[test]
    fn decodes_gpio2_latch_bit_to_both_states() {
        assert_eq!(
            state_from_latch(0b1010_0000, GpioPin::Gpio2),
            OutputState::Low
        );
        assert_eq!(
            state_from_latch(0b1010_0100, GpioPin::Gpio2),
            OutputState::High
        );
    }

    #[test]
    fn decodes_gpio3_latch_bit_to_both_states() {
        assert_eq!(
            state_from_latch(0b1010_0000, GpioPin::Gpio3),
            OutputState::Low
        );
        assert_eq!(
            state_from_latch(0b1010_1000, GpioPin::Gpio3),
            OutputState::High
        );
    }

    #[test]
    fn decodes_only_binary_line_values() {
        assert_eq!(state_from_value(0), Some(OutputState::Low));
        assert_eq!(state_from_value(1), Some(OutputState::High));
        assert_eq!(state_from_value(2), None);
    }

    #[test]
    fn rejects_other_gpio_numbers() {
        assert!(matches!(
            GpioPin::try_from(4),
            Err(AppError::UnsupportedGpioPin { pin: 4 })
        ));
    }

    #[derive(Default)]
    struct FakeController {
        current_latch: u8,
        write: Option<(String, u8, u8)>,
    }

    impl GpioController for FakeController {
        fn read_latch(&mut self, _port: &str) -> Result<u8, AppError> {
            Ok(self.current_latch)
        }

        fn write_latch(&mut self, port: &str, mask: u8, latch: u8) -> Result<(), AppError> {
            self.write = Some((port.to_owned(), mask, latch));
            Ok(())
        }
    }

    #[test]
    fn controller_service_preserves_unselected_latch_bits() {
        let mut controller = FakeController {
            current_latch: 0b1010_0000,
            ..FakeController::default()
        };

        set_pin(&mut controller, "COM7", GpioPin::Gpio2, OutputState::High).unwrap();

        assert_eq!(
            controller.write,
            Some(("COM7".to_owned(), 0b0000_0100, 0b1010_0100))
        );
    }

    #[test]
    fn controller_service_reads_selected_pin_state() {
        let mut controller = FakeController {
            current_latch: 0b1010_1000,
            ..FakeController::default()
        };

        assert_eq!(
            read_pin(&mut controller, "/dev/ttyUSB0", GpioPin::Gpio3).unwrap(),
            OutputState::High
        );
        assert_eq!(
            read_pin(&mut controller, "/dev/ttyUSB0", GpioPin::Gpio2).unwrap(),
            OutputState::Low
        );
        assert_eq!(controller.write, None);
    }
}
