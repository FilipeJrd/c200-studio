use eframe::egui::{self, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Vec2};

#[cfg(test)]
#[path = "widgets_tests.rs"]
mod tests;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub panel: Color32,
    pub inset: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub soft: Color32,
    pub warning: Color32,
}
impl Palette {
    pub fn get(ctx: &egui::Context) -> Self {
        let rgb = Color32::from_rgb;
        if ctx.style().visuals.dark_mode {
            Self {
                background: rgb(19, 23, 25),
                panel: rgb(28, 34, 37),
                inset: rgb(37, 45, 49),
                text: rgb(237, 242, 243),
                muted: rgb(166, 180, 185),
                line: rgb(50, 62, 67),
                accent: rgb(116, 215, 190),
                accent_text: rgb(17, 43, 37),
                soft: rgb(37, 62, 55),
                warning: rgb(242, 204, 140),
            }
        } else {
            Self {
                background: rgb(244, 245, 246),
                panel: Color32::WHITE,
                inset: rgb(241, 243, 244),
                text: rgb(32, 44, 48),
                muted: rgb(89, 106, 112),
                line: rgb(220, 226, 228),
                accent: rgb(10, 116, 104),
                accent_text: Color32::WHITE,
                soft: rgb(228, 243, 238),
                warning: rgb(134, 91, 19),
            }
        }
    }
    pub fn install(ctx: &egui::Context) {
        let p = Self::get(ctx);
        ctx.style_mut(|s| {
            s.text_styles
                .insert(egui::TextStyle::Body, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Button, FontId::proportional(14.0));
            s.text_styles
                .insert(egui::TextStyle::Small, FontId::proportional(12.0));
            s.text_styles
                .insert(egui::TextStyle::Heading, FontId::proportional(18.0));
            s.spacing.item_spacing = Vec2::new(8.0, 8.0);
            s.spacing.button_padding = Vec2::new(12.0, 8.0);
            s.spacing.interact_size = Vec2::new(36.0, 36.0);
            s.spacing.slider_width = 200.0;
            s.visuals.override_text_color = Some(p.text);
            s.visuals.panel_fill = p.panel;
            s.visuals.window_fill = p.panel;
            s.visuals.extreme_bg_color = p.background;
            s.visuals.faint_bg_color = p.inset;
            s.visuals.window_corner_radius = 12.into();
            s.visuals.window_stroke = Stroke::new(1.0_f32, p.line);
            s.visuals.selection.bg_fill = p.soft;
            s.visuals.selection.stroke = Stroke::new(1.0_f32, p.accent);
            s.visuals.hyperlink_color = p.accent;
            s.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.muted);
            s.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.line);
            s.visuals.widgets.inactive.bg_fill = p.inset;
            s.visuals.widgets.inactive.weak_bg_fill = p.inset;
            s.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.line);
            s.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
            s.visuals.widgets.inactive.corner_radius = 7.into();
            s.visuals.widgets.hovered.bg_fill = p.soft;
            s.visuals.widgets.hovered.weak_bg_fill = p.soft;
            s.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
            s.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text);
            s.visuals.widgets.hovered.corner_radius = 7.into();
            s.visuals.widgets.active.bg_fill = p.soft;
            s.visuals.widgets.active.weak_bg_fill = p.soft;
            s.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.text);
            s.visuals.widgets.active.corner_radius = 7.into();
        });
    }
}

pub fn primary(ui: &mut egui::Ui, label: &str, enabled: bool) -> Response {
    let p = Palette::get(ui.ctx());
    ui.add_enabled(
        enabled,
        egui::Button::new(egui::RichText::new(label).color(p.accent_text))
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .min_size(Vec2::new(72.0, 36.0)),
    )
}

pub fn toggle(ui: &mut egui::Ui, label: &str, hint: Option<&str>, checked: &mut bool) -> Response {
    let p = Palette::get(ui.ctx());
    let height = if hint.is_some() { 46.0 } else { 36.0 };
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *checked, label)
    });
    let text_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x - 48.0, rect.max.y));
    let label_galley = ui.painter().layout(
        label.into(),
        FontId::proportional(14.0),
        p.text,
        text_rect.width(),
    );
    ui.painter().galley(
        Pos2::new(
            text_rect.min.x,
            text_rect.min.y + if hint.is_some() { 2.0 } else { 8.0 },
        ),
        label_galley,
        p.text,
    );
    if let Some(hint) = hint {
        let galley = ui.painter().layout(
            hint.into(),
            FontId::proportional(12.0),
            p.muted,
            text_rect.width(),
        );
        ui.painter()
            .galley(text_rect.min + Vec2::new(0.0, 24.0), galley, p.muted);
    }
    let track = Rect::from_center_size(
        Pos2::new(rect.right() - 18.0, rect.center().y),
        Vec2::new(36.0, 21.0),
    );
    ui.painter().rect(
        track,
        11,
        if *checked { p.accent } else { p.inset },
        Stroke::new(1.0_f32, if *checked { p.accent } else { p.muted }),
        StrokeKind::Inside,
    );
    let x = if *checked {
        track.right() - 10.5
    } else {
        track.left() + 10.5
    };
    ui.painter().circle_filled(
        Pos2::new(x, track.center().y),
        7.0,
        if *checked { p.accent_text } else { p.panel },
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            7,
            Stroke::new(1.0_f32, p.accent),
            StrokeKind::Outside,
        );
    }
    response
}

pub fn fov_choice(
    ui: &mut egui::Ui,
    width: f32,
    degrees: i64,
    label: &str,
    selected: bool,
    enabled: bool,
) -> Response {
    let p = Palette::get(ui.ctx());
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .min_size(Vec2::new(width, 64.0))
            .fill(if selected { p.soft } else { p.background })
            .stroke(Stroke::new(
                1.0_f32,
                if selected { p.accent } else { p.line },
            )),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            enabled,
            selected,
            format!("{degrees} graus, {label}"),
        )
    });
    ui.painter().text(
        response.rect.center() - Vec2::new(0.0, 10.0),
        egui::Align2::CENTER_CENTER,
        format!("{degrees}°"),
        FontId::proportional(17.0),
        if selected { p.accent } else { p.text },
    );
    ui.painter().text(
        response.rect.center() + Vec2::new(0.0, 13.0),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        p.muted,
    );
    response
}

pub fn settings_icon(ui: &mut egui::Ui) -> Response {
    let p = Palette::get(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Preferências")
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 7, p.inset);
    }
    for (dy, knob) in [(-5.0, -3.0), (5.0, 4.0)] {
        let y = rect.center().y + dy;
        ui.painter().line_segment(
            [
                Pos2::new(rect.center().x - 9.0, y),
                Pos2::new(rect.center().x + 9.0, y),
            ],
            Stroke::new(1.4_f32, p.muted),
        );
        ui.painter().circle(
            Pos2::new(rect.center().x + knob, y),
            3.0,
            p.panel,
            Stroke::new(1.4_f32, p.muted),
        );
    }
    response.on_hover_text("Preferências")
}

pub fn brand(ui: &mut egui::Ui) {
    let p = Palette::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
    ui.painter()
        .circle_stroke(rect.center(), 9.0, Stroke::new(1.4_f32, p.accent));
    ui.painter()
        .circle_stroke(rect.center(), 4.0, Stroke::new(1.4_f32, p.accent));
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::TAU / 6.0;
        let a = Vec2::angled(angle);
        let b = Vec2::angled(angle + 0.5);
        ui.painter().line_segment(
            [rect.center() + a * 4.0, rect.center() + b * 9.0],
            Stroke::new(1.0_f32, p.accent),
        );
    }
    ui.label(egui::RichText::new("C200 Studio").size(17.0).strong());
}

pub fn joystick(ui: &mut egui::Ui, enabled: bool) -> (Response, [f32; 2]) {
    let p = Palette::get(ui.ctx());
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            ui.allocate_exact_size(Vec2::splat(112.0), Sense::click_and_drag())
                .1
        })
        .inner;
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            enabled,
            "Joystick de enquadramento. Use as setas do teclado; Shift para precisão.",
        )
    });
    let rect = response.rect;
    let held = enabled
        && ui.input(|i| i.focused && i.pointer.primary_down())
        && response.is_pointer_button_down_on();
    let offset = if held {
        response
            .interact_pointer_pos()
            .map(|pos| pos - rect.center())
            .unwrap_or_default()
    } else {
        Vec2::ZERO
    };
    let vector = crate::interaction::stick_direction(offset.x / 34.0, offset.y / 34.0);
    let nub = if held {
        offset * (30.0 / offset.length().max(30.0))
    } else {
        Vec2::ZERO
    };
    let stroke = if response.hovered() || response.has_focus() {
        p.accent
    } else {
        p.line
    };
    ui.painter().circle(
        rect.center(),
        55.0,
        if held { p.soft } else { p.background },
        Stroke::new(1.0_f32, stroke),
    );
    ui.painter()
        .circle_stroke(rect.center(), 31.0, Stroke::new(1.0_f32, p.line));
    for axis in [Vec2::X, Vec2::Y, -Vec2::X, -Vec2::Y] {
        let end = rect.center() + axis * 46.0;
        let cross = axis.rot90() * 3.0;
        ui.painter().line_segment(
            [end - axis * 3.0 + cross, end],
            Stroke::new(1.3_f32, p.muted),
        );
        ui.painter().line_segment(
            [end - axis * 3.0 - cross, end],
            Stroke::new(1.3_f32, p.muted),
        );
    }
    ui.painter().circle(
        rect.center() + nub,
        16.0,
        p.accent,
        Stroke::new(5.0_f32, p.panel),
    );
    if response.clicked() || held {
        response.request_focus();
    }
    (
        response
            .on_hover_cursor(if held {
                egui::CursorIcon::Grabbing
            } else if enabled {
                egui::CursorIcon::Grab
            } else {
                egui::CursorIcon::Default
            })
            .on_hover_text(
                "Arraste e segure para mover. Solte para parar. Setas do teclado também funcionam.",
            ),
        if held { vector } else { [0.0, 0.0] },
    )
}
