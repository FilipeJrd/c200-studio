use super::*;

fn studio() -> (egui::Context, Studio, tempfile::TempDir) {
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let mut studio = Studio::with_context(
        &ctx,
        Args {
            demo: true,
            ..Default::default()
        },
        dir.path().join("settings.json"),
    );
    studio.device = Some(DeviceInfo {
        id: "demo:c200".into(),
        name: "Demo".into(),
        path: "demo".into(),
    });
    studio.controls = c200_core::mock::mock_controls();
    studio.baseline = studio.control_values();
    (ctx, studio, dir)
}

#[test]
fn switching_profiles_keeps_unsaved_changes_until_a_decision() {
    let (ctx, mut studio, _dir) = studio();
    assert!(studio.store_profile("Work".into()));
    studio
        .controls
        .iter_mut()
        .find(|c| c.key == "pan_absolute")
        .unwrap()
        .value = Some(100);
    studio.recompute_dirty();
    assert!(studio.dirty);
    studio.request_action(PendingAction::Profile(None), &ctx);
    assert!(matches!(studio.dialog, Some(ProfileDialog::Unsaved)));
    assert_eq!(studio.settings.selected_profile.as_deref(), Some("Work"));
}

#[test]
fn profile_save_waits_for_readback_and_rolls_back_on_disk_failure() {
    let (_ctx, mut studio, dir) = studio();
    assert!(studio.store_profile("Work".into()));
    studio.queue("pan_absolute".into(), 100);
    assert!(!studio.store_profile("Pending".into()));
    studio.edits.clear();
    let original = studio.settings.selected_profile.clone();
    std::fs::write(dir.path().join("blocked"), "not a directory").unwrap();
    studio.config = dir.path().join("blocked/settings.json");
    assert!(!studio.store_profile("Failed".into()));
    assert_eq!(studio.settings.selected_profile, original);
    assert!(studio.settings.profile("Failed", "demo:c200").is_none());
    studio
        .controls
        .iter_mut()
        .find(|c| c.key == "pan_absolute")
        .unwrap()
        .value = None;
    assert!(!studio.ready_to_save());
}

#[test]
fn automatic_sensor_readings_do_not_mark_a_profile_as_edited() {
    let (_ctx, mut studio, _dir) = studio();
    assert!(studio.store_profile("Work".into()));
    let temperature = studio
        .controls
        .iter_mut()
        .find(|c| c.key == "white_balance_temperature")
        .unwrap();
    temperature.inactive = true;
    temperature.value = Some(168);
    studio.recompute_dirty();
    assert!(!studio.dirty);
    studio
        .controls
        .iter_mut()
        .find(|c| c.key == "brightness")
        .unwrap()
        .value = Some(130);
    studio.recompute_dirty();
    assert!(studio.dirty);
}
