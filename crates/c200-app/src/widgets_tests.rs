use super::*;

fn draw(ctx: &egui::Context, events: Vec<egui::Event>, focused: bool) -> (Rect, [f32; 2]) {
    let mut result = (Rect::NOTHING, [0.0, 0.0]);
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(240.0))),
            events,
            focused,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (response, vector) = joystick(ui, true);
                result = (response.rect, vector);
            });
        },
    );
    result
}

#[test]
fn joystick_stops_on_release_and_window_focus_loss() {
    let ctx = egui::Context::default();
    let (rect, _) = draw(&ctx, vec![], true);
    let center = rect.center();
    let press = egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    };
    draw(&ctx, vec![egui::Event::PointerMoved(center), press], true);
    let moved = center + Vec2::new(26.0, -26.0);
    let (_, direction) = draw(&ctx, vec![egui::Event::PointerMoved(moved)], true);
    assert!(direction[0] > 0.0 && direction[1] < 0.0);
    assert_eq!(draw(&ctx, vec![], false).1, [0.0, 0.0]);
    let release = egui::Event::PointerButton {
        pos: moved,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    };
    assert_eq!(draw(&ctx, vec![release], true).1, [0.0, 0.0]);
}
