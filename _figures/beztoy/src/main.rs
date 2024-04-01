// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! An interactive toy for experimenting with rendering of Bézier paths,
//! including Euler spiral based stroke expansion.

mod arc;
mod euler;
mod flatten;

use xilem_web::{
    document_body,
    elements::svg::{g, svg},
    interfaces::*,
    svg::{
        kurbo::{Arc, BezPath, Circle, CubicBez, Line, PathEl, Point, Shape},
        peniko::Color,
    },
    App, PointerMsg, View,
};

use crate::{
    arc::{espc_to_arcs, euler_to_arcs},
    euler::{CubicParams, CubicToEulerIter},
    flatten::flatten_offset,
};

#[derive(Default)]
struct AppState {
    p0: Point,
    p1: Point,
    p2: Point,
    p3: Point,
    grab: GrabState,
}

#[derive(Default)]
struct GrabState {
    is_down: bool,
    id: i32,
    dx: f64,
    dy: f64,
}

impl GrabState {
    fn handle(&mut self, pt: &mut Point, p: &PointerMsg) {
        match p {
            PointerMsg::Down(e) => {
                if e.button == 0 {
                    self.dx = pt.x - e.x;
                    self.dy = pt.y - e.y;
                    self.id = e.id;
                    self.is_down = true;
                }
            }
            PointerMsg::Move(e) => {
                if self.is_down && self.id == e.id {
                    pt.x = self.dx + e.x;
                    pt.y = self.dy + e.y;
                }
            }
            PointerMsg::Up(e) => {
                if self.id == e.id {
                    self.is_down = false;
                }
            }
        }
    }
}

// https://iamkate.com/data/12-bit-rainbow/
const RAINBOW_PALETTE: [Color; 12] = [
    Color::rgb8(0x88, 0x11, 0x66),
    Color::rgb8(0xaa, 0x33, 0x55),
    Color::rgb8(0xcc, 0x66, 0x66),
    Color::rgb8(0xee, 0x99, 0x44),
    Color::rgb8(0xee, 0xdd, 0x00),
    Color::rgb8(0x99, 0xdd, 0x55),
    Color::rgb8(0x44, 0xdd, 0x88),
    Color::rgb8(0x22, 0xcc, 0xbb),
    Color::rgb8(0x00, 0xbb, 0xcc),
    Color::rgb8(0x00, 0x99, 0xcc),
    Color::rgb8(0x33, 0x66, 0xbb),
    Color::rgb8(0x66, 0x33, 0x99),
];

fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    let r = (a.r as f64 + (b.r as f64 - a.r as f64) * t) * (1. / 255.);
    let g = (a.g as f64 + (b.g as f64 - a.g as f64) * t) * (1. / 255.);
    let b = (a.b as f64 + (b.b as f64 - a.b as f64) * t) * (1. / 255.);
    Color::rgb(r, g, b)
}

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    let mut path = BezPath::new();
    path.move_to(state.p0);
    path.curve_to(state.p1, state.p2, state.p3);
    let stroke = xilem_web::svg::kurbo::Stroke::new(2.0);
    let stroke_thick = xilem_web::svg::kurbo::Stroke::new(15.0);
    let stroke_thin = xilem_web::svg::kurbo::Stroke::new(1.0);
    const NONE: Color = Color::TRANSPARENT;
    const HANDLE_RADIUS: f64 = 4.0;
    let c = CubicBez::new(state.p0, state.p1, state.p2, state.p3);
    let params = CubicParams::from_cubic(c);
    let err = params.est_euler_err();
    let mut spirals = vec![];
    const TOL: f64 = 1.0;
    for (i, es) in CubicToEulerIter::new(c, TOL).enumerate() {
        let path = if es.params.cubic_ok() {
            es.to_cubic().into_path(1.0)
        } else {
            // Janky rendering, we should be more sophisticated
            // and subdivide into cubics with appropriate bounds
            let mut path = BezPath::new();
            const N: usize = 20;
            path.move_to(es.p0);
            for i in 1..N {
                let t = i as f64 / N as f64;
                path.line_to(es.eval(t));
            }
            path.line_to(es.p1);
            path
        };
        let color = RAINBOW_PALETTE[(i * 7) % 12];
        let color = lerp_color(color, Color::WHITE, 0.5);
        spirals.push(path.stroke(color, stroke_thick.clone()).fill(NONE));
    }
    let offset = 100.0;
    let flat_ref = flatten_offset(CubicToEulerIter::new(c, 0.01), offset, 0.01);
    let mut flat_pts = vec![];
    let mut flat = BezPath::new();
    web_sys::console::log_1(&"---".into());
    for es in CubicToEulerIter::new(c, TOL) {
        if flat.is_empty() {
            flat.move_to(es.eval_with_offset(0.0, offset));
        }
        for arc in espc_to_arcs(&es, offset, TOL) {
            let circle = Circle::new(arc.to, 2.0).fill(Color::BLACK);
            flat_pts.push(circle);
            if let Some(arc) = Arc::from_svg_arc(&arc) {
                flat.extend(arc.append_iter(0.1));
            } else {
                web_sys::console::log_1(&format!("conversion failed {arc:?}").into());
            }
        }
    }
    svg(g((
        g(spirals),
        path.stroke(Color::BLACK, stroke_thin.clone()).fill(NONE),
        flat_ref
            .stroke(Color::BLACK, stroke_thin.clone())
            .fill(NONE),
        flat.stroke(Color::RED, stroke_thin.clone()).fill(NONE),
        g(flat_pts),
        Line::new(state.p0, state.p1).stroke(Color::BLUE, stroke.clone()),
        Line::new(state.p2, state.p3).stroke(Color::BLUE, stroke.clone()),
        Line::new((790., 300.), (790., 300. - 1000. * err)).stroke(Color::RED, stroke.clone()),
        g((
            Circle::new(state.p0, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p0, &msg)),
            Circle::new(state.p1, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p1, &msg)),
            Circle::new(state.p2, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p2, &msg)),
            Circle::new(state.p3, HANDLE_RADIUS)
                .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p3, &msg)),
        )),
    )))
    .attr("width", 800)
    .attr("height", 600)
}

pub fn main() {
    console_error_panic_hook::set_once();
    let mut state = AppState::default();
    state.p0 = Point::new(100.0, 100.0);
    state.p1 = Point::new(300.0, 150.0);
    state.p2 = Point::new(500.0, 150.0);
    state.p3 = Point::new(700.0, 150.0);
    let app = App::new(state, app_logic);
    app.run(&document_body());
}
