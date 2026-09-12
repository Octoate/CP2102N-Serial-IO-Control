use std::{
    cmp::Reverse,
    fs,
    path::{Path, PathBuf},
};

use gpio_cdev::{Chip, LineRequestFlags};

use crate::{
    error::AppError,
    gpio::{GpioPin, OutputState},
};

const SYSFS_ROOT: &str = "/sys";
const DEVICE_ROOT: &str = "/dev";
const CONSUMER: &str = "cp2102n-serial-io-control";

pub fn set_pin(port: &str, pin: GpioPin, state: OutputState) -> Result<(), AppError> {
    let mut controller = LinuxGpioController::new();
    controller.set_output(port, pin, state)
}

struct LinuxGpioController {
    sysfs_root: PathBuf,
    device_root: PathBuf,
    chip_path: Option<PathBuf>,
}

impl LinuxGpioController {
    fn new() -> Self {
        Self {
            sysfs_root: PathBuf::from(SYSFS_ROOT),
            device_root: PathBuf::from(DEVICE_ROOT),
            chip_path: None,
        }
    }

    fn chip_path(&mut self, port: &str) -> Result<PathBuf, AppError> {
        if let Some(path) = &self.chip_path {
            return Ok(path.clone());
        }

        let path = find_gpiochip_for_port(port, &self.sysfs_root, &self.device_root)?;
        self.chip_path = Some(path.clone());
        Ok(path)
    }

    fn open_chip(&mut self, port: &str) -> Result<(PathBuf, Chip), AppError> {
        let path = self.chip_path(port)?;
        let chip = Chip::new(&path).map_err(|error| map_chip_error(port, &path, error))?;
        Ok((path, chip))
    }
}

impl LinuxGpioController {
    fn set_output(&mut self, port: &str, pin: GpioPin, state: OutputState) -> Result<(), AppError> {
        let (chip_path, mut chip) = self.open_chip(port)?;
        let (offset, value) = output_request(pin, state);
        let line = chip
            .get_line(offset)
            .map_err(|error| map_line_error(port, &chip_path, pin, error.to_string()))?;
        let handle = line
            .request(LineRequestFlags::OUTPUT, value, CONSUMER)
            .map_err(|error| map_line_error(port, &chip_path, pin, error.to_string()))?;

        handle
            .set_value(value)
            .map_err(|error| map_gpio_operation(port, &chip_path, "setting GPIO output", error))
    }
}

fn output_request(pin: GpioPin, state: OutputState) -> (u32, u8) {
    (
        u32::from(pin.number()),
        u8::from(matches!(state, OutputState::High)),
    )
}

#[derive(Debug)]
struct ChipCandidate {
    name: String,
    path: PathBuf,
    topology: PathBuf,
    specificity: usize,
}

fn find_gpiochip_for_port(
    port: &str,
    sysfs_root: &Path,
    device_root: &Path,
) -> Result<PathBuf, AppError> {
    let tty = tty_name(port).ok_or_else(|| AppError::LinuxSysfsUnavailable {
        port: port.to_owned(),
        path: sysfs_root.join("class/tty").display().to_string(),
        details: "the selected port is not a /dev/ttyUSB* or /dev/ttyACM* device".to_owned(),
    })?;
    let tty_device = sysfs_root.join("class/tty").join(tty).join("device");
    let port_topology =
        fs::canonicalize(&tty_device).map_err(|error| AppError::LinuxSysfsUnavailable {
            port: port.to_owned(),
            path: tty_device.display().to_string(),
            details: error.to_string(),
        })?;

    let gpio_class = sysfs_root.join("class/gpio");
    let entries =
        fs::read_dir(&gpio_class).map_err(|error| AppError::LinuxGpioChipUnavailable {
            port: port.to_owned(),
            details: format!("cannot enumerate {}: {error}", gpio_class.display()),
        })?;
    let mut candidates = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| AppError::LinuxGpioChipUnavailable {
            port: port.to_owned(),
            details: format!("cannot enumerate {}: {error}", gpio_class.display()),
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_gpiochip_name(&name) {
            continue;
        }

        let topology_link = entry.path().join("device");
        let Ok(topology) = fs::canonicalize(&topology_link) else {
            continue;
        };
        if !topology_matches(&port_topology, &topology) {
            continue;
        }

        let path = device_root.join(&name);
        candidates.push(ChipCandidate {
            name,
            path,
            specificity: topology.components().count(),
            topology,
        });
    }

    if candidates.is_empty() {
        return Err(AppError::LinuxGpioChipUnavailable {
            port: port.to_owned(),
            details: format!(
                "no gpiochip under {} has the selected tty's sysfs device as its parent; \
                 check kernel GPIOLIB/CP210x GPIO support",
                gpio_class.display()
            ),
        });
    }

    candidates.sort_by_key(|candidate| Reverse(candidate.specificity));
    let best_specificity = candidates[0].specificity;
    let best = candidates
        .iter()
        .filter(|candidate| candidate.specificity == best_specificity)
        .collect::<Vec<_>>();
    if best.len() > 1 {
        return Err(AppError::LinuxGpioChipAmbiguous {
            port: port.to_owned(),
            chips: best
                .iter()
                .map(|candidate| candidate.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    let candidate = candidates.remove(0);
    if !candidate.path.exists() {
        return Err(AppError::LinuxGpioChipUnavailable {
            port: port.to_owned(),
            details: format!(
                "sysfs gpiochip {} is associated through {}, but {} is missing; \
                 check that gpio-cdev and udev are available",
                candidate.name,
                candidate.topology.display(),
                candidate.path.display()
            ),
        });
    }

    Ok(candidate.path)
}

fn tty_name(port: &str) -> Option<&str> {
    let name = Path::new(port).file_name()?.to_str()?;
    ["ttyUSB", "ttyACM"].iter().find_map(|prefix| {
        name.strip_prefix(prefix)
            .filter(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
            })
            .map(|_| name)
    })
}

fn is_gpiochip_name(name: &str) -> bool {
    name.strip_prefix("gpiochip").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.chars().all(|character| character.is_ascii_digit())
    })
}

fn topology_matches(port_topology: &Path, chip_topology: &Path) -> bool {
    port_topology == chip_topology
        || port_topology.starts_with(chip_topology)
        || chip_topology.starts_with(port_topology)
}

fn map_chip_error(port: &str, path: &Path, error: gpio_cdev::Error) -> AppError {
    let details = error.to_string();
    if is_permission_error(&details) {
        AppError::LinuxGpioPermission {
            port: port.to_owned(),
            path: path.display().to_string(),
            details,
        }
    } else {
        AppError::LinuxGpioChipUnavailable {
            port: port.to_owned(),
            details: format!("cannot open {}: {details}", path.display()),
        }
    }
}

fn map_line_error(port: &str, path: &Path, pin: GpioPin, details: String) -> AppError {
    if is_permission_error(&details) {
        AppError::LinuxGpioPermission {
            port: port.to_owned(),
            path: path.display().to_string(),
            details,
        }
    } else if is_busy_error(&details) {
        AppError::LinuxGpioLineInUse {
            port: port.to_owned(),
            pin: pin.number(),
            details,
        }
    } else {
        AppError::LinuxGpioLineUnavailable {
            port: port.to_owned(),
            pin: pin.number(),
            details,
        }
    }
}

fn map_gpio_operation(
    port: &str,
    path: &Path,
    operation: &str,
    error: gpio_cdev::Error,
) -> AppError {
    let details = error.to_string();
    if is_permission_error(&details) {
        AppError::LinuxGpioPermission {
            port: port.to_owned(),
            path: path.display().to_string(),
            details,
        }
    } else {
        AppError::LinuxGpioOperation {
            port: port.to_owned(),
            operation: operation.to_owned(),
            details,
        }
    }
}

fn is_permission_error(details: &str) -> bool {
    let details = details.to_ascii_lowercase();
    details.contains("permission denied") || details.contains("operation not permitted")
}

fn is_busy_error(details: &str) -> bool {
    let details = details.to_ascii_lowercase();
    details.contains("device or resource busy") || details.contains("resource busy")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::symlink,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{
        find_gpiochip_for_port, is_busy_error, is_permission_error, output_request,
        topology_matches,
    };
    use crate::{
        error::AppError,
        gpio::{GpioPin, OutputState},
    };

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "cp2102n-linux-test-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn sysfs(&self) -> PathBuf {
            self.root.join("sys")
        }

        fn device_root(&self) -> PathBuf {
            self.root.join("dev")
        }

        fn add_topology(&self, chip_names: &[&str]) {
            let device = self.root.join("devices/usb/1-1/1-1:1.0");
            fs::create_dir_all(&device).unwrap();

            let tty_link = self.sysfs().join("class/tty/ttyUSB0/device");
            fs::create_dir_all(tty_link.parent().unwrap()).unwrap();
            symlink(&device, tty_link).unwrap();

            let gpio_class = self.sysfs().join("class/gpio");
            fs::create_dir_all(&gpio_class).unwrap();
            fs::create_dir_all(self.device_root()).unwrap();
            for name in chip_names {
                let chip = gpio_class.join(name);
                fs::create_dir_all(&chip).unwrap();
                symlink(&device, chip.join("device")).unwrap();
                fs::write(self.device_root().join(name), b"fixture").unwrap();
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn plans_only_the_selected_gpio_line_and_value() {
        assert_eq!(output_request(GpioPin::Gpio2, OutputState::High), (2, 1));
        assert_eq!(output_request(GpioPin::Gpio3, OutputState::Low), (3, 0));
    }

    #[test]
    fn matches_equal_and_ancestor_topology_paths() {
        let device = Path::new("/sys/devices/usb/1-1/1-1:1.0");
        assert!(topology_matches(device, device));
        assert!(topology_matches(device, Path::new("/sys/devices/usb/1-1")));
        assert!(topology_matches(Path::new("/sys/devices/usb/1-1"), device));
        assert!(!topology_matches(
            device,
            Path::new("/sys/devices/usb/2-1/2-1:1.0")
        ));
    }

    #[test]
    fn maps_a_serial_port_to_the_matching_gpiochip_without_fixed_numbering() {
        let fixture = Fixture::new();
        fixture.add_topology(&["gpiochip17"]);

        let chip = find_gpiochip_for_port("/dev/ttyUSB0", &fixture.sysfs(), &fixture.device_root())
            .unwrap();

        assert_eq!(chip, fixture.device_root().join("gpiochip17"));
    }

    #[test]
    fn reports_missing_gpiochips_separately_from_serial_topology() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.sysfs().join("class/tty/ttyUSB0")).unwrap();
        let device = fixture.root.join("devices/usb/1-1/1-1:1.0");
        fs::create_dir_all(&device).unwrap();
        symlink(&device, fixture.sysfs().join("class/tty/ttyUSB0/device")).unwrap();
        fs::create_dir_all(fixture.sysfs().join("class/gpio")).unwrap();

        let error =
            find_gpiochip_for_port("/dev/ttyUSB0", &fixture.sysfs(), &fixture.device_root())
                .unwrap_err();

        assert!(matches!(error, AppError::LinuxGpioChipUnavailable { .. }));
        assert!(error.to_string().contains("no gpiochip"));
    }

    #[test]
    fn rejects_ambiguous_equally_specific_gpiochips() {
        let fixture = Fixture::new();
        fixture.add_topology(&["gpiochip3", "gpiochip19"]);

        let error =
            find_gpiochip_for_port("/dev/ttyUSB0", &fixture.sysfs(), &fixture.device_root())
                .unwrap_err();

        assert!(matches!(error, AppError::LinuxGpioChipAmbiguous { .. }));
        assert!(error.to_string().contains("gpiochip3, gpiochip19"));
    }

    #[test]
    fn classifies_gpio_permission_failures() {
        assert!(is_permission_error("Ioctl failed: Permission denied"));
        assert!(is_permission_error("Operation not permitted"));
        assert!(!is_permission_error("Device or resource busy"));
    }

    #[test]
    fn classifies_busy_gpio_lines() {
        assert!(is_busy_error("Device or resource busy"));
        assert!(is_busy_error("resource busy"));
        assert!(!is_busy_error("Permission denied"));
    }
}
