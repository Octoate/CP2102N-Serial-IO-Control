# Repository conventions

This repository contains a Rust 2024 command-line tool for the Adafruit
CP2102N Friend. Keep changes narrowly scoped to the documented CP2102N
workflow: enumerate the default Silicon Labs device (`VID 0x10C4`,
`PID 0xEA60`) and set only GPIO2/GPIO3 runtime output state while preserving
the normal VCP serial connection.

## Source layout

* `Cargo.toml` and `Cargo.lock` define the Rust 2024 binary and locked
  dependencies.
* `src/main.rs` parses the CLI, discovers the selected device, dispatches the
  command, and converts errors to a nonzero exit status without panicking.
* `src/cli.rs` contains the `clap` command model and validated `list`/`set`
  arguments.
* `src/device.rs` contains the fixed VID/PID identity, USB metadata model, and
  `COM<number>`/`/dev/ttyUSB<number>`/`/dev/ttyACM<number>` selector rules.
* `src/discovery.rs` wraps `serialport::available_ports()`, filters CP2102N
  USB entries, sorts output by port name, and resolves exact selectors.
* `src/error.rs` contains user-facing structured diagnostics.
* `src/gpio.rs` contains the platform-neutral GPIO2/GPIO3 and high/low types,
  Windows latch read-modify-write helpers, and controller tests.
* `src/platform/mod.rs` is the platform boundary. `windows.rs` uses the
  VCP COM handle and dynamically loaded `CP210xRuntime.dll`; `linux.rs` uses
  the existing kernel GPIOLIB character device and sysfs topology; and
  `unsupported.rs` reports unsupported operating systems.
* `README.md` is the user/operator guide. Keep hardware prerequisites and
  safety limitations there when behavior changes.

## Dependency and platform rules

Common dependencies belong in `[dependencies]`. Keep `libloading` and
`windows-sys` under `cfg(windows)` and keep `gpio-cdev` under
`cfg(target_os = "linux")`; do not make Linux APIs or Windows FFI part of the
shared modules. Preserve the `cfg` isolation so the opposite platform can be
compile-checked without the other platform's APIs.

The normal path must retain the installed CP210x VCP driver and the user's
COM/tty mapping. Do not add a raw USB (`WinUSB`/`libusb`) fallback as a way to
avoid VCP/runtime or kernel/GPIOLIB errors. Windows runtime ABI details stay
inside `src/platform/windows.rs`; Linux GPIO discovery stays inside
`src/platform/linux.rs`.

Do not silently broaden device support. GPIO numbers are deliberately limited
to GPIO2 and GPIO3, corresponding to Adafruit IO2 and IO3. Port selection must
remain explicit; do not infer a device from enumeration order, a fixed
`gpiochip0`, or a guessed COM number.

## Safe hardware-operation rules

* Tests and ordinary builds must never write a real GPIO. Hardware writes are
  reserved for an explicitly requested manual smoke test on a non-critical
  circuit.
* Before a manual `set`, confirm the exact port, GPIO number, desired state,
  board wiring, common ground, and target voltage. Never connect two actively
  driven outputs together or use IO2/IO3 as a level shifter.
* Keep persistent CP2102N GPIO/alternate-function configuration outside this
  tool. The application changes transient output state only; it must not
  rewrite configuration memory or silently change electrical mode.
* Windows must report an exclusive-port/open failure instead of resetting or
  disrupting an external serial application. Linux must report a busy GPIO
  line instead of stealing it.
* Preserve unrelated outputs: Windows changes only the selected latch bit via
  read-modify-write; Linux requests only the selected line and does not probe
  or reconfigure the other supported line.
* Keep diagnostics actionable and return errors to `main`; do not add panic,
  unwrap, or expect paths to normal user/device failures.

## Tests and validation

The default test suite is hardware-independent. Add pure tests for parsing,
VID/PID matching, selector resolution, GPIO bit/value planning, status/error
translation, and Linux sysfs association. Backend tests may use fake
controllers or temporary topology fixtures, but must not open a host COM/tty,
GPIO character device, or physical board by default.

Run these commands from the repository root before handing off a change:

```text
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

When targets are installed, compile-check both platform configurations without
performing hardware I/O:

```text
cargo check --tests --target x86_64-pc-windows-msvc
cargo check --tests --target x86_64-unknown-linux-gnu
```

Use the target triples available from `rustup target list --installed`; do not
claim a cross-platform check passed when the target, linker, sysroot, or Linux
`libudev` development files are unavailable. Linux cross-compilation may need
a configured Linux sysroot/pkg-config environment because `serialport` uses
`libudev` there. A native Linux run is preferred for Linux-only tests when
available.

Do not run `set` as part of automated tests or CI. Hardware smoke tests are
manual and must be documented with the exact platform prerequisites, selected
port, selected pin, expected level, and cleanup/reopen procedure for the
external serial application.

## Documentation expectations

Update `README.md` when commands, driver/runtime prerequisites, GPIO mapping,
permissions, port ownership, or known limitations change. Document any new
platform-specific dependency in both `Cargo.toml` and the platform sections of
the README. Keep examples explicit about the port and pin, and never present a
fixed gpiochip number as a portable setup requirement.