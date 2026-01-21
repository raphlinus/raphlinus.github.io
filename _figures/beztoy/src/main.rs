// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! An interactive toy for experimenting with rendering of Bézier paths,
//! including Euler spiral based stroke expansion.

mod euler;
mod flatten;

use xilem_web::{svg::{kurbo::{Point, BezPath, CubicBez, PathEl, Circle, Line, Shape}, peniko::Color}, PointerMsg, View, App, elements::svg::{g, svg}, document_body, interfaces::*};

use crate::{
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

fn app_logic(state: &mut AppState) -> impl View<AppState> {
    let mut path = BezPath::new();
    path.move_to(state.p0);
    path.curve_to(state.p1, state.p2, state.p3);
    let stroke = xilem_web::svg::kurbo::Stroke::new(2.0);
    let stroke_thick = xilem_web::svg::kurbo::Stroke::new(8.0);
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
        spirals.push(path.stroke(color, stroke_thick.clone()).fill(NONE));
    }
    let offset = 40.0;
    let flat = flatten_offset(CubicToEulerIter::new(c, TOL), offset);
    let flat2 = flatten_offset(CubicToEulerIter::new(c, TOL), -offset);
    let mut flat_pts = vec![];
    for seg in flat.elements().iter().chain(flat2.elements().iter()) {
        match seg {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                let circle = Circle::new(*p, 2.0).fill(Color::BLACK);
                flat_pts.push(circle);
            }
            _ => (),
        }
    }
    svg(g((
        g(spirals),
        path.stroke(Color::BLACK, stroke_thin.clone()).fill(NONE),
        flat.stroke(Color::BLUE, stroke_thin.clone()).fill(NONE),
        flat2.stroke(Color::PURPLE, stroke_thin).fill(NONE),
        g(flat_pts),
        Line::new(state.p0, state.p1)
            .stroke(Color::BLUE, stroke.clone()),
        Line::new(state.p2, state.p3)
            .stroke(Color::BLUE, stroke.clone()),
        Line::new((790., 300.), (790., 300. - 1000. * err))
            .stroke(Color::RED, stroke.clone()),
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
