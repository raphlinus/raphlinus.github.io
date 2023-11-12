// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! An interactive toy for experimenting with rendering of Bézier paths,
//! including Euler spiral based stroke expansion.

mod euler;
mod flatten;

use xilem_svg::{
    group,
    kurbo::{BezPath, Circle, CubicBez, Line, Point, Shape},
    peniko::Color,
    App, PointerMsg, View, ViewExt,
};

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
    let stroke = xilem_svg::kurbo::Stroke::new(2.0);
    let stroke_thick = xilem_svg::kurbo::Stroke::new(8.0);
    let stroke_thin = xilem_svg::kurbo::Stroke::new(1.0);
    const NONE: Color = Color::rgba8(0, 0, 0, 0);
    const HANDLE_RADIUS: f64 = 4.0;
    let c = CubicBez::new(state.p0, state.p1, state.p2, state.p3);
    let params = CubicParams::from_cubic(c);
    let err = params.est_euler_err();
    let mut spirals = vec![];
    for (i, es) in CubicToEulerIter::new(c, 1.0).enumerate() {
        for i in 0..10 {
            let t = i as f64 * 0.1;
            es.params.eval_th(t);
        }
        let path = es.to_cubic().into_path(1.0);
        let color = RAINBOW_PALETTE[(i * 7) % 12];
        spirals.push(path.stroke(color, stroke_thick.clone()));
    }
    let offset = 40.0;
    let flat = flatten_offset(CubicToEulerIter::new(c, 1.0), offset);
    group((
        group(spirals).fill(NONE),
        path.stroke(Color::BLACK, stroke_thin.clone()).fill(NONE),
        flat.stroke(Color::BLUE, stroke_thin).fill(NONE),
        Line::new(state.p0, state.p1)
            .stroke(Color::BLUE, stroke.clone())
            .fill(NONE),
        Line::new(state.p2, state.p3)
            .stroke(Color::BLUE, stroke.clone())
            .fill(NONE),
        Line::new((790., 300.), (790., 300. - 1000. * err))
            .stroke(Color::RED, stroke.clone())
            .fill(NONE),
        Circle::new(state.p0, HANDLE_RADIUS)
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p0, &msg)),
        Circle::new(state.p1, HANDLE_RADIUS)
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p1, &msg)),
        Circle::new(state.p2, HANDLE_RADIUS)
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p2, &msg)),
        Circle::new(state.p3, HANDLE_RADIUS)
            .pointer(|s: &mut AppState, msg| s.grab.handle(&mut s.p3, &msg)),
    ))
}

pub fn main() {
    console_error_panic_hook::set_once();
    let mut state = AppState::default();
    state.p0 = Point::new(100.0, 100.0);
    state.p1 = Point::new(300.0, 150.0);
    state.p2 = Point::new(500.0, 150.0);
    state.p3 = Point::new(700.0, 150.0);
    let app = App::new(state, app_logic);
    app.run();
}
