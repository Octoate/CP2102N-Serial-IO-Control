use std::fmt;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::{device::PortSelector, gpio::GpioPin};

pub use crate::gpio::OutputState;

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "cp2102n-serial-io-control",
    version,
    about = "List CP2102N serial devices and control their exposed GPIO pins"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    /// List matching CP2102N serial devices.
    List(ListArgs),
    /// Set a CP2102N GPIO output state.
    Set(SetArgs),
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct ListArgs {
    /// Select one reported serial port instead of listing all matching devices.
    #[arg(long, value_name = "PORT")]
    pub port: Option<PortSelector>,
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct SetArgs {
    /// Reported serial port name, such as COM3 or /dev/ttyUSB0.
    #[arg(long, value_name = "PORT")]
    pub port: PortSelector,
    /// CP2102N GPIO number; only GPIO2 and GPIO3 are supported.
    #[arg(long, value_enum, value_name = "2|3")]
    pub pin: Pin,
    /// Requested output latch state.
    #[arg(long, value_enum, ignore_case = true, value_name = "high|low")]
    pub state: OutputState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Pin {
    #[value(name = "2")]
    Gpio2,
    #[value(name = "3")]
    Gpio3,
}

impl From<Pin> for GpioPin {
    fn from(pin: Pin) -> Self {
        match pin {
            Pin::Gpio2 => Self::Gpio2,
            Pin::Gpio3 => Self::Gpio3,
        }
    }
}

impl Pin {
    pub const fn number(self) -> u8 {
        match self {
            Self::Gpio2 => 2,
            Self::Gpio3 => 3,
        }
    }
}

impl fmt::Display for Pin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "GPIO{}", self.number())
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, ListArgs, OutputState, Pin, SetArgs};
    use clap::Parser;

    #[test]
    fn parses_list_without_a_port_filter() {
        let cli = Cli::try_parse_from(["cp2102n-serial-io-control", "list"]).unwrap();

        assert_eq!(cli.command, Command::List(ListArgs { port: None }));
    }

    #[test]
    fn parses_set_arguments_and_case_insensitive_state() {
        let cli = Cli::try_parse_from([
            "cp2102n-serial-io-control",
            "set",
            "--port",
            "com12",
            "--pin",
            "3",
            "--state",
            "HIGH",
        ])
        .unwrap();

        assert_eq!(
            cli.command,
            Command::Set(SetArgs {
                port: "COM12".parse().unwrap(),
                pin: Pin::Gpio3,
                state: OutputState::High,
            })
        );
    }

    #[test]
    fn rejects_unsupported_pin() {
        let result = Cli::try_parse_from([
            "cp2102n-serial-io-control",
            "set",
            "--port",
            "COM3",
            "--pin",
            "4",
            "--state",
            "low",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_port_selector() {
        let result = Cli::try_parse_from([
            "cp2102n-serial-io-control",
            "list",
            "--port",
            "/dev/not-a-serial-port",
        ]);

        assert!(result.is_err());
    }
}
