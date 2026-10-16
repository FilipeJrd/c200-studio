# C200 Studio interface proposal

The Studio direction was implemented in version 0.2.0, including the joystick,
zoom, two settings sections, and profile saving. Compact remains a proposal.
This document preserves the analysis behind the design; see README.md and
VALIDATION.md for the implemented behavior and validation.

Research date: October 16, 2026. This is a qualitative comparison of manufacturer
documentation and public screenshots against C200 Studio's UI and code. The
official apps were not installed or usability-tested. Published screenshots may
show different versions and devices from those currently available.

## Recommendation

Use **Studio**: a prominent preview on the left and a compact inspector on the
right, with two stable sections, **Framing** and **Image**. Reveal additional
settings through expansion. Use Logi Tune's simplicity to guide priorities and
AnkerWork/Camera Hub to guide grouping. Keep Rust/egui for this stage: the main
issues observed are hierarchy, composition, and control behavior. Reconsider
the framework if a visual implementation in egui reveals concrete limitations.

**Compact** is an alternative for quick adjustments, with the preview above
the controls. It can come later; the initial redesign does not need two modes.

## Benchmark

| Reference | Observed evidence | Application to C200 Studio | Limitations |
| --- | --- | --- | --- |
| [AnkerWork — Software Download Center](https://us.ankerwork.com/pages/download-software/) | Screenshots show control groups, automatic/manual modes, and visual brightness/temperature cues. | Group by intent, explain slider endpoints, and keep automatic/manual dependencies together. | Images include features of other devices, such as lighting. They are not evidence of a C200-specific screen or of features supported by the C200. |
| [Logi Tune — official Brio 500 manual, pp. 8 / 32](https://www.logitech.com/assets/66219/2/brio-500.pdf) | Compact screen with preview, direct field-of-view selection, and separate access to image adjustments. | Make framing immediately accessible and move less frequent settings to a second level. | The 2022 manual documents a layout, not the latest app version. Brio field-of-view angles differ from the C200's. |
| [Elgato Camera Hub — Settings Overview](https://help.elgato.com/hc/en-us/articles/4405055113357-Elgato-Facecam-Camera-Hub-Settings-Overview) | Device, zoom, image, exposure, white balance, and processing sections, with explicit automatic-mode dependencies. | Use coherent groups and reveal manual settings near their automatic mode. | The full panel is dense for everyday use. Saving settings to the camera is a feature of that product; do not assume equivalent C200 support. |
| [Camo Studio — Navigating Camo’s interface](https://camo.com/support/camo/how-to-use-camo) | Prominent central preview, distinct connection states, and a quick panel for frequent actions. Guide updated in July 2026. | Prioritize the image, distinguish connection failure from paused preview, and reserve a compact experience for quick use. | The complete UI has two side panels and production features. Its full density would not suit this project's scope. |

The usability recommendations below are design inferences from these references
and inspection of the MVP, rather than results measured with users.

## Issues observed in the MVP

1. A two-row header gives the brand, slogan, theme, camera, reconnect, and
   profiles similar prominence.
2. Four inspector tabs wrap onto two lines, reducing space for settings.
3. Every control has a similar title, editor, help text, and separator. Frequent
   choices and technical settings have little visual distinction.
4. Selecting a profile already applies it, but another Apply button duplicates
   the action and obscures the relationship between applying and saving.
5. Release camera requires understanding capture contention. Pause preview,
   paired with an explanation of its effect, is easier to understand.
6. Unavailable controls and raw values occupy the main flow even though they
   primarily serve diagnostics and compatibility checks.

## Proposed structure

### Header

- A restrained C200 Studio name.
- Camera name and connection state. Show a device selector when alternatives
  exist or the user needs to choose a camera.
- Preferences for appearance, preview format, information, and diagnostics.
- Contextual reconnect actions on failure, without occupying normal operation.

### Preview area

- Preserve the image's aspect ratio, without distortion or unintended cropping.
- Keep the saved profile near the image: selecting applies; saving explicitly
  updates the profile.
- Enable Save when confirmed values differ from the profile. Offer Save profile…
  on first use. Naming, save as, rename, and delete belong in the profile menu.
- Describe resolution and frame rate as **this app's preview format**. Do not
  imply they configure the resolution used by Meet, Zoom, or OBS.
- Pause preview only closes C200 Studio's capture. It does not disable the
  camera globally or interrupt another app.
- Do not add photo or recording buttons before the backend supports them.

### Inspector

| Group | Controls and behavior |
| --- | --- |
| Framing | 65° Narrow, 78° Balanced, 95° Wide; a two-axis positioning joystick, zoom, and mirroring, together at the first level. |
| Image | Brightness, autofocus, automatic white balance, and HDR, according to device capabilities. Reveal manual focus/color when automatic mode is off. |
| More image settings | Automatic/manual exposure, contrast, saturation, hue, gamma, and sharpness. Keep existing backend controls and respect dependencies. |
| Technical options | Power line frequency, raw vendor anti-flicker with an explanation, portrait mode when supported, and compatibility details. Do not invent Hz labels for undecoded vendor values. |

Preserve access to all 21 known controls through expansion and technical options,
without showing every control on the initial screen. Never silently hide read
failures: compatibility details explain unavailable capabilities; operation
failures require contextual feedback.

### Framing joystick

The first proposal was refined to keep positioning visible beside field of view,
with a two-axis virtual joystick and adjacent zoom control.

- Dragging and holding indicates direction and intensity. Diagonal movement
  acts on both axes with bounded total speed to avoid diagonal acceleration.
- Releasing centers the stick and stops movement while retaining the chosen
  framing. Returning the stick does not recenter the camera.
- Arrow keys move in small steps; Shift + arrow offers finer steps.
- Persistent state is horizontal/vertical position and zoom. The stick's
  temporary displacement is not saved in the profile.
- Gesture cancellation, focus loss, changing section, and opening a dialog
  must stop movement. Disconnecting the device must do the same.
- Use existing `pan_absolute`, `tilt_absolute`, and `zoom_absolute` controls,
  converting direction into increments bounded by device ranges and steps.
  Do not present a preview-only crop as a change that applies in other apps.
- Check which field-of-view/zoom combinations allow positioning and explain
  limits. The prototype only illustrates this relation using a bounded crop;
  it is not evidence of firmware behavior.
- Do not add automatic tracking or imply motorized camera movement. The control
  represents positioning already available through the app's backend.

The initial proposal put zoom and positioning at a second level. This revision
promotes them to Framing. Both section tabs fit on a single line.

### Applying settings and saving profiles

1. The user changes a control.
2. The interface discreetly indicates pending application without moving layout.
3. The backend confirms by readback. Only then show Applied to camera · profile
   not saved.
4. Saving persists the local profile and shows Profile Work · saved. It does
   not claim the firmware has stored a profile.
5. A failure retains the previous confirmed value and provides contextual feedback.
6. Switching profiles with unsaved changes offers save, continue without saving,
   or cancel. The original prototype only illustrates selection and dirty state;
   this decision flow must be implemented in the actual app.

Work and Daylight in the prototype are example profile names, not universal
lighting presets or automatic scene detection.

### States

| State | Message and action |
| --- | --- |
| No camera | Connect your PowerConf C200. Automatic detection and access to diagnostics when needed. |
| Connecting | Discreet progress; controls retain position while awaiting readback. |
| Preview active | Image and Pause preview action. |
| Preview paused | Capture is released so you can use the camera in a call. Resume preview action. |
| Capture unavailable | Explain that preview could not start; offer retry and details. Suggest contention only when supported by evidence. Keep controls usable when the driver permits it. |
| Disconnected during use | Keep the profile, report disconnection, and disable writes until reconnection. |

Capture failure alone does not prove another app is using the camera. Do not
identify other apps without a reliable source for that information.

## Visual direction

- Neutral palette with one mint accent; light and dark themes follow the system.
- An 8 px spacing grid; prefer 36–40 px desktop controls and larger interaction
  areas when needed.
- Control text at 14 px, group titles at 15–16 px, section titles at 18 px,
  and supporting text at 12 px with sufficient contrast.
- An inspector approximately 308–340 px wide. Reference window around
  1100 × 760; adapt to smaller sizes without wrapping navigation across rows.
- Few separators, grouping through spacing, subtle corners, and visible focus.
- Avoid continuous animations, layout jumps, success pop-ups for every slider,
  and excessive unlabeled icons.
- Optional short transitions should respect reduced motion. Prioritize smooth
  response and legibility at 100%, 150%, and 200% display scaling.

## Proposed implementation sequence

1. Create style tokens and egui components: header, segmented selector, setting
   row, toggle, expandable section, and status indicator. Start with a screen
   faithful to Studio.
2. Replace the four tabs with Framing and Image, revealing additional image
   settings progressively. Implement pointer capture, gesture cancellation,
   and keyboard access for the joystick. Respect capabilities and dependencies
   while keeping technical controls accessible.
3. Separate capture, connection, pending application, and unsaved profile states.
   Resolve profile saving without a duplicate Apply action.
4. Preserve capture/command workers. Keep device I/O out of rendering and keep
   sliders responsive while the backend confirms values.
5. Validate real-camera writes/readback, pause/release, reconnection, automatic
   focus/color, missing portrait mode, and externally changed controls.
6. Build a new AppImage after validating the redesign. The original design
   study itself did not change the executable or AppImage; implementation and
   validation followed separately.

Trying this proposal does not require a migration to Tauri. The conversational
prototype illustrates appearance and flow, not rendering parity between HTML
and egui. Evaluate a reference egui implementation before choosing a migration.

## Acceptance criteria

- The initial screen identifies the camera, shows the image, exposes framing,
  and makes the profile easy to find without opening Preferences.
- Field of view, zoom, joystick, and pause remain visible in Framing. Brightness
  is one selection away in Image; profiles do not require another screen.
- Releasing/cancelling the joystick, changing section, or losing focus stops
  movement without losing the selected position.
- The joystick supports diagonals and arrow keys without exceeding device limits.
- Manual settings clearly explain automatic-mode dependencies.
- Confirmation uses the value read back from the camera, without jumping to
  defaults during reads.
- Unavailable controls do not look usable; compatibility details explain why.
- Keyboard navigation and focus are visible, without clipped controls/text at 150%.
- Profile changes are never silently lost.
- Smaller windows retain essential actions. Compact can follow Studio later.

## Prototype limitations

Both proposals use an office illustration and approximate visual responses for
field of view, zoom, position, brightness, contrast, saturation, and mirroring.
They do not access hardware. Zoom scale and normalized position are illustrative;
the installed app must use device-reported ranges. Percentage sliders are a
presentation convention: map the driver's actual range and step, preserve raw
values in details, and show physical units only when known. The prototype covers
main flows rather than every product control and dialog.
