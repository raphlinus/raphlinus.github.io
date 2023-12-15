// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! Calculations and utilities for Euler spirals

use xilem_web::svg::kurbo::{CubicBez, ParamCurve, Point, Vec2};

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
}

impl CubicParams {
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
        let e0 = (2. / 3.) / (1.0 + self.th0.cos());
        let e1 = (2. / 3.) / (1.0 + self.th1.cos());
        let s0 = self.th0.sin();
        let s1 = self.th1.sin();
        let s01 = (s0 + s1).sin();
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

impl Iterator for CubicToEulerIter {
    type Item = EulerSeg;

    fn next(&mut self) -> Option<EulerSeg> {
        let t0 = (self.t0 as f64) * self.dt;
        if t0 == 1.0 {
            return None;
        }
        loop {
            let t1 = t0 + self.dt;
            let cubic = self.c.subsegment(t0..t1);
            let cubic_params = CubicParams::from_cubic(cubic);
            let est_err: f64 = cubic_params.est_euler_err();
            let err = est_err * cubic.p0.distance(cubic.p3);
            if err <= self.tolerance {
                self.t0 += 1;
                let shift = self.t0.trailing_zeros();
                self.t0 >>= shift;
                self.dt *= (1 << shift) as f64;
                let euler_params = EulerParams::from_angles(cubic_params.th0, cubic_params.th1);
                let es = EulerSeg::from_params(cubic.p0, cubic.p3, euler_params);
                return Some(es);
            }
            self.t0 *= 2;
            self.dt *= 0.5;
        }
    }
}

impl CubicToEulerIter {
    pub fn new(c: CubicBez, tolerance: f64) -> Self {
        CubicToEulerIter {
            c,
            tolerance,
            t0: 0,
            dt: 1.0,
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
