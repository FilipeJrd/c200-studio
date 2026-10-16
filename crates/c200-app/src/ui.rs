use crate::{
    Args,
    controller::{Command, Controller, Event},
    interaction::{AxisMotion, Edits},
    widgets::{self, Palette},
};
use c200_core::{
    profiles::{Profile, Settings},
    *,
};
use eframe::egui::{self, Color32, RichText, Vec2};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};

mod view;

#[derive(Clone, Copy)]
enum ProfileDialog {
    Save,
    Rename,
    Delete,
    Unsaved,
}

#[derive(Clone)]
enum PendingAction {
    Profile(Option<String>),
    Device(DeviceInfo),
    Close,
}

pub struct Studio {
    controller: Controller,
    settings: Settings,
    config: PathBuf,
    config_writable: bool,
    devices: Vec<DeviceInfo>,
    device: Option<DeviceInfo>,
    wanted_device: Option<String>,
    connecting: bool,
    controls: Vec<Control>,
    modes: Vec<CaptureMode>,
    mode: Option<CaptureMode>,
    preview_active: bool,
    preview_message: String,
    texture: Option<egui::TextureHandle>,
    group: Group,
    edits: Edits,
    baseline: BTreeMap<String, i64>,
    motion: [AxisMotion; 2],
    motion_blocked: bool,
    dirty: bool,
    applying: bool,
    notice: Option<(String, bool)>,
    dialog: Option<ProfileDialog>,
    profile_name: String,
    settings_open: bool,
    pending_action: Option<PendingAction>,
    allow_close: bool,
    demo: bool,
    started: Instant,
    args: Args,
    screenshot_requested: bool,
}

impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, args: Args, config: PathBuf) -> Self {
        Self::with_context(&cc.egui_ctx, args, config)
    }
    fn with_context(ctx: &egui::Context, args: Args, config: PathBuf) -> Self {
        let (settings, config_writable, notice) = match Settings::load(&config) {
            Ok(settings) => (settings, true, None),
            Err(error) => (
                Settings::default(),
                false,
                Some((
                    format!(
                        "{error} Fix or move {} to allow saving settings.",
                        config.display()
                    ),
                    true,
                )),
            ),
        };
        ctx.set_theme(match settings.theme.as_deref() {
            Some("light") => egui::ThemePreference::Light,
            Some("dark") => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        });
        Palette::install(ctx);
        Self {
            wanted_device: settings.selected_device.clone(),
            controller: Controller::new(args.demo),
            settings,
            config,
            config_writable,
            devices: Vec::new(),
            device: None,
            connecting: false,
            controls: Vec::new(),
            modes: Vec::new(),
            mode: None,
            preview_active: false,
            preview_message: "Waiting for camera".into(),
            texture: None,
            group: Group::Framing,
            edits: Edits::default(),
            baseline: BTreeMap::new(),
            motion: Default::default(),
            motion_blocked: false,
            dirty: false,
            applying: false,
            notice,
            dialog: None,
            profile_name: String::new(),
            settings_open: false,
            pending_action: None,
            allow_close: false,
            demo: args.demo,
            started: Instant::now(),
            args,
            screenshot_requested: false,
        }
    }
    fn send(&mut self, command: Command) {
        if self.controller.commands.send(command).is_err() {
            self.notice = Some((
                "The camera worker has stopped. Restart the app.".into(),
                true,
            ));
        }
    }
    fn save_settings(&mut self) -> bool {
        if let Some(device) = &self.device {
            if let Some(name) = &self.settings.selected_profile {
                self.settings
                    .selected_profiles
                    .insert(device.id.clone(), name.clone());
            } else {
                self.settings.selected_profiles.remove(&device.id);
            }
        }
        if !self.config_writable {
            self.notice = Some((
                format!(
                    "Configuration preserved: fix or move {} before saving.",
                    self.config.display()
                ),
                true,
            ));
            return false;
        }
        if let Err(error) = self.settings.save(&self.config) {
            self.notice = Some((format!("Could not save: {error}"), true));
            return false;
        }
        true
    }
    fn selected_profile(&self) -> Option<&Profile> {
        let device = self.device.as_ref()?;
        self.settings
            .profile(self.settings.selected_profile.as_deref()?, &device.id)
    }
    fn connect(&mut self, device: DeviceInfo) {
        self.settings.selected_profile = self
            .settings
            .selected_profiles
            .get(&device.id)
            .cloned()
            .or_else(|| {
                if self.settings.selected_device.as_deref() == Some(&device.id) {
                    self.settings.selected_profile.clone()
                } else {
                    None
                }
            });
        let profile = self
            .settings
            .selected_profile
            .as_deref()
            .and_then(|name| self.settings.profile(name, &device.id))
            .map(|p| p.values.clone());
        self.wanted_device = Some(device.id.clone());
        self.connecting = true;
        self.device = None;
        self.modes.clear();
        self.mode = None;
        self.preview_message = "Connecting camera…".into();
        self.applying = profile.is_some();
        self.controls.clear();
        self.texture = None;
        self.preview_active = false;
        self.edits.clear();
        self.stop_motion();
        self.dialog = None;
        self.pending_action = None;
        self.send(Command::Connect {
            device,
            profile,
            preview: self.settings.preview_enabled,
        });
    }
    fn events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.controller.events.try_recv() {
            match event {
                Event::Devices(devices) => {
                    self.devices = devices;
                    if self.device.is_none() && !self.connecting {
                        let selected = self
                            .wanted_device
                            .as_deref()
                            .and_then(|id| self.devices.iter().find(|d| d.id == id))
                            .cloned()
                            .or_else(|| {
                                if self.devices.len() == 1 {
                                    self.devices.first().cloned()
                                } else {
                                    None
                                }
                            });
                        if let Some(device) = selected {
                            self.connect(device);
                        }
                    }
                }
                Event::Connected {
                    device,
                    controls,
                    modes,
                    selected_mode,
                } => {
                    self.settings.selected_device = Some(device.id.clone());
                    self.device = Some(device);
                    self.connecting = false;
                    self.controls = controls;
                    self.baseline = self.control_values();
                    self.modes = modes;
                    self.mode = selected_mode;
                    self.dirty = false;
                    self.save_settings();
                }
                Event::Snapshot { controls, ack } => {
                    self.controls = controls;
                    self.edits.acknowledge(&ack);
                    self.recompute_dirty();
                }
                Event::WriteFailed(ack) => {
                    let uncertain = self.edits.acknowledge(&ack);
                    for control in &mut self.controls {
                        if uncertain.contains(&control.key) {
                            control.value = None;
                            control.reason = Some(
                                "Value unconfirmed. Refresh camera settings in Preferences.".into(),
                            );
                        }
                    }
                    self.recompute_dirty();
                }
                Event::Applied(errors) => {
                    self.applying = false;
                    self.recompute_dirty();
                    self.dirty |= !errors.is_empty();
                    self.notice = if errors.is_empty() {
                        None
                    } else {
                        Some((
                            format!("Profile partially applied: {}", errors.join("; ")),
                            true,
                        ))
                    };
                }
                Event::Preview {
                    active,
                    message,
                    mode,
                } => {
                    self.preview_active = active;
                    self.preview_message = message;
                    if let Some(mode) = mode {
                        self.mode = Some(mode);
                    }
                    if !active {
                        self.texture = None;
                    }
                }
                Event::Disconnected(message) => {
                    self.device = None;
                    self.connecting = false;
                    self.controls.clear();
                    self.modes.clear();
                    self.mode = None;
                    self.texture = None;
                    self.preview_active = false;
                    self.preview_message = message;
                    self.edits.clear();
                    self.stop_motion();
                    self.dialog = None;
                    self.pending_action = None;
                    self.applying = false;
                }
                Event::Error(message) => {
                    self.notice = Some((message, true));
                    self.stop_motion();
                }
            }
        }
        let frame = self
            .controller
            .frames
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        if let Some(frame) = frame {
            let image = egui::ColorImage::from_rgb(
                [frame.width as usize, frame.height as usize],
                &frame.rgb,
            );
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture =
                    Some(ctx.load_texture("camera-preview", image, egui::TextureOptions::LINEAR));
            }
        }
        let now = Instant::now();
        if let Some((ticket, values)) = self.edits.take_due(now) {
            self.send(Command::Write { ticket, values });
        }
    }
    fn queue(&mut self, key: String, value: i64) {
        if let Some(control) = self.controls.iter().find(|control| control.key == key)
            && control.validate(value).is_ok()
            && self.edits.value(control) != Some(value)
        {
            self.edits.queue(key, value, Instant::now());
        }
    }
    fn apply_profile(&mut self) {
        if let Some(values) = self.selected_profile().map(|p| p.values.clone()) {
            self.baseline = self.control_values();
            self.edits.clear();
            self.stop_motion();
            self.applying = true;
            self.send(Command::Apply(values));
        }
    }
    fn automation(&mut self, ctx: &egui::Context) {
        if self.args.screenshot.is_some()
            && !self.screenshot_requested
            && self.started.elapsed() > Duration::from_secs(2)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.screenshot_requested = true;
        }
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot
            && let Some(path) = &self.args.screenshot
        {
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            match image::save_buffer(
                path,
                &rgba,
                image.width() as u32,
                image.height() as u32,
                image::ColorType::Rgba8,
            ) {
                Ok(()) => {
                    eprintln!("Screenshot: {}", path.display());
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(error) => {
                    eprintln!("Could not save screenshot: {error}");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if self.args.smoke_test && self.started.elapsed() > Duration::from_secs(4) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn integer_editor(ui: &mut egui::Ui, value: &mut i64, control: &Control) -> egui::Response {
    ui.scope(|ui| {
        let p = Palette::get(ui.ctx());
        let style = ui.style_mut();
        style.visuals.selection.bg_fill = p.accent;
        style.visuals.widgets.inactive.bg_fill = p.inset;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.5_f32, p.accent);
        style.visuals.widgets.hovered.bg_fill = p.accent;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::NONE;
        style.visuals.widgets.active.bg_fill = p.accent;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::NONE;
        style.spacing.interact_size.y = 22.0;
        ui.add(
            egui::Slider::new(value, control.min..=control.max)
                .clamping(egui::SliderClamping::Edits)
                .step_by(control.step.max(1) as f64)
                .integer()
                .show_value(false)
                .trailing_fill(true)
                .handle_shape(egui::style::HandleShape::Circle),
        )
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toolbar_keeps_space_for_preview_and_controls() {
        let ctx = egui::Context::default();
        let args = Args {
            demo: true,
            ..Default::default()
        };
        let mut studio = Studio::with_context(
            &ctx,
            args,
            std::env::temp_dir().join(format!(
                "c200-layout-test-{}/settings.json",
                std::process::id()
            )),
        );
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1180.0, 760.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::TopBottomPanel::top("test-toolbar").show(ctx, |ui| {
                studio.toolbar(ui);
                assert!(ui.min_rect().height() < 160.0);
            });
        });
    }
    #[test]
    fn rendering_out_of_range_auto_values_does_not_edit_camera_state() {
        let control = Control {
            value: Some(168),
            inactive: true,
            ..c200_core::mock::mock_controls()
                .into_iter()
                .find(|c| c.key == "white_balance_temperature")
                .unwrap()
        };
        let mut value = 168;
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_enabled_ui(false, |ui| {
                    assert!(!integer_editor(ui, &mut value, &control).changed());
                });
            });
        });
        assert_eq!(value, 168);
    }
}
