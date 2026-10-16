//! Independently drawn Oteryn pointer; operating-system cursor remains selectable.

use egui::{Color32, CursorIcon, Id, LayerId, Order, Stroke};
use oteryn_client::settings::ClientSettings;

pub fn show(context: &egui::Context, settings: &ClientSettings) {
    // Unknown preferences retain the existing system pointer. Text, resize and
    // other specialized system cursors keep their semantics; this replaces the
    // ordinary arrow, not the unimplemented gameplay-context cursor family.
    if settings.stored_bool("interface.interface.system_cursor") != Some(false)
        || context.output(|output| output.cursor_icon) != CursorIcon::Default
        || !context.input(|input| input.focused)
    {
        return;
    }
    let Some(position) = context.input(|input| input.pointer.hover_pos()) else {
        return;
    };
    if !context.content_rect().contains(position) {
        return;
    }
    let scale = if settings.stored_bool("interface.interface.big_cursor") == Some(true) {
        2.0
    } else {
        1.0
    };
    let points = [
        [0.0, 0.0],
        [0.0, 23.0],
        [6.0, 17.0],
        [10.0, 27.0],
        [14.0, 25.0],
        [10.0, 15.0],
        [19.0, 15.0],
    ]
    .map(|[x, y]| position + egui::vec2(x, y) * scale);
    // The arrow is concave: triangulate it explicitly rather than asking the
    // convex-polygon painter to fill a shape outside its contract.
    let mut mesh = egui::Mesh::default();
    for point in points {
        mesh.colored_vertex(point, Color32::from_rgb(238, 207, 139));
    }
    for triangle in [[0, 1, 2], [0, 2, 5], [0, 5, 6], [2, 3, 4], [2, 4, 5]] {
        mesh.add_triangle(triangle[0], triangle[1], triangle[2]);
    }
    let painter = context.layer_painter(LayerId::new(Order::Tooltip, Id::new("oteryn-pointer")));
    painter.add(mesh);
    painter.add(egui::Shape::closed_line(
        points.to_vec(),
        Stroke::new(scale, Color32::BLACK),
    ));
    context.set_cursor_icon(CursorIcon::None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client::settings_catalog::FutureValue;

    fn render(settings: &ClientSettings, events: Vec<egui::Event>) -> egui::FullOutput {
        let context = egui::Context::default();
        context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(200.0, 100.0),
                )),
                events,
                focused: true,
                ..Default::default()
            },
            |ui| show(ui.ctx(), settings),
        )
    }

    #[test]
    fn native_custom_and_large_pointers_have_real_output_without_changing_settings()
    -> Result<(), Box<dyn std::error::Error>> {
        let pointer = vec![egui::Event::PointerMoved(egui::pos2(12.0, 34.0))];
        let mut settings = ClientSettings::default();
        assert_eq!(
            render(&settings, pointer.clone())
                .platform_output
                .cursor_icon,
            CursorIcon::Default
        );
        settings.future_preferences.insert(
            "interface.interface.system_cursor".into(),
            FutureValue::Bool(false),
        );
        let before = serde_json::to_value(&settings)?;
        let small = render(&settings, pointer.clone());
        assert_eq!(small.platform_output.cursor_icon, CursorIcon::None);
        let arrow = small
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.vertices.len() == 7 => Some(mesh.calc_bounds()),
                _ => None,
            })
            .ok_or("drawn custom arrow")?;
        assert_eq!(arrow.min, egui::pos2(12.0, 34.0));
        assert_eq!(serde_json::to_value(&settings)?, before);
        settings.future_preferences.insert(
            "interface.interface.big_cursor".into(),
            FutureValue::Bool(true),
        );
        let large = render(&settings, pointer.clone());
        let large_arrow = large
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.vertices.len() == 7 => Some(mesh.calc_bounds()),
                _ => None,
            })
            .ok_or("drawn large arrow")?;
        assert_eq!(large_arrow.size(), arrow.size() * 2.0);
        assert_eq!(large_arrow.min, arrow.min);
        assert_eq!(
            render(&settings, vec![egui::Event::PointerGone])
                .platform_output
                .cursor_icon,
            CursorIcon::Default
        );
        settings.future_preferences.insert(
            "interface.interface.system_cursor".into(),
            FutureValue::Bool(true),
        );
        assert_eq!(
            render(&settings, pointer).platform_output.cursor_icon,
            CursorIcon::Default
        );
        Ok(())
    }
}
