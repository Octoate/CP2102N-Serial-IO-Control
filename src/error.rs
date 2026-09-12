use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
pub enum PortSelectorError {
    #[error("port selector must be non-empty and contain no whitespace")]
    EmptyOrWhitespace,

    #[error("Windows port selectors must use COM followed by a positive number")]
    InvalidWindowsSelector,

    #[error("Linux port selectors must use {prefix}<number>")]
    InvalidLinuxSelector { prefix: &'static str },

    #[error("expected COM<number>, /dev/ttyUSB<number>, or /dev/ttyACM<number>")]
    UnsupportedFormat,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid port selector: {0}")]
    InvalidPortSelector(#[from] PortSelectorError),

    #[error("failed to enumerate serial ports: {0}")]
    SerialPortEnumeration(#[from] serialport::Error),

    #[error("no matching CP2102N serial devices were found (VID 0x10C4, PID 0xEA60)")]
    NoMatchingDevices,

    #[error("no matching CP2102N serial device was found for port '{port}'")]
    PortNotFound { port: String },

    #[error(
        "multiple matching CP2102N serial devices were found for port '{port}' ({matches} matches)"
    )]
    AmbiguousPort { port: String, matches: usize },

    #[error("unsupported GPIO{pin}; only GPIO2 and GPIO3 are supported")]
    UnsupportedGpioPin { pin: u8 },

    #[cfg(all(not(windows), not(target_os = "linux")))]
    #[error(
        "CP210x GPIO control is not available on this platform yet; use Windows CP210xRuntime.dll"
    )]
    UnsupportedPlatform,

    #[cfg(target_os = "linux")]
    #[error("could not inspect Linux sysfs topology for serial port '{port}' at {path}: {details}")]
    LinuxSysfsUnavailable {
        port: String,
        path: String,
        details: String,
    },

    #[cfg(target_os = "linux")]
    #[error("no usable gpiochip is associated with serial port '{port}': {details}")]
    LinuxGpioChipUnavailable { port: String, details: String },

    #[cfg(target_os = "linux")]
    #[error("multiple gpiochips are associated with serial port '{port}': {chips}")]
    LinuxGpioChipAmbiguous { port: String, chips: String },

    #[cfg(target_os = "linux")]
    #[error(
        "permission denied while accessing Linux GPIO for serial port '{port}' at {path}: {details}"
    )]
    LinuxGpioPermission {
        port: String,
        path: String,
        details: String,
    },

    #[cfg(target_os = "linux")]
    #[error("GPIO{pin} for serial port '{port}' is already in use: {details}")]
    LinuxGpioLineInUse {
        port: String,
        pin: u8,
        details: String,
    },

    #[cfg(target_os = "linux")]
    #[error("GPIO{pin} is unavailable for serial port '{port}': {details}")]
    LinuxGpioLineUnavailable {
        port: String,
        pin: u8,
        details: String,
    },

    #[cfg(target_os = "linux")]
    #[error("Linux GPIO operation '{operation}' failed for serial port '{port}': {details}")]
    LinuxGpioOperation {
        port: String,
        operation: String,
        details: String,
    },

    #[cfg(windows)]
    #[error(
        "CP210xRuntime.dll could not be loaded: {details}; ship a CP2102N-compatible runtime DLL side-by-side with the executable"
    )]
    RuntimeDllUnavailable { details: String },

    #[cfg(windows)]
    #[error("CP210xRuntime.dll does not export {symbol}; install a compatible CP210x runtime DLL")]
    RuntimeSymbolUnavailable { symbol: String },

    #[cfg(windows)]
    #[error(
        "CP210xRuntime.dll does not support {operation} (status 0x{status:02X}); install a CP2102N-compatible runtime DLL"
    )]
    RuntimeUnsupported { operation: String, status: u32 },

    #[cfg(windows)]
    #[error("CP210xRuntime.dll failed while {operation} (status 0x{status:02X}): {details}")]
    RuntimeOperationFailed {
        operation: String,
        status: u32,
        details: String,
    },

    #[cfg(windows)]
    #[error(
        "serial port '{port}' is held exclusively by another process; close the external serial application and retry"
    )]
    PortInUse { port: String },

    #[cfg(windows)]
    #[error("could not open serial port '{port}' through Windows (Win32 error {code})")]
    WindowsPortOpen { port: String, code: u32 },
}
