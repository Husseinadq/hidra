//! Top-down drawing of the G502 X with clickable hotspots, on the same
//! 300×440 grid as the design mockup.

use std::sync::OnceLock;

use eframe::egui::{
    epaint::{CubicBezierShape, Mesh, PathShape},
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
};

use super::theme::{self, ACCENT, ACCENT_INK, TEXT};
use crate::catalog::CONTROLS;

const W: f32 = 300.0;
const H: f32 = 440.0;
const BODY: Color32 = Color32::from_rgb(0x1E, 0x22, 0x28);
const EDGE: Color32 = Color32::from_rgb(0x3A, 0x41, 0x4B);
const PART: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x37);
const PART_EDGE: Color32 = Color32::from_rgb(0x4A, 0x52, 0x5E);
const DETAIL: Color32 = Color32::from_rgb(0x2B, 0x30, 0x38);

struct Outline {
    points: Vec<Pos2>,
    triangles: Vec<[u32; 3]>,
}

fn outline() -> &'static Outline {
    static O: OnceLock<Outline> = OnceLock::new();
    O.get_or_init(|| {
        let mut pts = vec![pos2(150.0, 22.0)];
        let cubic = |pts: &mut Vec<Pos2>, c1: (f32, f32), c2: (f32, f32), end: (f32, f32)| {
            let start = *pts.last().unwrap();
            let shape = CubicBezierShape::from_points_stroke([start, pos2(c1.0, c1.1), pos2(c2.0, c2.1), pos2(end.0, end.1)], false, Color32::TRANSPARENT, Stroke::NONE);
            pts.extend(shape.flatten(Some(0.4)).into_iter().skip(1));
        };
        cubic(&mut pts, (200.0, 22.0), (236.0, 40.0), (252.0, 80.0));
        pts.push(pos2(262.0, 150.0));
        cubic(&mut pts, (270.0, 210.0), (268.0, 290.0), (250.0, 350.0));
        cubic(&mut pts, (236.0, 398.0), (196.0, 424.0), (150.0, 424.0));
        cubic(&mut pts, (104.0, 424.0), (66.0, 404.0), (50.0, 360.0));
        pts.push(pos2(40.0, 300.0));
        cubic(&mut pts, (30.0, 296.0), (18.0, 280.0), (16.0, 250.0));
        pts.push(pos2(16.0, 200.0));
        cubic(&mut pts, (16.0, 178.0), (26.0, 164.0), (40.0, 160.0));
        pts.push(pos2(46.0, 110.0));
        cubic(&mut pts, (52.0, 60.0), (92.0, 22.0), (150.0, 22.0));
        pts.pop(); // closing point duplicates the first
        pts.dedup_by(|a, b| (*a - *b).length() < 0.05);
        let triangles = triangulate(&pts);
        Outline { points: pts, triangles }
    })
}

/// Ear clipping for a simple polygon (the body outline is not convex).
fn triangulate(pts: &[Pos2]) -> Vec<[u32; 3]> {
    let area: f32 = pts.iter().zip(pts.iter().cycle().skip(1)).map(|(a, b)| a.x * b.y - b.x * a.y).sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..pts.len()).collect();
    let mut out = Vec::new();
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guard = 0;
    while idx.len() > 3 && guard < 100_000 {
        guard += 1;
        let n = idx.len();
        let mut clipped = false;
        for i in 0..n {
            let (ia, ib, ic) = (idx[(i + n - 1) % n], idx[i], idx[(i + 1) % n]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            let cr = cross(a, b, c);
            if (cr > 0.0) != ccw || cr.abs() < 1e-6 {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let p = pts[j];
                let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
                if ccw {
                    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
                } else {
                    d1 <= 0.0 && d2 <= 0.0 && d3 <= 0.0
                }
            });
            if inside {
                continue;
            }
            out.push([ia as u32, ib as u32, ic as u32]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0] as u32, idx[1] as u32, idx[2] as u32]);
    }
    out
}

/// Draws the mouse and returns the index of a clicked hotspot.
pub fn show(ui: &mut Ui, selected: usize) -> Option<usize> {
    let scale = (ui.available_width() / W).min(1.0);
    let (rect, _) = ui.allocate_exact_size(vec2(W * scale, H * scale), Sense::hover());
    let at = |x: f32, y: f32| rect.min + vec2(x, y) * scale;
    let p = ui.painter();

    let o = outline();
    let mut mesh = Mesh::default();
    for pt in &o.points {
        mesh.colored_vertex(at(pt.x, pt.y), BODY);
    }
    for t in &o.triangles {
        mesh.add_triangle(t[0], t[1], t[2]);
    }
    p.add(Shape::mesh(mesh));
    let scaled: Vec<Pos2> = o.points.iter().map(|q| at(q.x, q.y)).collect();
    p.add(PathShape::closed_line(scaled, Stroke::new(2.0, EDGE)));

    let st = |c| Stroke::new(2.0 * scale.max(0.6), c);
    // button split and lower edge of the main buttons
    p.line_segment([at(150.0, 24.0), at(150.0, 58.0)], st(EDGE));
    p.line_segment([at(150.0, 124.0), at(150.0, 134.0)], st(EDGE));
    p.line_segment([at(150.0, 150.0), at(150.0, 164.0)], st(EDGE));
    let curve = CubicBezierShape::from_points_stroke([at(44.0, 162.0), at(100.0, 180.0), at(210.0, 180.0), at(262.0, 158.0)], false, Color32::TRANSPARENT, st(EDGE));
    p.add(curve);
    // angular body details
    p.line(vec![at(84.0, 200.0), at(118.0, 300.0), at(150.0, 330.0), at(182.0, 300.0), at(216.0, 200.0)], st(DETAIL));
    p.line_segment([at(60.0, 340.0), at(100.0, 392.0)], st(DETAIL));
    p.line_segment([at(240.0, 340.0), at(200.0, 392.0)], st(DETAIL));

    let part = |x: f32, y: f32, w: f32, h: f32, r: f32| {
        let rr = Rect::from_min_size(at(x, y), vec2(w, h) * scale);
        p.rect(rr, CornerRadius::same((r * scale) as u8), PART, Stroke::new(1.5, PART_EDGE), StrokeKind::Middle);
    };
    part(58.0, 76.0, 18.0, 28.0, 5.0);
    part(58.0, 114.0, 18.0, 28.0, 5.0);
    part(143.0, 58.0, 24.0, 66.0, 12.0);
    for y in [72.0, 82.0, 92.0, 102.0, 112.0] {
        p.line_segment([at(147.0, y), at(163.0, y)], st(Color32::from_rgb(0x3B, 0x42, 0x4C)));
    }
    part(147.0, 134.0, 16.0, 16.0, 4.0);
    part(18.0, 168.0, 14.0, 26.0, 5.0);
    part(16.0, 206.0, 12.0, 30.0, 5.0);
    part(16.0, 244.0, 12.0, 30.0, 5.0);

    let mut clicked = None;
    for (i, c) in CONTROLS.iter().enumerate() {
        let centre = at(c.at.0, c.at.1);
        let r = 15.0;
        let resp = ui
            .interact(Rect::from_center_size(centre, vec2(2.0 * r, 2.0 * r)), ui.id().with(("hotspot", i)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(c.name);
        let on = i == selected;
        let p = ui.painter();
        if on {
            p.circle_filled(centre, r + 5.0, theme::accent_soft());
        }
        let (fill, edge, fg) = if on {
            (ACCENT, ACCENT, ACCENT_INK)
        } else if resp.hovered() {
            (Color32::from_rgb(0x1F, 0x23, 0x2A), Color32::from_rgb(0x9A, 0xA2, 0xAE), TEXT)
        } else {
            (Color32::from_rgb(0x0F, 0x11, 0x14), Color32::from_rgb(0x5A, 0x62, 0x6E), TEXT)
        };
        p.circle(centre, r, fill, Stroke::new(2.0, edge));
        p.text(centre, Align2::CENTER_CENTER, c.tag, theme::mono(10.5), fg);
        if resp.clicked() {
            clicked = Some(i);
        }
    }
    clicked
}
