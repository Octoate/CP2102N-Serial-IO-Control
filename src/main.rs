mod cli;
mod device;
mod discovery;
mod error;
mod gpio;
mod platform;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};
use discovery::{discover_devices, select_device};
use error::AppError;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();

    match cli.command {
        Command::List(args) => {
            let devices = discover_devices()?;

            if let Some(port) = args.port {
                println!("{}", select_device(&devices, &port)?);
            } else if devices.is_empty() {
                return Err(AppError::NoMatchingDevices);
            } else {
                for device in devices {
                    println!("{device}");
                }
            }

            Ok(())
        }
        Command::Set(args) => {
            let devices = discover_devices()?;
            let device = select_device(&devices, &args.port)?;
            platform::set_pin(device.port_name(), args.pin.into(), args.state)?;
            println!("set {} {} to {}", device.port_name(), args.pin, args.state);
            Ok(())
        }
    }
}
