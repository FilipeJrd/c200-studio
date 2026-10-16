use super::*;

/// Explicit opt-in: change one framing control at a time, then restore it.
#[test]
#[ignore = "Requires a connected C200; briefly writes and restores pan, tilt and zoom"]
fn framing_controls_round_trip_on_hardware() {
    let mut camera = LinuxCamera::default();
    let device = camera
        .devices()
        .unwrap()
        .into_iter()
        .next()
        .expect("Connect a C200");
    camera.connect(&device).unwrap();
    let original = camera.controls().unwrap();
    let mut errors = Vec::new();
    for key in ["pan_absolute", "tilt_absolute", "zoom_absolute"] {
        let control = original.iter().find(|c| c.key == key).unwrap();
        let value = control.value.expect("Control must be readable");
        let target = if value.saturating_add(control.step) <= control.max {
            value + control.step
        } else {
            value - control.step
        };
        control.validate(target).unwrap();
        if let Err(error) = camera.write(key, target) {
            errors.push(format!("{key}: {error}"));
        }
        let read = camera.controls().ok().and_then(|controls| {
            controls
                .into_iter()
                .find(|c| c.key == key)
                .and_then(|c| c.value)
        });
        if read != Some(target) {
            errors.push(format!("{key}: readback did not confirm {target}"));
        }
        // Restore before asserting, even when writing or confirmation failed.
        let restored = camera.write(key, value);
        let read = camera.controls().ok().and_then(|controls| {
            controls
                .into_iter()
                .find(|c| c.key == key)
                .and_then(|c| c.value)
        });
        if restored.is_err() || read != Some(value) {
            errors.push(format!("{key}: ORIGINAL VALUE COULD NOT BE CONFIRMED"));
            break;
        }
        println!("{key}: write/readback validated; original value {value} restored");
    }
    camera.disconnect();
    assert!(errors.is_empty(), "{}", errors.join("; "));
}
