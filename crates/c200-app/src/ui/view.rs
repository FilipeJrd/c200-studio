use super::*;
use crate::interaction::keyboard_target;
use egui::{Align, Align2, FontId, Layout, Pos2, Rect, Sense, Stroke};

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

impl Studio {
    pub(super) fn control_values(&self) -> BTreeMap<String, i64> {
        self.controls
            .iter()
            .filter(|c| c.available && c.writable && !c.volatile)
            .filter_map(|c| c.value.map(|v| (c.key.clone(), v)))
            .collect()
    }
    pub(super) fn recompute_dirty(&mut self) {
        let baseline = self
            .selected_profile()
            .map(|p| &p.values)
            .unwrap_or(&self.baseline);
        self.dirty = self
            .controls
            .iter()
            .filter(|c| c.available && c.writable && !c.volatile && !c.inactive)
            .any(|c| {
                c.value.is_some_and(|v| {
                    baseline
                        .get(&c.key)
                        .or_else(|| self.baseline.get(&c.key))
                        .is_some_and(|old| *old != v)
                })
            });
    }
    pub(super) fn stop_motion(&mut self) {
        self.motion_blocked = true;
        for axis in &mut self.motion {
            axis.stop();
        }
    }
    fn control(&self, key: &str) -> Option<Control> {
        self.controls.iter().find(|c| c.key == key).cloned()
    }
    fn value(&self, key: &str) -> Option<i64> {
        self.control(key).and_then(|c| self.edits.value(&c))
    }
    fn available(&self, key: &str) -> bool {
        self.control(key)
            .is_some_and(|c| c.available && c.value.is_some())
    }

    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        self.stop_motion();
        if self.dirty || self.edits.busy() {
            self.pending_action = Some(action);
            self.dialog = Some(ProfileDialog::Unsaved);
        } else {
            self.perform_action(action, ctx);
        }
    }
    fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        self.dialog = None;
        self.pending_action = None;
        match action {
            PendingAction::Profile(name) => {
                let previous = self.settings.clone();
                self.settings.selected_profile = name;
                if !self.save_settings() {
                    self.settings = previous;
                    return;
                }
                if self.selected_profile().is_some() {
                    self.apply_profile();
                } else {
                    self.baseline = self.control_values();
                    self.recompute_dirty();
                }
            }
            PendingAction::Device(device) => self.connect(device),
            PendingAction::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    fn finish_action(&mut self, ctx: &egui::Context) {
        self.dialog = None;
        if let Some(action) = self.pending_action.take() {
            self.perform_action(action, ctx);
        }
    }
    fn store_profile(&mut self, name: String) -> bool {
        if !self.ready_to_save() {
            return false;
        }
        let previous = self.settings.clone();
        let profile = Profile {
            name: name.clone(),
            device_id: self.device.as_ref().unwrap().id.clone(),
            values: self.control_values(),
        };
        if let Err(error) = self.settings.store(profile) {
            self.notice = Some((error.to_string(), true));
            return false;
        }
        self.settings.selected_profile = Some(name);
        if !self.save_settings() {
            self.settings = previous;
            return false;
        }
        self.baseline = self.control_values();
        self.recompute_dirty();
        self.notice = None;
        true
    }
    fn save_current(&mut self) {
        if let Some(name) = self.selected_profile().map(|p| p.name.clone()) {
            self.store_profile(name);
        } else {
            self.profile_name.clear();
            self.dialog = Some(ProfileDialog::Save);
        }
    }
    fn ready_to_save(&self) -> bool {
        self.device.is_some()
            && self
                .controls
                .iter()
                .any(|c| c.available && c.writable && c.value.is_some())
            && self.config_writable
            && !self.edits.busy()
            && !self.applying
            && !self.controls.iter().any(|c| {
                c.available && c.writable && !c.volatile && !c.inactive && c.value.is_none()
            })
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let p = Palette::get(ui.ctx());
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 36.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                widgets::brand(ui);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::settings_icon(ui).clicked() {
                        self.stop_motion();
                        self.settings_open = true;
                    }
                    let status = if self.connecting {
                        "Connecting…"
                    } else if self.device.is_some() {
                        "Connected"
                    } else {
                        "No camera"
                    };
                    ui.label(RichText::new(status).size(12.0).color(p.muted));
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        3.0,
                        if self.device.is_some() {
                            p.accent
                        } else {
                            p.muted
                        },
                    );
                    if self.devices.len() > 1 {
                        let mut selected = self.wanted_device.clone();
                        egui::ComboBox::from_id_salt("camera")
                            .width(200.0)
                            .selected_text(
                                self.device
                                    .as_ref()
                                    .map(|d| d.name.as_str())
                                    .unwrap_or("Select camera"),
                            )
                            .show_ui(ui, |ui| {
                                for device in &self.devices {
                                    ui.selectable_value(
                                        &mut selected,
                                        Some(device.id.clone()),
                                        &device.name,
                                    );
                                }
                            });
                        if selected != self.wanted_device
                            && let Some(device) = self
                                .devices
                                .iter()
                                .find(|d| Some(&d.id) == selected.as_ref())
                                .cloned()
                        {
                            self.request_action(PendingAction::Device(device), ui.ctx());
                        }
                    } else if ui.available_width() > 190.0 {
                        ui.label(
                            RichText::new(if self.demo {
                                "PowerConf C200 · demo"
                            } else {
                                "PowerConf C200"
                            })
                            .size(13.0),
                        );
                    }
                });
            },
        );
    }
    fn profile_picker(&mut self, ui: &mut egui::Ui) {
        if self.applying || self.device.is_none() {
            ui.disable();
        }
        let p = Palette::get(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Profile").size(12.0).color(p.muted));
            let before = self.settings.selected_profile.clone();
            let mut selected = before.clone();
            egui::ComboBox::from_id_salt("profile")
                .width(150.0)
                .selected_text(before.as_deref().unwrap_or("No profile"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, "No profile");
                    for profile in &self.settings.profiles {
                        if self
                            .device
                            .as_ref()
                            .is_some_and(|d| d.id == profile.device_id)
                        {
                            ui.selectable_value(
                                &mut selected,
                                Some(profile.name.clone()),
                                &profile.name,
                            );
                        }
                    }
                });
            if selected != before {
                self.request_action(PendingAction::Profile(selected), ui.ctx());
            }
            let can_save = self.ready_to_save();
            if widgets::primary(
                ui,
                if self.selected_profile().is_some() {
                    "Save"
                } else {
                    "Save profile…"
                },
                can_save && (self.dirty || self.selected_profile().is_none()),
            )
            .clicked()
            {
                self.save_current();
            }
            ui.menu_button("···", |ui| {
                if ui
                    .add_enabled(can_save, egui::Button::new("Save as…"))
                    .clicked()
                {
                    self.profile_name.clear();
                    self.dialog = Some(ProfileDialog::Save);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.selected_profile().is_some() && !self.edits.busy(),
                        egui::Button::new("Rename profile…"),
                    )
                    .clicked()
                {
                    self.profile_name = self.settings.selected_profile.clone().unwrap_or_default();
                    self.dialog = Some(ProfileDialog::Rename);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.selected_profile().is_some() && !self.edits.busy(),
                        egui::Button::new("Delete profile…"),
                    )
                    .clicked()
                {
                    self.dialog = Some(ProfileDialog::Delete);
                    ui.close();
                }
            })
            .response
            .on_hover_text("Profile options");
        });
    }
    fn settings_panel(&mut self, ui: &mut egui::Ui) {
        let p = Palette::get(ui.ctx());
        ui.label(RichText::new("Camera settings").strong().size(15.0));
        ui.add_space(16.0);
        egui::Frame::new()
            .fill(p.background)
            .corner_radius(9)
            .inner_margin(4)
            .show(ui, |ui| {
                ui.columns(2, |columns| {
                    for (column, (group, label)) in columns
                        .iter_mut()
                        .zip([(Group::Framing, "Framing"), (Group::Image, "Image")])
                    {
                        let active = self.group == group;
                        let button =
                            egui::Button::new(RichText::new(label).size(13.0).color(if active {
                                p.accent
                            } else {
                                p.text
                            }))
                            .fill(if active {
                                p.panel
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE);
                        if column
                            .add_sized([column.available_width(), 32.0], button)
                            .clicked()
                        {
                            self.group = group;
                            self.stop_motion();
                        }
                    }
                });
            });
        ui.add_space(18.0);
        if self.device.is_none() {
            ui.label(RichText::new("Settings appear when a camera is connected.").color(p.muted));
            return;
        }
        let enabled = !self.applying && self.dialog.is_none() && !self.settings_open;
        ui.add_enabled_ui(enabled, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("inspector-controls")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.group == Group::Framing {
                        self.framing_settings(ui);
                    } else {
                        self.image_settings(ui);
                    }
                });
        });
    }
    fn framing_settings(&mut self, ui: &mut egui::Ui) {
        let p = Palette::get(ui.ctx());
        ui.label("Field of view");
        ui.add_space(4.0);
        if let Some(control) = self
            .control("fov")
            .filter(|c| c.available && c.value.is_some())
        {
            let value = self.edits.value(&control);
            let width = (ui.available_width() - 16.0) / 3.0;
            ui.horizontal(|ui| {
                for (degrees, label) in [(65, "Narrow"), (78, "Balanced"), (95, "Wide")] {
                    let selected = value == Some(degrees);
                    if widgets::fov_choice(
                        ui,
                        width,
                        degrees,
                        label,
                        selected,
                        control.validate(degrees).is_ok(),
                    )
                    .clicked()
                    {
                        self.queue("fov".into(), degrees);
                    }
                }
            });
        } else {
            ui.label(
                RichText::new("Field of view unavailable")
                    .small()
                    .color(p.muted),
            );
        }
        ui.add_space(22.0);
        ui.label("Position and zoom");
        ui.add_space(6.0);
        let pan = self.control("pan_absolute");
        let tilt = self.control("tilt_absolute");
        let supports_motion = [&pan, &tilt]
            .into_iter()
            .flatten()
            .any(|c| c.editable() && c.value.is_some() && c.min < c.max);
        let can_move = supports_motion && !self.motion_blocked;
        let mut direction = [0.0, 0.0];
        let mut keyboard = [0, 0];
        ui.horizontal(|ui| {
            let (response, vector) = widgets::joystick(ui, can_move && ui.is_enabled());
            direction = vector;
            if can_move && response.has_focus() && ui.is_enabled() {
                ui.input_mut(|i| {
                    keyboard[0] = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                        as i64
                        - i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) as i64;
                    keyboard[1] = i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) as i64
                        - i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) as i64;
                    if i.modifiers.shift {
                        keyboard[0] += i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowRight)
                            as i64
                            - i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowLeft) as i64;
                        keyboard[1] += i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowUp)
                            as i64
                            - i.consume_key(egui::Modifiers::SHIFT, egui::Key::ArrowDown) as i64;
                    }
                });
            }
            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), 112.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.add_space(25.0);
                    self.editor(ui, "zoom_absolute", "Zoom", false);
                },
            );
        });
        let (dt, precise) = ui.input(|i| (i.stable_dt, i.modifiers.shift));
        if self.dialog.is_some() || self.settings_open || !ui.input(|i| i.focused) {
            direction = [0.0, 0.0];
            keyboard = [0, 0];
        }
        for (index, control) in [pan.as_ref(), tilt.as_ref()].into_iter().enumerate() {
            if let Some(control) = control
                && let Some(value) = self.edits.value(control)
            {
                let axis_direction = if index == 0 {
                    direction[0]
                } else {
                    -direction[1]
                };
                let next = if keyboard[index] != 0 {
                    keyboard_target(control, value, keyboard[index], precise)
                } else {
                    self.motion[index].advance(control, value, axis_direction, dt, precise)
                };
                if let Some(next) = next {
                    self.queue(control.key.clone(), next);
                }
            }
        }
        if direction != [0.0, 0.0] {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        ui.add_space(4.0);
        let positions: Vec<_> = [("pan_absolute", "H"), ("tilt_absolute", "V")]
            .into_iter()
            .filter_map(|(key, label)| {
                self.control(key).and_then(|c| {
                    self.edits
                        .value(&c)
                        .map(|value| format!("{label} {value:+}"))
                })
            })
            .collect();
        ui.label(
            RichText::new(self.position_description())
                .size(12.0)
                .color(p.muted),
        )
        .on_hover_text(positions.join("   ·   "));
        ui.label(
            RichText::new(if supports_motion {
                "Drag and hold, or use the arrow keys.\nRelease to stop. Hold Shift for precision."
            } else {
                "This device does not support adjustable positioning."
            })
            .size(12.0)
            .color(p.muted),
        );
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);
        self.toggle_control(
            ui,
            "horizontal_flip",
            "Mirror image",
            Some("Also affects other apps"),
        );
        if self.available("vertical_screen") {
            self.toggle_control(ui, "vertical_screen", "Portrait mode", None);
        }
    }
    fn position_description(&self) -> String {
        let mut parts = Vec::new();
        let mut readable = false;
        for (key, positive, negative) in [
            ("pan_absolute", "Right", "Left"),
            ("tilt_absolute", "Up", "Down"),
        ] {
            if let Some(control) = self.control(key).filter(|c| c.available)
                && let Some(value) = self.edits.value(&control)
            {
                readable = true;
                let center = control.default.unwrap_or(0).clamp(control.min, control.max);
                let offset = value as f64 - center as f64;
                let span = if offset >= 0.0 {
                    control.max as f64 - center as f64
                } else {
                    center as f64 - control.min as f64
                };
                if offset != 0.0 && span > 0.0 {
                    parts.push(format!(
                        "{} {:.0}%",
                        if offset > 0.0 { positive } else { negative },
                        (offset.abs() / span * 100.0).clamp(1.0, 100.0)
                    ));
                }
            }
        }
        if !readable {
            "Position unavailable".into()
        } else if parts.is_empty() {
            "Centered".into()
        } else {
            parts.join(" · ")
        }
    }
    fn image_settings(&mut self, ui: &mut egui::Ui) {
        self.editor(ui, "brightness", "Brightness", false);
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);
        self.toggle_control(
            ui,
            "focus_automatic_continuous",
            "Autofocus",
            Some("Let the camera focus for you"),
        );
        if self.value("focus_automatic_continuous") != Some(1) {
            self.editor(ui, "focus_absolute", "Manual focus", false);
        }
        ui.add_space(8.0);
        self.toggle_control(
            ui,
            "white_balance_automatic",
            "Auto white balance",
            Some("Let the camera adjust color"),
        );
        if self.value("white_balance_automatic") != Some(1) {
            self.editor(ui, "white_balance_temperature", "Color temperature", false);
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);
        self.toggle_control(ui, "hdr", "HDR", Some("Balance bright and dark areas"));
        ui.add_space(16.0);
        egui::CollapsingHeader::new("More settings")
            .id_salt("more-image")
            .show(ui, |ui| {
                for (key, label) in [
                    ("contrast", "Contrast"),
                    ("saturation", "Saturation"),
                    ("sharpness", "Sharpness"),
                ] {
                    self.editor(ui, key, label, false);
                    ui.add_space(12.0);
                }
                self.editor(ui, "auto_exposure", "Exposure", false);
                if self
                    .value("auto_exposure")
                    .is_none_or(|value| value == 1 || value == 2)
                {
                    self.editor(ui, "exposure_time_absolute", "Exposure time", false);
                }
                ui.add_space(8.0);
                self.editor(ui, "power_line_frequency", "Lighting frequency", false);
            });
        ui.add_space(6.0);
        egui::CollapsingHeader::new("Technical options").id_salt("technical").show(ui, |ui| {
            self.editor(ui,"hue","Hue",false);
            self.editor(ui,"gamma","Gamma",false);
            self.editor(ui,"anti_flicker","Vendor anti-flicker",true);
            ui.label(RichText::new("Raw value, separate from lighting frequency. The meaning of these values has not been confirmed.").small().color(Palette::get(ui.ctx()).muted));
            ui.add_space(12.0);
            self.editor(ui,"fov","Field of view · raw value",true);
            self.editor(ui,"pan_absolute","Horizontal position",true);
            self.editor(ui,"tilt_absolute","Vertical position",true);
        });
    }
    fn toggle_control(&mut self, ui: &mut egui::Ui, key: &str, label: &str, hint: Option<&str>) {
        let Some(control) = self.control(key).filter(|c| c.available) else {
            return;
        };
        let Some(value) = self.edits.value(&control) else {
            ui.label(RichText::new(format!("{label} · reading unavailable")).small());
            return;
        };
        let mut checked = value != 0;
        let response = ui
            .add_enabled_ui(control.editable(), |ui| {
                widgets::toggle(ui, label, hint, &mut checked)
            })
            .inner;
        if response.changed() {
            self.queue(key.into(), i64::from(checked));
        }
        if let Some(reason) = &control.reason {
            response.on_hover_text(reason);
        }
    }
    fn editor(&mut self, ui: &mut egui::Ui, key: &str, label: &str, raw: bool) {
        let Some(control) = self.control(key).filter(|c| c.available) else {
            return;
        };
        ui.push_id((key, raw), |ui| {
            let p = Palette::get(ui.ctx());
            let Some(original) = self.edits.value(&control) else {
                ui.label(
                    RichText::new(format!("{label} · unconfirmed"))
                        .small()
                        .color(p.warning),
                );
                return;
            };
            let mut value = original;
            if matches!(control.kind, ControlKind::Integer) || raw {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if key == "white_balance_temperature" && !raw {
                                format!("{value} K")
                            } else if key == "zoom_absolute" && control.min == 100 && !raw {
                                format!("{:.2}×", value as f64 / 100.0)
                            } else {
                                value.to_string()
                            })
                            .size(12.0)
                            .color(p.muted),
                        );
                    });
                });
            } else {
                ui.label(label);
            }
            ui.add_enabled_ui(control.editable() && control.min < control.max, |ui| {
                if raw || matches!(control.kind, ControlKind::Integer) {
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = ui.available_width();
                        integer_editor(ui, &mut value, &control);
                    });
                } else if let ControlKind::Menu(items) = &control.kind {
                    let text = items
                        .iter()
                        .find(|(id, _)| *id == value)
                        .map(|(_, name)| name.clone())
                        .unwrap_or_else(|| format!("Value {value}"));
                    egui::ComboBox::from_id_salt("value")
                        .width(ui.available_width() - 8.0)
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            for (id, label) in items {
                                ui.selectable_value(&mut value, *id, label);
                            }
                        });
                } else if matches!(control.kind, ControlKind::Boolean) {
                    let mut checked = value != 0;
                    if ui.checkbox(&mut checked, "Enabled").changed() {
                        value = i64::from(checked);
                    }
                }
            })
            .response
            .on_hover_text(format!(
                "Device range: {} to {} · step {}{}",
                control.min,
                control.max,
                control.step,
                control
                    .reason
                    .as_ref()
                    .map(|reason| format!("\n{reason}"))
                    .unwrap_or_default()
            ));
            if control.editable() && value != original {
                self.queue(key.into(), value);
            }
            if key == "brightness" && !raw {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Darker").small().color(p.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("Brighter").small().color(p.muted));
                    });
                });
            }
        });
    }
    fn preview_panel(&mut self, ui: &mut egui::Ui) {
        let p = Palette::get(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Your preview").size(19.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let width = if self.selected_profile().is_some() {
                    328.0_f32
                } else {
                    374.0_f32
                };
                ui.allocate_ui_with_layout(
                    Vec2::new(width.min(ui.available_width()), 36.0),
                    Layout::left_to_right(Align::Center),
                    |ui| self.profile_picker(ui),
                );
            });
        });
        ui.add_space(18.0);
        let room = ui.available_size();
        let max_height = (room.y - 108.0).max(130.0);
        let size = Vec2::new(
            room.x.min(max_height * 16.0 / 9.0),
            max_height.min(room.x * 9.0 / 16.0),
        );
        let (outer, _) = ui.allocate_exact_size(Vec2::new(room.x, size.y), Sense::hover());
        let rect = Rect::from_center_size(outer.center(), size);
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect, 12, Color32::from_rgb(16, 22, 24));
        if let Some(texture) = &self.texture {
            let image_size = texture.size_vec2();
            let scale = (rect.width() / image_size.x).min(rect.height() / image_size.y);
            let image_rect = Rect::from_center_size(rect.center(), image_size * scale);
            let uv = if self.demo {
                self.demo_uv()
            } else {
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0))
            };
            painter.image(texture.id(), image_rect, uv, Color32::WHITE);
            let badge = Rect::from_min_size(
                rect.min + Vec2::splat(12.0),
                Vec2::new(if self.demo { 104.0 } else { 75.0 }, 25.0),
            );
            painter.rect_filled(badge, 5, Color32::from_black_alpha(150));
            painter.text(
                badge.center(),
                Align2::CENTER_CENTER,
                if self.demo { "Demo" } else { "Preview active" },
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        } else {
            let (title, detail) = if self.connecting {
                ("Connecting camera…", "Reading camera settings.")
            } else if self.device.is_none() {
                (
                    "Connect your PowerConf C200",
                    "Your camera will be detected automatically.",
                )
            } else if !self.settings.preview_enabled {
                (
                    "Preview paused",
                    "Capture is released so you can use the camera in a call.",
                )
            } else if self.preview_message.starts_with("Starting") {
                ("Starting preview…", "Preparing video capture.")
            } else {
                (
                    "Preview unavailable",
                    "Resume preview to try again. Camera settings remain available.",
                )
            };
            let title_galley = painter.layout(
                title.into(),
                FontId::proportional(19.0),
                Color32::from_rgb(237, 242, 243),
                (rect.width() - 48.0).max(20.0),
            );
            let detail_galley = painter.layout(
                detail.into(),
                FontId::proportional(13.0),
                Color32::from_rgb(166, 180, 185),
                (rect.width() - 64.0).max(20.0),
            );
            painter.galley(
                rect.center() - Vec2::new(title_galley.size().x / 2.0, 36.0),
                title_galley,
                Color32::WHITE,
            );
            painter.galley(
                rect.center() + Vec2::new(-detail_galley.size().x / 2.0, 1.0),
                detail_galley,
                p.muted,
            );
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(
                    self.mode
                        .as_ref()
                        .map(|m| format!("{} × {} · {} fps", m.width, m.height, m.fps))
                        .unwrap_or_else(|| "No capture".into()),
                )
                .small()
                .color(p.muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let running = self.preview_active || self.preview_message.starts_with("Starting");
                if ui
                    .add_enabled(
                        self.device.is_some() && self.mode.is_some() && !self.applying,
                        egui::Button::new(if running {
                            "Pause preview"
                        } else {
                            "Resume preview"
                        })
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    self.settings.preview_enabled = !running;
                    self.stop_motion();
                    self.send(Command::Preview(if self.settings.preview_enabled {
                        self.mode.clone()
                    } else {
                        None
                    }));
                    self.save_settings();
                }
            });
        });
        ui.add_space(10.0);
        ui.label(RichText::new("Settings apply to the camera in real time. Pause this preview to use the camera in a call.").small().color(p.muted));
        if self.device.is_none() && !self.connecting && ui.button("Find camera").clicked() {
            if let Some(device) = self
                .devices
                .iter()
                .find(|d| Some(&d.id) == self.wanted_device.as_ref())
                .cloned()
            {
                self.connect(device);
            } else {
                self.send(Command::Scan);
            }
        }
    }
    fn demo_uv(&self) -> Rect {
        let value = |key: &str, fallback: i64| {
            self.controls
                .iter()
                .find(|c| c.key == key)
                .and_then(|c| c.value)
                .unwrap_or(fallback) as f32
        };
        let fov = value("fov", 78).max(30.0);
        let zoom = (value("zoom_absolute", 100) / 100.0).max(1.0);
        let span = (fov / 95.0 / zoom).clamp(0.05, 1.0);
        let pan = value("pan_absolute", 0) / 3600.0;
        let tilt = value("tilt_absolute", 0) / 3600.0;
        let center = Pos2::new(
            0.5 + pan * (1.0 - span) / 2.0,
            0.5 - tilt * (1.0 - span) / 2.0,
        );
        let mut uv = Rect::from_center_size(center, Vec2::splat(span));
        if value("horizontal_flip", 0) != 0.0 {
            std::mem::swap(&mut uv.min.x, &mut uv.max.x);
        }
        uv
    }
    fn preferences(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Preferences").open(&mut open).collapsible(false).resizable(true).default_width(460.0).max_height((ctx.content_rect().height()-70.0).max(240.0)).anchor(Align2::CENTER_CENTER,Vec2::ZERO).vscroll(true).show(ctx, |ui| {
            let p = Palette::get(ctx);
            ui.label(RichText::new("Appearance").strong());
            let mut theme = self.settings.theme.clone();
            ui.horizontal(|ui| {
                for (value,label) in [(None,"System"),(Some("light".into()),"Light"),(Some("dark".into()),"Dark")] { ui.selectable_value(&mut theme,value,label); }
            });
            if theme != self.settings.theme {
                ctx.set_theme(match theme.as_deref() {Some("light")=>egui::ThemePreference::Light,Some("dark")=>egui::ThemePreference::Dark,_=>egui::ThemePreference::System});
                self.settings.theme = theme; self.save_settings();
            }
            ui.add_space(16.0); ui.label(RichText::new("Preview format").strong());
            ui.label(RichText::new("This only affects capture in C200 Studio. Each app chooses its own resolution.").small().color(p.muted));
            let before = self.mode.clone();
            egui::ComboBox::from_id_salt("capture-mode").width(330.0).selected_text(self.mode.as_ref().map(CaptureMode::label).unwrap_or_else(||"No mode available".into())).show_ui(ui, |ui| { for mode in &self.modes { ui.selectable_value(&mut self.mode,Some(mode.clone()),mode.label()); } });
            if self.mode!=before && self.settings.preview_enabled { self.send(Command::Preview(self.mode.clone())); }
            ui.add_space(16.0); ui.separator(); ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(self.device.is_some() && !self.edits.busy() && !self.applying,egui::Button::new("Refresh camera settings")).clicked() { self.send(Command::Refresh); }
                if ui.add_enabled(!self.connecting && !self.edits.busy(),egui::Button::new("Reconnect…")).clicked() {
                    if let Some(device) = self.device.clone().or_else(||self.devices.iter().find(|d|Some(&d.id)==self.wanted_device.as_ref()).cloned()) { self.request_action(PendingAction::Device(device),ctx); self.settings_open=false; }
                    else { self.send(Command::Scan); }
                }
            });
            if let Some((message,error))=&self.notice { ui.add_space(10.0); ui.label(RichText::new(message).color(if *error {p.warning} else {p.muted})); }
            if !self.preview_active && self.settings.preview_enabled { ui.label(RichText::new(&self.preview_message).small().color(p.muted)); }
            ui.add_space(10.0);
            egui::CollapsingHeader::new("Compatibility and controls").show(ui, |ui| {
                for control in &self.controls {
                    ui.label(RichText::new(&control.label).strong());
                    if control.available { ui.label(RichText::new(format!("{} · range {}…{} · step {}{}",control.value.map(|v|v.to_string()).unwrap_or_else(||"unconfirmed".into()),control.min,control.max,control.step,if control.inactive {" · automatic mode active"} else {""})).small().color(p.muted)); }
                    if let Some(reason)=&control.reason { ui.label(RichText::new(reason).small().color(p.muted)); }
                    ui.add_space(8.0);
                }
            });
            ui.add_space(14.0); ui.separator();
            ui.label(RichText::new(format!("C200 Studio {} · independent, unofficial app",env!("CARGO_PKG_VERSION"))).small().color(p.muted));
            ui.label(RichText::new("Profiles are stored locally. Closing the app stops capture and device monitoring.").small().color(p.muted));
        });
        if !open {
            self.settings_open = false;
        }
    }
    fn profile_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut skip = false;
        let mut cancel = false;
        let mut delete = false;
        let title = match dialog {
            ProfileDialog::Save => "Save profile",
            ProfileDialog::Rename => "Rename profile",
            ProfileDialog::Delete => "Delete profile?",
            ProfileDialog::Unsaved => "Save profile changes?",
        };
        egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER,Vec2::ZERO).show(ctx, |ui| {
            ui.set_max_width(390.0);
            match dialog {
                ProfileDialog::Save | ProfileDialog::Rename => {
                    ui.label("Profile name");
                    ui.add(egui::TextEdit::singleline(&mut self.profile_name).desired_width(300.0).hint_text("e.g. Work, Daylight…"));
                    ui.label(RichText::new("Reapplied when you open the app or reconnect this camera.").small().color(Palette::get(ctx).muted));
                    ui.horizontal(|ui| { save=widgets::primary(ui,"Save",!self.profile_name.trim().is_empty() && !self.edits.busy() && !self.applying).clicked(); cancel=ui.button("Cancel").clicked(); });
                }
                ProfileDialog::Unsaved => {
                    ui.label("These settings are already on the camera. Save a profile to reuse them later.");
                    if self.edits.busy() { ui.label("Waiting for the latest settings to be confirmed…"); }
                    ui.horizontal_wrapped(|ui| { save=widgets::primary(ui,"Save and continue",!self.edits.busy() && !self.applying && self.config_writable).clicked(); skip=ui.add_enabled(!self.edits.busy() && !self.applying,egui::Button::new("Continue without saving")).clicked(); cancel=ui.button("Cancel").clicked(); });
                }
                ProfileDialog::Delete => {
                    ui.label(format!("Delete the profile “{}”?",self.settings.selected_profile.as_deref().unwrap_or("")));
                    ui.label("Current camera settings will be kept.");
                    ui.horizontal(|ui| { delete=ui.button("Delete profile").clicked(); cancel=ui.button("Cancel").clicked(); });
                }
            }
            if let Some((message,true))=&self.notice { ui.add_space(8.0); ui.label(RichText::new(message).small().color(Palette::get(ctx).warning)); }
        });
        if !open || cancel {
            self.dialog = None;
            self.pending_action = None;
            return;
        }
        if skip {
            self.finish_action(ctx);
        }
        if delete {
            let previous = self.settings.clone();
            let id = self.device.as_ref().map(|d| d.id.as_str());
            let name = self.settings.selected_profile.as_deref();
            self.settings
                .profiles
                .retain(|p| !(Some(p.device_id.as_str()) == id && Some(p.name.as_str()) == name));
            self.settings.selected_profile = None;
            if self.save_settings() {
                self.baseline = self.control_values();
                self.recompute_dirty();
                self.dialog = None;
            } else {
                self.settings = previous;
            }
        }
        if save {
            match dialog {
                ProfileDialog::Unsaved => {
                    if let Some(name) = self.settings.selected_profile.clone() {
                        if self.store_profile(name) {
                            self.finish_action(ctx);
                        }
                    } else {
                        self.profile_name.clear();
                        self.dialog = Some(ProfileDialog::Save);
                    }
                }
                ProfileDialog::Save => {
                    let name = self.profile_name.trim().to_string();
                    let exists = self
                        .device
                        .as_ref()
                        .is_some_and(|d| self.settings.profile(&name, &d.id).is_some());
                    if exists && self.settings.selected_profile.as_deref() != Some(name.as_str()) {
                        self.notice = Some((
                            "A profile with this name already exists. Choose another name.".into(),
                            true,
                        ));
                    } else if self.store_profile(name) {
                        self.finish_action(ctx);
                    }
                }
                ProfileDialog::Rename => {
                    let name = self.profile_name.trim().to_string();
                    let previous = self.settings.clone();
                    let device_id = self
                        .device
                        .as_ref()
                        .map(|d| d.id.clone())
                        .unwrap_or_default();
                    let old = self.settings.selected_profile.clone();
                    if old.as_deref() == Some(name.as_str()) {
                        self.dialog = None;
                        return;
                    }
                    if self.settings.profile(&name, &device_id).is_some() {
                        self.notice =
                            Some(("A profile with this name already exists.".into(), true));
                        return;
                    }
                    if let Some(profile) = self
                        .settings
                        .profiles
                        .iter_mut()
                        .find(|p| p.device_id == device_id && Some(&p.name) == old.as_ref())
                    {
                        profile.name = name.clone();
                    }
                    self.settings.selected_profile = Some(name);
                    if self.save_settings() {
                        self.dialog = None;
                    } else {
                        self.settings = previous;
                    }
                }
                ProfileDialog::Delete => (),
            }
        }
    }
    pub(super) fn render(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.pointer.primary_down()) {
            self.motion_blocked = false;
        }
        Palette::install(ctx);
        self.events(ctx);
        if self.group != Group::Framing
            || self.settings_open
            || self.dialog.is_some()
            || !ctx.input(|i| i.focused)
        {
            self.stop_motion();
        }
        let p = Palette::get(ctx);
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(22, 14))
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0_f32, p.line)),
            )
            .show(ctx, |ui| self.toolbar(ui));
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(24, 10))
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0_f32, p.line)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let (text, color) = if self.applying {
                        ("Applying profile…".into(), p.muted)
                    } else if self.edits.busy() {
                        ("Confirming camera settings…".into(), p.muted)
                    } else if self.dirty {
                        ("Applied to camera · profile not saved".into(), p.warning)
                    } else if let Some(profile) = self.selected_profile() {
                        (format!("Profile {} · saved", profile.name), p.muted)
                    } else {
                        ("No profile · current camera settings".into(), p.muted)
                    };
                    ui.label(RichText::new(text).small().color(color));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("Changes also apply in other apps")
                                .small()
                                .color(p.muted),
                        );
                    });
                });
            });
        if self.notice.as_ref().is_some_and(|(_, error)| *error) {
            egui::TopBottomPanel::bottom("notice")
                .frame(
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(24, 6))
                        .fill(p.soft),
                )
                .show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("An operation could not be completed.").color(p.warning),
                        );
                        if ui.small_button("View details").clicked() {
                            self.settings_open = true;
                        }
                        if ui.small_button("Close").clicked() {
                            self.notice = None;
                        }
                    });
                });
        }
        egui::SidePanel::right("settings")
            .exact_width(336.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .inner_margin(22)
                    .fill(p.panel)
                    .stroke(Stroke::new(1.0_f32, p.line)),
            )
            .show(ctx, |ui| self.settings_panel(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(24).fill(p.background))
            .show(ctx, |ui| {
                ui.add_enabled_ui(self.dialog.is_none(), |ui| self.preview_panel(ui));
            });
        self.preferences(ctx);
        self.profile_dialog(ctx);
        self.automation(ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && !self.args.smoke_test
            && self.args.screenshot.is_none()
            && self.selected_profile().is_some()
            && (self.dirty || self.edits.busy())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.dialog.is_none() {
                self.request_action(PendingAction::Close, ctx);
            }
        }
        ctx.request_repaint_after(
            if self.preview_active
                || self.edits.busy()
                || self.args.smoke_test
                || self.args.screenshot.is_some()
            {
                Duration::from_millis(33)
            } else {
                Duration::from_millis(200)
            },
        );
    }
}

impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
}
