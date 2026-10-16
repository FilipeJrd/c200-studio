# User guide

See the [README](../README.md) for downloads, first steps, and build requirements.
This guide covers day-to-day camera use, profiles, and troubleshooting.

## Framing and image controls

**Framing** brings together field of view, the joystick, zoom, and mirroring.
Portrait mode appears when supported by the firmware. **Image** includes
brightness, focus, white balance, and HDR. Manual focus and color controls
appear when their automatic mode is turned off. Exposure and other adjustments
are in expandable groups. **Preferences** shows capabilities and reasons for
unavailable controls.

The joystick uses the real V4L2 pan/tilt controls, limits, and steps. Drag and
hold to move; release to stop. Returning the stick to its center leaves your
framing in place. Arrow keys adjust the position; Shift uses the driver's
smallest step. Zoom is shown as a multiplier when the C200 reports a base of
100; raw values and ranges remain available in Preferences and tooltips.

With a real camera, preview displays the image received from the device.
Demo mode simulates cropping so you can try framing without hardware. Axis
directions and exposure follow the
[kernel V4L2 reference](https://docs.kernel.org/userspace-api/media/v4l/ext-ctrls-camera.html).

In **Image → Technical options**, vendor anti-flicker is a raw `0..255` byte,
separate from standard V4L2 power line frequency. The meaning of these values
has not been confirmed. Raw field of view is also available for parity with
the original CLI.

## Use the camera in a call

Choose **Pause preview** before using the camera in another app. This stops
C200 Studio's stream and releases capture. Controls continue to work when
the driver permits it; an unavailable preview does not block other controls.
Choose **Resume preview** when you want the live view back.

Each app chooses its own capture resolution and frame rate. Preview format
in C200 Studio only affects this app. C200 Studio does not provide a virtual
camera or capture audio. The physical lens cover cannot be detected by the app.

## Try demo mode

```sh
cargo run --release -p c200-app -- --demo
```

Or pass `--demo` to the AppImage. Demo mode does not access hardware and stores
settings in a separate temporary directory. Use `--config /path/to/settings.json`
to choose another file, including when you want to keep demo profiles.

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


## Troubleshooting

- **Camera cannot be opened:** check the USB connection, session ACLs, and your
  distribution's device access rules. Run the app as a normal user.
- **Preview is busy:** stop capture in the other app or keep C200 Studio's
  preview paused while using camera controls.
- **A control is unavailable:** check **Preferences → Compatibility and controls**.
  Availability depends on the device's driver, firmware, and automatic modes.
- **Settings cannot be saved:** check the path shown in the error. Invalid files
  and files with a newer schema are preserved so you can fix or move them.

See [VALIDATION.md](../VALIDATION.md) for tested environments and known hardware
limitations, and [CONTRIBUTING.md](../CONTRIBUTING.md) for development checks.
