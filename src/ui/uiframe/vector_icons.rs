//! Vector icons drawn with egui shapes (no font glyphs).

use egui::{Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Hamburger,
    Settings,
    Close,
    Minimize,
    /// Maximize workspace pane to fill the workspace.
    Maximize,
    /// Restore from pane fullscreen.
    Restore,
    NewWindow,
    Back,
    Plus,
    Keyboard,
    FontSmaller,
    FontLarger,
    /// Active sessions (stacked rows).
    Sessions,
    /// Saved connections (card / plug).
    Connections,
    /// Folder for file browser.
    Folder,
    /// Performance monitor (sparkline glyph).
    Chart,
    /// Favorite commands (prompt chevron).
    Commands,
}

pub fn paint(ui: &Ui, rect: Rect, icon: Icon, color: Color32, stroke: f32) {
    let shapes = shapes_for_icon(icon, rect, color, stroke);
    ui.painter_at(rect).add(shapes);
}

pub fn button(ui: &mut Ui, rect: Rect, id: Id, icon: Icon, stroke: f32) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if resp.hovered() {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().weak_text_color()
        };
        paint(ui, rect, icon, color, stroke);
    }
    resp
}

pub fn icon_color(ui: &Ui, resp: &Response) -> Color32 {
    if resp.hovered() {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().weak_text_color()
    }
}

fn shapes_for_icon(icon: Icon, rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    match icon {
        Icon::Hamburger => hamburger(rect, color, stroke),
        Icon::Settings => settings(rect, color, stroke),
        Icon::Close => close(rect, color, stroke),
        Icon::Minimize => minimize(rect, color, stroke),
        Icon::Maximize => maximize(rect, color, stroke),
        Icon::Restore => restore(rect, color, stroke),
        Icon::NewWindow => new_window(rect, color, stroke),
        Icon::Back => back(rect, color, stroke),
        Icon::Plus => plus(rect, color, stroke),
        Icon::Keyboard => keyboard(rect, color, stroke),
        Icon::FontSmaller => font_smaller(rect, color, stroke),
        Icon::FontLarger => font_larger(rect, color, stroke),
        Icon::Sessions => sessions(rect, color, stroke),
        Icon::Connections => connections(rect, color, stroke),
        Icon::Folder => folder(rect, color, stroke),
        Icon::Chart => chart(rect, color, stroke),
        Icon::Commands => commands_icon(rect, color, stroke),
    }
}

fn map_pt(rect: Rect, x: f32, y: f32) -> Pos2 {
    Pos2::new(
        rect.left() + rect.width() * x,
        rect.top() + rect.height() * y,
    )
}

fn line(rect: Rect, color: Color32, stroke: f32, a: (f32, f32), b: (f32, f32)) -> Shape {
    Shape::line_segment(
        [map_pt(rect, a.0, a.1), map_pt(rect, b.0, b.1)],
        Stroke::new(stroke, color),
    )
}

fn hamburger(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    vec![
        line(rect, color, stroke, (0.18, 0.28), (0.82, 0.28)),
        line(rect, color, stroke, (0.18, 0.50), (0.82, 0.50)),
        line(rect, color, stroke, (0.18, 0.72), (0.82, 0.72)),
    ]
}

fn settings(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.18;
    let mut shapes = vec![Shape::circle_stroke(c, r, Stroke::new(stroke, color))];
    for i in 0..8 {
        let a = std::f32::consts::TAU * i as f32 / 8.0;
        let dir = Vec2::angled(a);
        let inner = c + dir * r * 1.1;
        let outer = c + dir * r * 1.9;
        shapes.push(Shape::line_segment(
            [inner, outer],
            Stroke::new(stroke * 0.9, color),
        ));
    }
    shapes
}

fn close(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    vec![
        line(rect, color, stroke, (0.22, 0.22), (0.78, 0.78)),
        line(rect, color, stroke, (0.78, 0.22), (0.22, 0.78)),
    ]
}

fn minimize(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    vec![line(rect, color, stroke, (0.18, 0.62), (0.82, 0.62))]
}

fn maximize(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let r = egui::Rect::from_min_max(map_pt(rect, 0.22, 0.22), map_pt(rect, 0.78, 0.78));
    vec![Shape::rect_stroke(
        r,
        1.0,
        Stroke::new(stroke, color),
        egui::StrokeKind::Inside,
    )]
}

fn restore(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let back = egui::Rect::from_min_max(map_pt(rect, 0.30, 0.18), map_pt(rect, 0.82, 0.62));
    let front = egui::Rect::from_min_max(map_pt(rect, 0.18, 0.38), map_pt(rect, 0.70, 0.82));
    vec![
        Shape::rect_stroke(
            back,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
        Shape::rect_filled(front, 1.0, Color32::TRANSPARENT),
        Shape::rect_stroke(
            front,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
    ]
}

fn new_window(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let back = egui::Rect::from_min_max(map_pt(rect, 0.12, 0.38), map_pt(rect, 0.62, 0.88));
    let front = egui::Rect::from_min_max(map_pt(rect, 0.38, 0.12), map_pt(rect, 0.88, 0.62));
    vec![
        Shape::rect_stroke(
            back,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
        Shape::rect_stroke(
            front,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
    ]
}

fn back(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    vec![
        line(rect, color, stroke, (0.55, 0.18), (0.22, 0.50)),
        line(rect, color, stroke, (0.55, 0.82), (0.22, 0.50)),
        line(rect, color, stroke, (0.22, 0.50), (0.82, 0.50)),
    ]
}

fn plus(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    vec![
        line(rect, color, stroke, (0.50, 0.18), (0.50, 0.82)),
        line(rect, color, stroke, (0.18, 0.50), (0.82, 0.50)),
    ]
}

fn keyboard(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let body = egui::Rect::from_min_max(map_pt(rect, 0.14, 0.32), map_pt(rect, 0.86, 0.86));
    vec![
        Shape::rect_stroke(
            body,
            1.5,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
        line(rect, color, stroke * 0.85, (0.26, 0.52), (0.74, 0.52)),
        line(rect, color, stroke * 0.85, (0.26, 0.66), (0.74, 0.66)),
        line(rect, color, stroke * 0.85, (0.38, 0.76), (0.62, 0.76)),
    ]
}

fn font_smaller(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let mut shapes = vec![
        line(rect, color, stroke, (0.20, 0.72), (0.55, 0.72)),
        line(rect, color, stroke, (0.20, 0.72), (0.20, 0.28)),
        line(rect, color, stroke, (0.20, 0.28), (0.55, 0.28)),
    ];
    shapes.push(line(rect, color, stroke * 0.9, (0.68, 0.62), (0.82, 0.76)));
    shapes.push(line(rect, color, stroke * 0.9, (0.82, 0.62), (0.68, 0.76)));
    shapes
}

fn font_larger(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let mut shapes = vec![
        line(rect, color, stroke, (0.18, 0.72), (0.52, 0.72)),
        line(rect, color, stroke, (0.18, 0.72), (0.18, 0.28)),
        line(rect, color, stroke, (0.18, 0.28), (0.52, 0.28)),
    ];
    shapes.push(line(rect, color, stroke * 0.9, (0.66, 0.58), (0.84, 0.58)));
    shapes.push(line(rect, color, stroke * 0.9, (0.75, 0.50), (0.75, 0.66)));
    shapes
}

fn sessions(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    // Three horizontal rows suggesting an active session list.
    vec![
        line(rect, color, stroke, (0.20, 0.28), (0.80, 0.28)),
        line(rect, color, stroke, (0.20, 0.50), (0.80, 0.50)),
        line(rect, color, stroke, (0.20, 0.72), (0.62, 0.72)),
    ]
}

fn connections(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    // Two small cards + link between them.
    let a = egui::Rect::from_min_max(map_pt(rect, 0.14, 0.28), map_pt(rect, 0.42, 0.72));
    let b = egui::Rect::from_min_max(map_pt(rect, 0.58, 0.28), map_pt(rect, 0.86, 0.72));
    vec![
        Shape::rect_stroke(a, 1.0, Stroke::new(stroke, color), egui::StrokeKind::Inside),
        Shape::rect_stroke(b, 1.0, Stroke::new(stroke, color), egui::StrokeKind::Inside),
        line(rect, color, stroke * 0.9, (0.42, 0.50), (0.58, 0.50)),
    ]
}

fn folder(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    let tab = egui::Rect::from_min_max(map_pt(rect, 0.16, 0.28), map_pt(rect, 0.48, 0.42));
    let body = egui::Rect::from_min_max(map_pt(rect, 0.16, 0.40), map_pt(rect, 0.84, 0.78));
    vec![
        Shape::rect_stroke(
            tab,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
        Shape::rect_stroke(
            body,
            1.0,
            Stroke::new(stroke, color),
            egui::StrokeKind::Inside,
        ),
    ]
}

fn chart(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    // Axes + rising polyline.
    let mut shapes = vec![
        line(rect, color, stroke, (0.18, 0.78), (0.82, 0.78)),
        line(rect, color, stroke, (0.18, 0.22), (0.18, 0.78)),
    ];
    let pts = [
        (0.28, 0.62),
        (0.42, 0.48),
        (0.56, 0.55),
        (0.70, 0.34),
        (0.82, 0.28),
    ];
    for w in pts.windows(2) {
        shapes.push(line(rect, color, stroke, w[0], w[1]));
    }
    shapes
}

fn commands_icon(rect: Rect, color: Color32, stroke: f32) -> Vec<Shape> {
    // `>` chevron + underscore cursor.
    vec![
        line(rect, color, stroke, (0.22, 0.28), (0.48, 0.50)),
        line(rect, color, stroke, (0.48, 0.50), (0.22, 0.72)),
        line(rect, color, stroke, (0.52, 0.72), (0.82, 0.72)),
    ]
}
