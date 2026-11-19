# CP2102N Serial IO Control

`cp2102n-serial-io-control` lists Silicon Labs CP2102N serial devices, reads
their runtime GPIO level, and sets the runtime output state of the Adafruit
CP2102N Friend's exposed `IO2` and `IO3` pins. It uses the normal VCP serial
driver on Windows and the kernel CP210x/GPIOLIB interface on Linux. It does
**not** replace the serial driver with WinUSB/libusb and it does not transfer
serial data for another program.

The default device identity is Silicon Labs VID `0x10C4`, PID `0xEA60`.
Custom VID/PID matching is not supported in this version.

## Library usage

The package provides both the `cp2102n-serial-io-control` executable and the
`cp2102n_serial_io_control` Rust library. Add it as a dependency with the
default CLI feature disabled when only the library is needed:

```toml
[dependencies]
cp2102n-serial-io-control = { version = "0.1", default-features = false }
```

Discover devices, select an explicit port, and use the device methods for GPIO
operations:

```rust,no_run
use cp2102n_serial_io_control::{
    GpioPin, OutputState, discover_devices, select_device,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let devices = discover_devices()?;
    let selector = "COM7".parse()?; // Use /dev/ttyUSB0 or /dev/ttyACM0 on Linux.
    let device = select_device(&devices, &selector)?;

    device.set_pin(GpioPin::Gpio2, OutputState::High)?;
    let state = device.get_pin(GpioPin::Gpio2)?;
    println!("GPIO2 is {state}");
    Ok(())
}
```

`discover_devices` returns an empty vector when no supported board is present;
the library does not turn that normal result into an error. `select_device`
requires an exact selector match and reports missing or ambiguous matches.
`get_pin` and `set_pin` return the structured library `Error` type on failure.
The crate root also exports a convenient `Result<T>` alias.

The device value contains discovery-time metadata. Discover and select the
board again after unplugging, replugging, or changing its port mapping. Each
GPIO call uses the platform backend directly; the API does not keep a serial
port or GPIO line open between calls. As with the CLI, `set_pin` performs a real
hardware write and must only be used after checking the pin, wiring, and target
voltage.

The default `cli` feature builds the command-line executable and enables
`clap`. Library-only consumers should use `default-features = false` as above;
this avoids the CLI dependency and Windows DLL staging during their build.

## Commands

Run the binary as `cp2102n-serial-io-control` below, or use `cargo run --` while
developing:

```text
# List every matching device, sorted by reported port name.
cp2102n-serial-io-control list

# Print one matching device's metadata.
cp2102n-serial-io-control list --port /dev/ttyUSB0

# Linux examples.
cp2102n-serial-io-control get --port /dev/ttyUSB0 --pin 2
cp2102n-serial-io-control set --port /dev/ttyUSB0 --pin 2 --state high
cp2102n-serial-io-control get --port /dev/ttyUSB0 --pin 3
cp2102n-serial-io-control set --port /dev/ttyUSB0 --pin 3 --state low

# Windows examples.
cp2102n-serial-io-control get --port COM7 --pin 2
cp2102n-serial-io-control set --port COM7 --pin 2 --state high
cp2102n-serial-io-control get --port COM12 --pin 3
cp2102n-serial-io-control set --port COM12 --pin 3 --state low
```

`--port` is required by `get` and `set` and must be the exact port reported by
`list`: `COM<number>` on Windows or `/dev/ttyUSB<number>`/
`/dev/ttyACM<number>` on Linux. Windows COM names are matched without regard
to case; Linux paths are matched exactly. There is no implicit "first device"
selection. A missing or ambiguous match is an error.

Only pin `2` (`GPIO2`, Adafruit `IO2`) and pin `3` (`GPIO3`, Adafruit `IO3`)
are accepted. `--state` is `high` or `low`, case-insensitively. A successful
command prints a short confirmation and exits with status zero; `get` prints
the selected runtime level as `high` or `low`, for example
`get COM7 GPIO2 = high`. `get` reads the current GPIO latch/line value and
does not write a GPIO value, change persistent configuration, or monitor
edges/events. Failures print an actionable diagnostic to stderr and return a
nonzero status.

The `list` output contains the reported port, VID/PID, USB serial number,
manufacturer, and product string. Missing USB descriptor fields are shown as
`<none>`. GPIO availability is not probed by `list`; Linux port-to-gpiochip
association and line availability are checked when `get` or `set` runs.

## Hardware wiring and safety

Before connecting a target circuit, power the board down and consult the
Adafruit CP2102N Friend documentation for the board's signal voltage and
current limits:

* Connect `IO2` to the target input controlled by GPIO2 and `IO3` to the
  target input controlled by GPIO3. Connect the board and target grounds.
* Do not tie the two CP2102N outputs together, connect either output to another
  actively driven output, or use these pins as a level shifter. Confirm that
  the target accepts the board's logic-high voltage.
* Keep the board's persistent CP2102N configuration prepared for GPIO2/GPIO3
  output use. This tool changes only the runtime latch/output state; it does
  not program configuration memory, change alternate functions, or configure
  pull/open-drain behavior.
* A reset, unplug/replug, or power cycle may restore the configured/default
  state. Do not use this command as a substitute for a safety interlock.
* Verify the selected port and pin before every hardware write. Test first on
  an unloaded input or other non-critical circuit.

The Windows backend reads the current latch for `get` and writes only the
selected bit for `set`, preserving unrelated latch bits. The Linux `get`
backend requests only the selected line as-is, without requesting input/output
direction or supplying a write value, then reads its value. It does not call a
write operation or probe/reconfigure the other supported line. Linux `set`
requests only the selected line as an output with the requested value.

## Windows setup

1. Install the Silicon Labs CP210x **VCP** driver appropriate for the board
   and confirm that the board appears as the expected `COM<number>` in Device
   Manager. Leave that driver installed; replacing it with WinUSB/libusb is not
   supported by the normal workflow.
2. Build the project normally on Windows. The build stages the vendor
   `third_party/CP210xRuntime.dll` beside the executable in each Cargo profile:

   ```text
   target\debug\cp2102n-serial-io-control.exe
   target\debug\CP210xRuntime.dll
   target\release\cp2102n-serial-io-control.exe
   target\release\CP210xRuntime.dll
   ```

   The executable loads only the DLL beside itself; a DLL found through the
   caller's working directory or `PATH` is not used. For a release
   distribution, ship the executable and `CP210xRuntime.dll` together in the
   same directory. The DLL architecture must match the executable, and it
   must export `CP210xRT_ReadLatch` and `CP210xRT_WriteLatch`. Those runtime
   functions use 16-bit `WORD` latch and mask values, which the backend
   mirrors.

   When another Rust application uses the library, the relevant executable is
   that consuming application, not the CLI. Place a compatible
   `CP210xRuntime.dll` beside the consuming executable yourself; the library
   does not copy it into the consuming application's output directory. This
   placement is required at GPIO operation time, not while compiling a
   library-only dependency with default features disabled.
3. The runtime DLL is a vendor binary. Use a Silicon Labs/vendor-approved
   CP2102N-compatible version and verify that its applicable redistribution
   terms permit shipping it with the application. Do not substitute an
   unrelated runtime DLL or replace the required VCP driver with WinUSB/libusb.
4. Check compatibility before relying on the tool in an application workflow:
   run `list`, then run `get` or test one GPIO state with the external serial
   application closed. A runtime that can enumerate a COM port but does not
   support these CP2102N runtime calls is not sufficient.

The runtime DLL is loaded dynamically, so a missing DLL, missing export, or
unsupported runtime operation is reported clearly at operation time. Runtime
DLL versions and CP2102N support can differ; keep a known-good, compatible
copy beside the deployed executable. The VCP driver remains required for COM
enumeration and access even when the runtime DLL is present.

The application opens the COM handle without sharing it. If the external
serial application already owns the port exclusively, `get` and `set` report
that the port is in use; neither command resets, reconfigures, or interrupts
that connection. Close or pause the external application, perform the read or
GPIO change, and reopen it. The VCP driver and the COM mapping remain installed
throughout.

## Linux setup

Linux uses the existing kernel `cp210x` serial driver and GPIO character
devices through `gpio-cdev`. The running kernel must provide:

* CP210x GPIO/GPIOLIB support in the `cp210x` driver;
* GPIO character-device support (`CONFIG_GPIO_CDEV`); and
* an associated `/dev/gpiochipN` exposing the CP2102N lines.

The tool does not assume `gpiochip0`. For a selected `/dev/ttyUSB*` or
`/dev/ttyACM*`, it resolves `/sys/class/tty/<tty>/device`, enumerates
`/sys/class/gpio/gpiochip*/device`, and selects the most specific matching
sysfs topology. This avoids depending on the global gpiochip number, which can
change when devices are added or reordered. An equally specific ambiguous
mapping is rejected.

The invoking user needs access to **both** the selected serial device and the
associated GPIO character device. Use the distro's udev mechanism to assign
appropriate ownership/group/mode for the board and gpiochip; a rule that only
grants access to `/dev/ttyUSB0` is not sufficient. Inspect the result with:

```text
ls -l /dev/ttyUSB0 /dev/gpiochip*
udevadm info --query=all --name=/dev/ttyUSB0
readlink -f /sys/class/tty/ttyUSB0/device
readlink -f /sys/class/gpio/gpiochipN/device
```

Replace `ttyUSB0` and `gpiochipN` with the values for the selected board.
After changing udev rules, reload the rules and replug the board according to
the distribution's procedure. Do not grant broad write access to all GPIO
chips when a narrower board-specific rule is possible.

Each Linux `get` operation opens the associated chip, requests only GPIO2 or
GPIO3 with its current configuration left as-is, reads the line value, and
releases the line. It does not call a write operation or change the line
direction. A Linux `set` operation instead requests only the selected line as
an output, sets its value, and releases the line. Because the line is not held
after the command exits, the set level is not guaranteed to persist if another
process claims or reconfigures that line, and it must not be used as a safety
interlock. A line claimed by another kernel or userspace consumer is reported
as busy rather than forcibly taken.
The selected GPIO must already be available in the CP2102N configuration;
`set` also requires it to be usable as an output.

## Port ownership and limitations

The tool does not own the external application's serial protocol or send
serial data. Discovery and device association use the reported port, while
reading or changing a GPIO may also open the VCP/tty or GPIO character device.
A process that holds the needed resource exclusively can prevent `get` or `set`
from running. The tool never kills a process, steals a GPIO line, replaces a
driver, or silently falls back to raw USB.

Supported scope is intentionally limited to the default CP2102N VID/PID,
GPIO2/GPIO3 runtime level readback and transient output changes. Serial
configuration, GPIO input/event monitoring, persistent CP2102N configuration
programming, arbitrary custom VID/PID devices, and output-mode
reconfiguration are out of scope.

## Troubleshooting

 * **No matching devices:** confirm the board is connected, the VCP/kernel
   driver is installed, and its USB identity is `10C4:EA60`. On Linux also
   check permissions for the tty. Use `list` before `get` or `set`.
* **Port selector rejected/not found:** copy the port name exactly from
  `list`; do not use a `/dev/ttyS*` path or a guessed COM number.
* **Windows runtime DLL error:** verify that the executable and
  `CP210xRuntime.dll` are in the same directory, that the DLL matches the
   executable architecture, and that it exports `CP210xRT_ReadLatch` for
   `get` and both latch functions for `set`. For a Cargo build, rebuild or
   restore the staged DLL from
  `third_party/CP210xRuntime.dll`; the loader does not fall back to `PATH`.
  Also confirm that the Silicon Labs CP210x VCP driver is installed and the
  board has an assigned COM port. Do not solve this by replacing the VCP
  driver.
 * **Windows port held exclusively:** close the external serial application and
   retry `get` or `set`. The error is intentional protection of the existing
   connection.
* **Linux no associated gpiochip:** check CP210x GPIO/GPIOLIB and
  `CONFIG_GPIO_CDEV`, `/sys/class/gpio`, udev-created `/dev/gpiochipN`, and the
  selected tty's sysfs topology. The number is not required to be zero.
* **Linux permission denied:** grant the invoking user access to both the tty
  and the associated gpiochip, then replug/recheck device permissions.
 * **GPIO line busy/unavailable:** stop the other GPIO consumer and verify that
   GPIO2/GPIO3 are configured as GPIOs rather than an alternate function. A
   `set` operation additionally requires an output-capable configuration.
* **Electrical level is unexpected:** recheck common ground, wiring, target
  voltage compatibility, pull/open-drain configuration, and whether a reset
  restored the device's configured state.

## Development and validation

Tests exercise Cargo build-directory layout handling, parsing,
identity/selector matching, latch calculations and readback, Windows pure
helpers, and Linux sysfs mapping/planning without
opening a real serial/GPIO device or writing to hardware. Hardware validation
is a separate smoke test: verify the installed driver/runtime or kernel
support, run `list`, read both supported pins with `get`, and test both states
with `set` on a non-critical circuit while monitoring the serial application.

See `AGENTS.md` for repository conventions, platform boundaries, safe hardware
rules, and the complete validation command set.