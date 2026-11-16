//! Discover CP2102N devices and control their GPIO2/GPIO3 runtime levels.
//!
//! The library supports the default Silicon Labs USB identity (`10C4:EA60`)
//! and preserves the normal VCP serial driver and port mapping. Device
//! discovery is separate from GPIO access so callers can select an explicit
//! port and handle an empty discovery result according to their own needs.
//!
//! ```no_run
//! use cp2102n_serial_io_control::{
//!     GpioPin, OutputState, discover_devices, select_device,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let devices = discover_devices()?;
//!     let selector = "COM7".parse()?;
//!     let device = select_device(&devices, &selector)?;
//!
//!     device.set_pin(GpioPin::Gpio2, OutputState::High)?;
//!     assert_eq!(device.get_pin(GpioPin::Gpio2)?, OutputState::High);
//!     Ok(())
//! }
//! ```

mod device;
mod discovery;
mod error;
mod gpio;
mod platform;

pub use device::{Cp2102nDevice, DEFAULT_PRODUCT_ID, DEFAULT_VENDOR_ID, PortSelector};
pub use discovery::{discover_devices, select_device};
pub use error::{AppError as Error, PortSelectorError};
pub use gpio::{GpioPin, OutputState};

/// Result type returned by CP2102N discovery, selection, and GPIO operations.
pub type Result<T> = std::result::Result<T, Error>;

impl Cp2102nDevice {
    /// Read the current runtime level of GPIO2 or GPIO3.
    ///
    /// This does not change persistent CP2102N configuration. The device
    /// metadata is a discovery snapshot; unplugging or replugging the board
    /// may require discovering and selecting it again.
    pub fn get_pin(&self, pin: GpioPin) -> Result<OutputState> {
        platform::get_pin(self.port_name(), pin)
    }

    /// Set the transient output state of GPIO2 or GPIO3.
    ///
    /// This performs a real hardware write. Verify the selected device, pin,
    /// wiring, and target voltage before calling it. Persistent CP2102N
    /// configuration is not modified.
    pub fn set_pin(&self, pin: GpioPin, state: OutputState) -> Result<()> {
        platform::set_pin(self.port_name(), pin, state)
    }
}
