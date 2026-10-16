# C200 Studio

[![CI](https://github.com/FilipeJrd/c200-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/FilipeJrd/c200-studio/actions/workflows/ci.yml)
[![AppImage](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml/badge.svg)](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml)

An independent Linux app for the **Anker PowerConf C200** (`291a:3369`),
with a Rust/egui interface, live preview, framing joystick, and local camera
profiles. This is an unofficial project, unaffiliated with Anker, and does
not depend on the original C executable.

## Run

```sh
cargo run --release -p c200-app
```

Without a camera, try the interface and profiles with a simulated device:

```sh
cargo run --release -p c200-app -- --demo
```

Demo mode does not access hardware and stores settings in a separate temporary
directory. Use `--config /path/to/settings.json` to choose another file,
including when you want to keep demo settings between sessions.

## Features

- Studio interface with a large preview, **Framing** and **Image** sections,
  and separate preferences for appearance, capture, and diagnostics.
- Two-axis virtual joystick: drag and hold to move; release to stop.
  Arrow keys adjust the position; Shift uses the driver's smallest step.
- C200 discovery and automatic reconnection, without assuming `/dev/video0`.
- Vendor controls for 65°, 78°, and 95° field of view, HDR, mirroring,
  and portrait mode when supported by the firmware.
- V4L2 brightness, contrast, saturation, hue, gamma, sharpness, power line
  frequency, white balance, focus, exposure, pan, tilt, and zoom.
- Control ranges, menu entries, and availability queried from the driver.
- MJPEG/YUYV preview with resolution and frame rate selection; prefers 720p30.
- Profiles bound to a camera and reapplied at startup or reconnection while
  the app is open; changes confirmed by reading values back from the device.
- Light/dark themes, HiDPI, and keyboard operation.
- JSON diagnostics and a demo mode for development.

In **Image → Technical options**, vendor anti-flicker is a raw `0..255` byte,
separate from standard V4L2 power line frequency. The meaning of these values
has not been confirmed. Raw field of view is also available for parity with
the original CLI.

During calls, choose **Pause preview** to stop streaming and release capture
for another app. Controls continue to work when the driver permits it.
An unavailable preview does not block the other controls. C200 Studio does
not provide a virtual camera or capture audio.

**Framing** brings together field of view, the joystick, zoom, and mirroring.
Portrait mode appears when supported by the firmware. **Image** includes
brightness, focus, color, and HDR; manual controls appear when their automatic
mode is turned off. Exposure and additional settings are in expandable groups.
**Preferences** shows device capabilities and reasons for unavailable controls.

The joystick uses the device's real V4L2 pan/tilt controls, limits, and steps.
Returning the stick to its center stops sending new positions; it leaves
your framing in place. With a real camera, the preview displays the image
received from the device. Demo mode simulates cropping so you can try the
control without hardware.

## Build on Linux

Install the Rust toolchain specified in `rust-toolchain.toml`, including
Cargo, rustfmt, and Clippy. With rustup, the toolchain file selects and installs
these automatically. The app uses egui/eframe 0.33, direct V4L2 access, and
image decoding in Rust. It does not require Qt, GTK for rendering, GStreamer,
or `v4l2-ctl` at runtime. Building V4L2 requires clang and kernel headers;
OpenGL and Wayland/X11 libraries are provided by the system.

On CachyOS/Arch:

```sh
sudo pacman -S --needed rust clang pkgconf linux-api-headers libxkbcommon libxkbcommon-x11 wayland libx11 libxi libxrandr libxcursor libglvnd
cargo build --release --locked -p c200-app
./target/release/c200-studio
```

On Ubuntu 22.04/24.04 (install Rust separately):

```sh
sudo apt-get install clang libclang-dev pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev libudev-dev libdbus-1-dev
cargo build --release --locked -p c200-app
```

## Settings and profiles

Settings are stored in `$XDG_CONFIG_HOME/c200-studio/settings.json`, or
`~/.config/c200-studio/settings.json`. Configuration files are not written
beside the executable or AppImage. Invalid files and files with a newer schema
version are preserved; the app reports the problem instead of overwriting them.

Adjust the camera, wait for confirmation, and choose **Save profile…** or
**Save**. Saving with the current name updates that profile; selecting a
profile applies it. Later changes only enter the profile when you save again.
Switching profiles or cameras with unsaved changes offers to save, continue
without saving, or cancel. Status messages distinguish pending confirmation
from settings already applied to the camera but not yet saved in a profile.
The profile menu offers rename, save as, and delete actions.

Profiles are not applied to a camera with a different identity. If the camera
has no USB serial number, its identity depends on the physical USB port/hub;
moving it may require creating or selecting a profile again. Profile names
are user content: switching the app to English does not rename existing profiles
or change the settings schema.

The app does not install a service, start automatically at login, or promise
firmware persistence. Closing the window stops capture and device monitoring.
Starting without a selected profile only reads the controls.

## Diagnostics and tests

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p c200-app -- --diagnose
cargo run -p c200-app -- --probe-preview
cargo run -p c200-app -- --demo --smoke-test --screenshot dist/demo.png
```

`--diagnose` only reads controls. `--probe-preview` opens capture, negotiates
a video mode, receives ten frames, and releases resources, without saving
images or changing image controls. Check these on real hardware as well;
simulated tests do not prove firmware compatibility.

The following optional test requires a C200. It changes pan, tilt, and zoom
by one step, confirms readback, and restores the original values. It is
excluded from normal test runs and CI:

```sh
cargo test -p c200-app --locked framing_controls_round_trip_on_hardware -- --ignored --nocapture
```

Axis directions and exposure follow the
[kernel V4L2 reference](https://docs.kernel.org/userspace-api/media/v4l/ext-ctrls-camera.html).
Zoom is shown as a multiplier when the C200 reports a base of 100; raw values
and limits remain available in Preferences and control tooltips.

If a device cannot be opened, check session ACLs and your distribution's
device access rules. Run the app as a normal user. If capture is busy, stop
capture in the other app or keep only camera controls active in C200 Studio.
The app cannot detect the physical lens cover.

## AppImage

```sh
cargo install cargo-about --version 0.9.2 --locked --features cli
bash scripts/appimage.sh
chmod +x dist/C200-Studio-x86_64.AppImage
./dist/C200-Studio-x86_64.AppImage
```

The script uses linuxdeploy and includes license notices. To target Ubuntu
22.04 and newer systems, build on Ubuntu 22.04, as in CI. An AppImage built
on CachyOS may require a newer glibc and is suitable for local testing rather
than establishing portability. Without FUSE, use `--appimage-extract-and-run`.

To build on Ubuntu 22.04 using Docker:

```sh
bash scripts/portable-appimage.sh
```

## Releases

There are currently no versioned GitHub Releases or release tags. The
[AppImage workflow](https://github.com/FilipeJrd/c200-studio/actions/workflows/appimage.yml)
builds development packages for pull requests and pushes to `main`. Its
artifacts contain an x86_64 AppImage, a SHA-256 checksum, and demo screenshots,
and are retained for 14 days. They are tied to the tested commit.

Tagging a version does not currently publish a release. See
[the release guide](docs/releases.md) for the current process, manual release
steps, and the proposed tag-based automation.

## Project structure

- `c200-core`: domain model, validation, ordered profile application, and mock backend.
- `c200-linux`: sysfs discovery, V4L2, Extension Unit controls, and capture.
- `c200-app`: GUI, control worker, profiles, and preview lifecycle.

Hotplug is checked every two seconds through sysfs, without a daemon.
Capture and camera I/O run outside the UI thread, keeping only the latest
pending frame. Changing the capture mode preserves the session's image settings.

The vendor protocol was ported from Eran Sandler's project. See
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the source, revision, and license.

## Contributing

Pull requests run formatting checks, Clippy, and tests on Ubuntu 22.04/24.04,
plus AppImage packaging and GUI smoke tests. Development packages and demo
screenshots are available in the AppImage workflow artifacts.
See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow.
