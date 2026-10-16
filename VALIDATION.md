# Version 0.2.0 validation

## Development

- System Rust 1.99.0, CachyOS x86_64, KDE/Wayland.
- `cargo fmt --all -- --check` and Clippy with `-D warnings`.
- 24 automated tests covering protocol encoding, limits and menus, automatic
  mode dependencies, profiles, write confirmation, capture release, and preview.
- Joystick: bounded diagonal speed, accumulation of movement smaller than a
  driver step, stopping on release/focus loss, and keyboard steps.
- Continuous writes batched at most every 80 ms, with worker-side coalescing
  and sequence tickets to prevent older acknowledgements from clearing new edits.
- Profiles: decisions before switching with unsaved changes, waiting for
  readback, restoring in-memory state after a failed save, and automatic sensor
  readings that do not mark a profile as edited.
- Studio demo interface rendered and inspected in light and dark themes,
  including the joystick, zoom, and profile controls.
- A window with the real camera opened and closed by the smoke test using
  temporary settings and without applying profiles.

## Hardware

Anker PowerConf C200, USB `291a:3369`:

- Discovered as `/dev/video0`, excluding the metadata node.
- Read 20 of the 21 catalogued controls, including field of view, HDR,
  horizontal mirroring, and vendor anti-flicker.
- `vertical_screen` is unsupported by this unit's firmware and is shown as
  unavailable. Other controls remain usable.
- MJPEG preview at 1280×720, 30 FPS: ten frames decoded and capture released.
- Nine format/resolution/frame-rate combinations enumerated, up to 2560×1440 MJPEG.

The opt-in framing test was also run on the C200 on October 16, 2026:

- Pan and tilt: step 3,600, range -36,000 to 36,000; one-step writes,
  readback, and restoration of both original values to 0 confirmed.
- Zoom: range 100 to 400; one-step write, readback, and restoration of the
  original value 295 confirmed.

Other write tests use simulated transports. The full set of 21 controls was
not exercised through real writes, and cropping was not visually measured at
every field-of-view/zoom combination. Successful pan/tilt/zoom readback does
not establish compatibility with all C200 firmware versions.

## Distribution

The Dockerfile builds the AppImage on Ubuntu 22.04, runs tests, and opens the
package in virtual X11 with software rendering. That check produces
`dist/ubuntu22-demo.png`. Also smoke-test the package on the target desktop
before distribution.

The 0.2.0 AppImage passed the Ubuntu 22.04 GUI check, normal FUSE launch on
CachyOS, and a real ten-frame C200 capture. Its SHA-256 checksum was verified.
The virtual check explicitly uses X11 and a 25-second timeout; running
`xvfb-run` as container PID 1 can prevent its startup protocol from completing.

GitHub Actions also passed formatting, workflow validation, Clippy, and all
24 automated tests on Ubuntu 22.04 and 24.04. The AppImage workflow builds
on Ubuntu 22.04, opens both the native binary and packaged app in demo mode,
checks screenshots were created, and verifies the package checksum.

These automated Ubuntu 24.04 checks do not replace dedicated GUI and hardware
validation on that desktop. Other desktop/distribution combinations still
need dedicated validation; AppImage generation alone does not prove compatibility.
Published release status and development artifacts are described in
[the release guide](docs/releases.md).
