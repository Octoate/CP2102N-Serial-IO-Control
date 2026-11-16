mod cli;

use std::process::ExitCode;

use clap::Parser;
use cp2102n_serial_io_control::{Error, discover_devices, select_device};

use cli::{Cli, Command};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::List(args) => {
            let devices = discover_devices()?;

            if let Some(port) = args.port {
                println!("{}", select_device(&devices, &port)?);
            } else if devices.is_empty() {
                return Err(Error::NoMatchingDevices);
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
            device.set_pin(args.pin.into(), args.state.into())?;
            println!("set {} {} to {}", device.port_name(), args.pin, args.state);
            Ok(())
        }
        Command::Get(args) => {
            let devices = discover_devices()?;
            let device = select_device(&devices, &args.port)?;
            let state = device.get_pin(args.pin.into())?;
            println!("get {} {} = {state}", device.port_name(), args.pin);
            Ok(())
        }
    }
}
