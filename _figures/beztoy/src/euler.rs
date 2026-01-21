// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! Calculations and utilities for Euler spirals

use xilem_web::svg::kurbo::{CubicBez, Point, Vec2};

#[derive(Debug)]
pub struct CubicParams {
    pub th0: f64,
    pub th1: f64,
    pub d0: f64,
    pub d1: f64,
}

#[derive(Debug)]
pub struct EulerParams {
    pub th0: f64,
    pub th1: f64,
    pub k0: f64,
    pub k1: f64,
    pub ch: f64,
}

#[derive(Debug)]
pub struct EulerSeg {
    pub p0: Point,
    pub p1: Point,
    pub params: EulerParams,
}

pub struct CubicToEulerIter {
    c: CubicBez,
    tolerance: f64,
    // [t0 * dt .. (t0 + 1) * dt] is the range we're currently considering
    t0: u64,
    dt: f64,
    last_p: Vec2,
    last_q: Vec2,
    last_t: f64,
}

impl CubicParams {
    /// Compute parameters from endpoints and derivatives.
    pub fn from_points_derivs(p0: Vec2, p1: Vec2, q0: Vec2, q1: Vec2, dt: f64) -> Self {
        let chord = p1 - p0;
        // Robustness note: we must protect this function from being called when the
        // chord length is (near-)zero.
        let scale = dt / chord.length_squared();
        let h0 = Vec2::new(
            q0.x * chord.x + q0.y * chord.y,
            q0.y * chord.x - q0.x * chord.y,
        );
        let th0 = h0.atan2();
        let d0 = h0.length() * scale;
        let h1 = Vec2::new(
            q1.x * chord.x + q1.y * chord.y,
            q1.x * chord.y - q1.y * chord.x,
        );
        let th1 = h1.atan2();
        let d1 = h1.length() * scale;
        // Robustness note: we may want to clamp the magnitude of the angles to
        // a bit less than pi. Perhaps here, perhaps downstream.
        CubicParams { th0, th1, d0, d1 }
    }

    pub fn from_cubic(c: CubicBez) -> Self {
        let chord = c.p3 - c.p0;
        // TODO: if chord is 0, we have a problem
        let d01 = c.p1 - c.p0;
        let h0 = Vec2::new(
            d01.x * chord.x + d01.y * chord.y,
            d01.y * chord.x - d01.x * chord.y,
        );
        let th0 = h0.atan2();
        let d0 = h0.hypot() / chord.hypot2();
        let d23 = c.p3 - c.p2;
        let h1 = Vec2::new(
            d23.x * chord.x + d23.y * chord.y,
            d23.x * chord.y - d23.y * chord.x,
        );
        let th1 = h1.atan2();
        let d1 = h1.hypot() / chord.hypot2();
        CubicParams { th0, th1, d0, d1 }
    }

    // Estimated error of GH to Euler spiral
    //
    // Return value is normalized to chord - to get actual error, multiply
    // by chord.
    pub fn est_euler_err(&self) -> f64 {
        // Potential optimization: work with unit vector rather than angle
        let cth0 = self.th0.cos();
        let cth1 = self.th1.cos();
        if cth0 * cth1 < 0.0 {
            // Rationale: this happens when fitting a cusp or near-cusp with
            // a near 180 degree u-turn. The actual ES is bounded in that case.
            // Further subdivision won't reduce the angles if actually a cusp.
            return 2.0;
        }
        let e0 = (2. / 3.) / (1.0 + cth0);
        let e1 = (2. / 3.) / (1.0 + cth1);
        let s0 = self.th0.sin();
        let s1 = self.th1.sin();
        // Note: some other versions take sin of s0 + s1 instead. Those are incorrect.
        // Strangely, calibration is the same, but more work could be done.
        let s01 = cth0 * s1 + cth1 * s0;
        let amin = 0.15 * (2. * e0 * s0 + 2. * e1 * s1 - e0 * e1 * s01);
        let a = 0.15 * (2. * self.d0 * s0 + 2. * self.d1 * s1 - self.d0 * self.d1 * s01);
        let aerr = (a - amin).abs();
        let symm = (self.th0 + self.th1).abs();
        let asymm = (self.th0 - self.th1).abs();
        let dist = (self.d0 - e0).hypot(self.d1 - e1);
        let ctr = 3.7e-6 * symm.powi(5) + 6e-3 * asymm * symm.powi(2);
        let halo_symm = 5e-3 * symm * dist;
        let halo_asymm = 7e-2 * asymm * dist;
        1.25 * ctr + 1.55 * aerr + halo_symm + halo_asymm
    }
}

impl EulerParams {
    pub fn from_angles(th0: f64, th1: f64) -> EulerParams {
        let k0 = th0 + th1;
        let dth = th1 - th0;
        let d2 = dth * dth;
        let k2 = k0 * k0;
        let mut a = 6.0;
        a -= d2 * (1. / 70.);
        a -= (d2 * d2) * (1. / 10780.);
        a += (d2 * d2 * d2) * 2.769178184818219e-07;
        let b = -0.1 + d2 * (1. / 4200.) + d2 * d2 * 1.6959677820260655e-05;
        let c = -1. / 1400. + d2 * 6.84915970574303e-05 - k2 * 7.936475029053326e-06;
        a += (b + c * k2) * k2;
        let k1 = dth * a;

        // calculation of chord
        let mut ch = 1.0;
        ch -= d2 * (1. / 40.);
        ch += (d2 * d2) * 0.00034226190482569864;
        ch -= (d2 * d2 * d2) * 1.9349474568904524e-06;
        let b = -1. / 24. + d2 * 0.0024702380951963226 - d2 * d2 * 3.7297408997537985e-05;
        let c = 1. / 1920. - d2 * 4.87350869747975e-05 - k2 * 3.1001936068463107e-06;
        ch += (b + c * k2) * k2;
        EulerParams {
            th0,
            th1,
            k0,
            k1,
            ch,
        }
    }

    pub fn eval_th(&self, t: f64) -> f64 {
        (self.k0 + 0.5 * self.k1 * (t - 1.0)) * t - self.th0
    }

    /// Evaluate the curve at the given parameter.
    ///
    /// The parameter is in the range 0..1, and the result goes from (0, 0) to (1, 0).
    fn eval(&self, t: f64) -> Point {
        let thm = self.eval_th(t * 0.5);
        let k0 = self.k0;
        let k1 = self.k1;
        let (u, v) = integ_euler_10((k0 + k1 * (0.5 * t - 0.5)) * t, k1 * t * t);
        let s = t / self.ch * thm.sin();
        let c = t / self.ch * thm.cos();
        let x = u * c - v * s;
        let y = -v * c - u * s;
        Point::new(x, y)
    }

    fn eval_with_offset(&self, t: f64, offset: f64) -> Point {
        let th = self.eval_th(t);
        let v = Vec2::new(offset * th.sin(), offset * th.cos());
        self.eval(t) + v
    }

    // Determine whether a render as a single cubic will be adequate
    pub fn cubic_ok(&self) -> bool {
        self.th0.abs() < 1.0 && self.th1.abs() < 1.0
    }
}

impl EulerSeg {
    pub fn from_params(p0: Point, p1: Point, params: EulerParams) -> Self {
        EulerSeg { p0, p1, params }
    }

    /// Use two-parabola approximation.
    pub fn to_cubic(&self) -> CubicBez {
        let (s0, c0) = self.params.th0.sin_cos();
        let (s1, c1) = self.params.th1.sin_cos();
        let d0 = (2. / 3.) / (1.0 + c0);
        let d1 = (2. / 3.) / (1.0 + c1);
        let chord = self.p1 - self.p0;
        let p1 = self.p0 + d0 * Vec2::new(chord.x * c0 - chord.y * s0, chord.y * c0 + chord.x * s0);
        let p2 = self.p1 - d1 * Vec2::new(chord.x * c1 + chord.y * s1, chord.y * c1 - chord.x * s1);
        CubicBez::new(self.p0, p1, p2, self.p1)
    }

    #[allow(unused)]
    pub fn eval(&self, t: f64) -> Point {
        let Point { x, y } = self.params.eval(t);
        let chord = self.p1 - self.p0;
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }

    pub fn eval_with_offset(&self, t: f64, offset: f64) -> Point {
        let chord = self.p1 - self.p0;
        let scaled = offset / chord.hypot();
        let Point { x, y } = self.params.eval_with_offset(t, scaled);
        Point::new(
            self.p0.x + chord.x * x - chord.y * y,
            self.p0.y + chord.x * y + chord.y * x,
        )
    }
}

/// Evaluate both the point and derivative of a cubic bezier.
fn eval_cubic_and_deriv(c: &CubicBez, t: f64) -> (Vec2, Vec2) {
    let p0 = c.p0.to_vec2();
    let p1 = c.p1.to_vec2();
    let p2 = c.p2.to_vec2();
    let p3 = c.p3.to_vec2();
    let m = 1.0 - t;
    let mm = m * m;
    let mt = m * t;
    let tt = t * t;
    let p = p0 * (mm * m) + (p1 * (3.0 * mm) + p2 * (3.0 * mt) + p3 * tt) * t;
    let q = (p1 - p0) * mm + (p2 - p1) * (2.0 * mt) + (p3 - p2) * tt;
    (p, q)
}

impl Iterator for CubicToEulerIter {
    type Item = EulerSeg;

    fn next(&mut self) -> Option<EulerSeg> {
        let t0 = (self.t0 as f64) * self.dt;
        if t0 == 1.0 {
            return None;
        }
        loop {
            let mut t1 = t0 + self.dt;
            let p0 = self.last_p;
            let q0 = self.last_q;
            let (mut p1, mut q1) = eval_cubic_and_deriv(&self.c, t1);
            if q1.length_squared() < DERIV_THRESH.powi(2) {
                let (new_p1, new_q1) = eval_cubic_and_deriv(&self.c, t1 - DERIV_EPS);
                q1 = new_q1;
                if t1 < 1. {
                    p1 = new_p1;
                    t1 -= DERIV_EPS;
                }
            }
            // TODO: robustness
            let actual_dt = t1 - self.last_t;
            let cubic_params = CubicParams::from_points_derivs(p0, p1, q0, q1, actual_dt);
            let est_err: f64 = cubic_params.est_euler_err();
            let err = est_err * (p0 - p1).hypot();
            if err <= self.tolerance {
                self.t0 += 1;
                let shift = self.t0.trailing_zeros();
                self.t0 >>= shift;
                self.dt *= (1 << shift) as f64;
                let euler_params = EulerParams::from_angles(cubic_params.th0, cubic_params.th1);
                let es = EulerSeg::from_params(p0.to_point(), p1.to_point(), euler_params);
                self.last_p = p1;
                self.last_q = q1;
                self.last_t = t1;
                return Some(es);
            }
            self.t0 *= 2;
            self.dt *= 0.5;
        }
    }
}

/// Threshold below which a derivative is considered too small.
const DERIV_THRESH: f64 = 1e-6;
/// Amount to nudge t when derivative is near-zero.
const DERIV_EPS: f64 = 1e-6;

impl CubicToEulerIter {
    pub fn new(c: CubicBez, tolerance: f64) -> Self {
        let mut last_q = c.p1 - c.p0;
        // TODO: tweak
        if last_q.length_squared() < DERIV_THRESH.powi(2) {
            last_q = eval_cubic_and_deriv(&c, DERIV_EPS).1;
        }
        CubicToEulerIter {
            c,
            tolerance,
            t0: 0,
            dt: 1.0,
            last_p: c.p0.to_vec2(),
            last_q,
            last_t: 0.0,
        }
    }
}

/// Integrate Euler spiral.
///
/// TODO: investigate needed accuracy. We might be able to get away
/// with 8th order.
fn integ_euler_10(k0: f64, k1: f64) -> (f64, f64) {
    let t1_1 = k0;
    let t1_2 = 0.5 * k1;
    let t2_2 = t1_1 * t1_1;
    let t2_3 = 2. * (t1_1 * t1_2);
    let t2_4 = t1_2 * t1_2;
    let t3_4 = t2_2 * t1_2 + t2_3 * t1_1;
    let t3_6 = t2_4 * t1_2;
    let t4_4 = t2_2 * t2_2;
    let t4_5 = 2. * (t2_2 * t2_3);
    let t4_6 = 2. * (t2_2 * t2_4) + t2_3 * t2_3;
    let t4_7 = 2. * (t2_3 * t2_4);
    let t4_8 = t2_4 * t2_4;
    let t5_6 = t4_4 * t1_2 + t4_5 * t1_1;
    let t5_8 = t4_6 * t1_2 + t4_7 * t1_1;
    let t6_6 = t4_4 * t2_2;
    let t6_7 = t4_4 * t2_3 + t4_5 * t2_2;
    let t6_8 = t4_4 * t2_4 + t4_5 * t2_3 + t4_6 * t2_2;
    let t7_8 = t6_6 * t1_2 + t6_7 * t1_1;
    let t8_8 = t6_6 * t2_2;
    let mut u = 1.;
    u -= (1. / 24.) * t2_2 + (1. / 160.) * t2_4;
    u += (1. / 1920.) * t4_4 + (1. / 10752.) * t4_6 + (1. / 55296.) * t4_8;
    u -= (1. / 322560.) * t6_6 + (1. / 1658880.) * t6_8;
    u += (1. / 92897280.) * t8_8;
    let mut v = (1. / 12.) * t1_2;
    v -= (1. / 480.) * t3_4 + (1. / 2688.) * t3_6;
    v += (1. / 53760.) * t5_6 + (1. / 276480.) * t5_8;
    v -= (1. / 11612160.) * t7_8;
    (u, v)
}
