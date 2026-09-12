use std::{ffi::OsStr, iter, os::windows::ffi::OsStrExt};

use libloading::{Library, Symbol};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING},
};

use crate::{
    error::AppError,
    gpio::{self, GpioController, GpioPin, OutputState},
};

const CP210X_SUCCESS: u32 = 0;
const CP210X_INVALID_HANDLE: u32 = 1;
const CP210X_DEVICE_IO_FAILED: u32 = 3;
const CP210X_FUNCTION_NOT_SUPPORTED: u32 = 4;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_SHARING_VIOLATION: u32 = 32;
const ERROR_LOCK_VIOLATION: u32 = 33;

type ReadLatch = unsafe extern "system" fn(HANDLE, *mut u8) -> u32;
type WriteLatch = unsafe extern "system" fn(HANDLE, u8, u8) -> u32;

pub fn set_pin(port: &str, pin: GpioPin, state: OutputState) -> Result<(), AppError> {
    let mut controller = WindowsGpioController::new()?;
    gpio::set_pin(&mut controller, port, pin, state)
}

pub fn get_pin(port: &str, pin: GpioPin) -> Result<OutputState, AppError> {
    let mut controller = WindowsGpioController::new()?;
    gpio::read_pin(&mut controller, port, pin)
}

struct WindowsGpioController {
    runtime: Runtime,
}

impl WindowsGpioController {
    fn new() -> Result<Self, AppError> {
        Ok(Self {
            runtime: Runtime::load()?,
        })
    }
}

impl GpioController for WindowsGpioController {
    fn read_latch(&mut self, port: &str) -> Result<u8, AppError> {
        let handle = ComHandle::open(port)?;
        self.runtime.read_latch(handle.raw())
    }

    fn write_latch(&mut self, port: &str, mask: u8, latch: u8) -> Result<(), AppError> {
        let handle = ComHandle::open(port)?;
        self.runtime.write_latch(handle.raw(), mask, latch)
    }
}

struct Runtime {
    library: Library,
}

impl Runtime {
    fn load() -> Result<Self, AppError> {
        let executable =
            std::env::current_exe().map_err(|error| AppError::RuntimeDllUnavailable {
                details: format!("could not determine the executable path: {error}"),
            })?;
        let executable_directory =
            executable
                .parent()
                .ok_or_else(|| AppError::RuntimeDllUnavailable {
                    details: format!(
                        "could not determine the executable directory from '{}'",
                        executable.display()
                    ),
                })?;
        let library_path = executable_directory.join("CP210xRuntime.dll");
        let library = unsafe { Library::new(&library_path) }.map_err(|error| {
            AppError::RuntimeDllUnavailable {
                details: format!("could not load '{}': {error}", library_path.display()),
            }
        })?;

        Ok(Self { library })
    }

    fn read_latch(&self, handle: HANDLE) -> Result<u8, AppError> {
        let function = self.symbol::<ReadLatch>(b"CP210xRT_ReadLatch\0", "CP210xRT_ReadLatch")?;
        let mut latch = 0;
        let status = unsafe { function(handle, &mut latch) };
        runtime_status("reading the GPIO latch", status).map(|()| latch)
    }

    fn write_latch(&self, handle: HANDLE, mask: u8, latch: u8) -> Result<(), AppError> {
        let function =
            self.symbol::<WriteLatch>(b"CP210xRT_WriteLatch\0", "CP210xRT_WriteLatch")?;
        let status = unsafe { function(handle, mask, latch) };
        runtime_status("writing the GPIO latch", status)
    }

    fn symbol<'library, T>(
        &'library self,
        name: &[u8],
        display_name: &str,
    ) -> Result<Symbol<'library, T>, AppError> {
        unsafe { self.library.get(name) }.map_err(|_| AppError::RuntimeSymbolUnavailable {
            symbol: display_name.to_owned(),
        })
    }
}

struct ComHandle(HANDLE);

impl ComHandle {
    fn open(port: &str) -> Result<Self, AppError> {
        let path = normalize_com_path(port);
        let wide_path = OsStr::new(&path)
            .encode_wide()
            .chain(iter::once(0))
            .collect::<Vec<_>>();

        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let code = unsafe { GetLastError() };
            return Err(open_port_error(port, code));
        }

        Ok(Self(handle))
    }

    const fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ComHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub(crate) fn normalize_com_path(port: &str) -> String {
    let normalized = port
        .get(..3)
        .filter(|prefix| prefix.eq_ignore_ascii_case("COM"))
        .map_or_else(|| port.to_owned(), |_| port.to_ascii_uppercase());

    format!(r"\\.\{normalized}")
}

fn open_port_error(port: &str, code: u32) -> AppError {
    if matches!(
        code,
        ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION
    ) {
        AppError::PortInUse {
            port: port.to_owned(),
        }
    } else {
        AppError::WindowsPortOpen {
            port: port.to_owned(),
            code,
        }
    }
}

fn runtime_status(operation: &str, status: u32) -> Result<(), AppError> {
    match status {
        CP210X_SUCCESS => Ok(()),
        CP210X_FUNCTION_NOT_SUPPORTED => Err(AppError::RuntimeUnsupported {
            operation: operation.to_owned(),
            status,
        }),
        CP210X_INVALID_HANDLE => Err(AppError::RuntimeOperationFailed {
            operation: operation.to_owned(),
            status,
            details: "the COM handle was rejected; verify that the port is still connected"
                .to_owned(),
        }),
        CP210X_DEVICE_IO_FAILED => Err(AppError::RuntimeOperationFailed {
            operation: operation.to_owned(),
            status,
            details: "the device I/O request failed; verify CP2102N runtime compatibility"
                .to_owned(),
        }),
        _ => Err(AppError::RuntimeOperationFailed {
            operation: operation.to_owned(),
            status,
            details: "the runtime returned an unknown status".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CP210X_DEVICE_IO_FAILED, ERROR_SHARING_VIOLATION, normalize_com_path, open_port_error,
        runtime_status,
    };
    use crate::error::AppError;

    #[test]
    fn normalizes_single_digit_com_ports_for_create_file() {
        assert_eq!(normalize_com_path("COM7"), r"\\.\COM7");
    }

    #[test]
    fn normalizes_multi_digit_com_ports_for_create_file() {
        assert_eq!(normalize_com_path("com12"), r"\\.\COM12");
    }

    #[test]
    fn translates_unsupported_runtime_status() {
        let error = runtime_status("reading the GPIO latch", 4).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("does not support reading the GPIO latch")
        );
        assert!(error.to_string().contains("CP2102N-compatible"));
    }

    #[test]
    fn translates_read_latch_io_failures() {
        let error = runtime_status("reading the GPIO latch", CP210X_DEVICE_IO_FAILED).unwrap_err();
        let message = error.to_string();

        assert!(matches!(
            &error,
            AppError::RuntimeOperationFailed {
                operation,
                status: CP210X_DEVICE_IO_FAILED,
                ..
            } if operation == "reading the GPIO latch"
        ));
        assert!(message.contains("device I/O request failed"));
    }

    #[test]
    fn translates_exclusive_port_errors() {
        let error = open_port_error("COM7", ERROR_SHARING_VIOLATION);

        assert!(error.to_string().contains("held exclusively"));
        assert!(
            error
                .to_string()
                .contains("close the external serial application")
        );
    }
}
