// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! Math for flattening of Euler spiral parallel curve

use std::f64::consts::FRAC_PI_4;

use xilem_web::svg::kurbo::BezPath;

use crate::euler::EulerSeg;

pub fn flatten_offset(iter: impl Iterator<Item = EulerSeg>, offset: f64, tol: f64) -> BezPath {
    let mut result = BezPath::new();
    let mut first = true;
    for es in iter {
        if core::mem::take(&mut first) {
            result.move_to(es.eval_with_offset(0.0, offset));
        }
        let scale = es.p0.distance(es.p1);
        let (k0, k1) = (es.params.k0 - 0.5 * es.params.k1, es.params.k1);
        // compute forward integral to determine number of subdivisions
        let dist_scaled = offset / scale;
        let a = -2.0 * dist_scaled * k1;
        let b = -1.0 - 2.0 * dist_scaled * k0;
        let int0 = espc_int_approx(b);
        let int1 = espc_int_approx(a + b);
        let integral = int1 - int0;
        let k_peak = k0 - k1 * b / a;
        let integrand_peak = (k_peak * (k_peak * dist_scaled + 1.0)).abs().sqrt();
        let scaled_int = integral * integrand_peak / a;
        let n_frac = 0.5 * (scale / tol).sqrt() * scaled_int;
        let n = n_frac.ceil();
        for i in 0..n as usize {
            let t = (i + 1) as f64 / n;
            let inv = espc_int_inv_approx(integral * t + int0);
            let s = (inv - b) / a;
            result.line_to(es.eval_with_offset(s, offset));
        }
    }
    result
}

const BREAK1: f64 = 0.8;
const BREAK2: f64 = 1.25;
const BREAK3: f64 = 2.1;
const SIN_SCALE: f64 = 1.0976991822760038;
const QUAD_A1: f64 = 0.6406;
const QUAD_B1: f64 = -0.81;
const QUAD_C1: f64 = 0.9148117935952064;
const QUAD_A2: f64 = 0.5;
const QUAD_B2: f64 = -0.156;
const QUAD_C2: f64 = 0.16145779359520596;

fn espc_int_approx(x: f64) -> f64 {
    let y = x.abs();
    let a = if y < BREAK1 {
        (SIN_SCALE * y).sin() * (1.0 / SIN_SCALE)
    } else if y < BREAK2 {
        (8.0f64.sqrt() / 3.0) * (y - 1.0) * (y - 1.0).abs().sqrt() + FRAC_PI_4
    } else {
        let (a, b, c) = if y < BREAK3 {
            (QUAD_A1, QUAD_B1, QUAD_C1)
        } else {
            (QUAD_A2, QUAD_B2, QUAD_C2)
        };
        a * y * y + b * y + c
    };
    a.copysign(x)
}

fn espc_int_inv_approx(x: f64) -> f64 {
    let y = x.abs();
    let a = if y < 0.7010707591262915 {
        (x * SIN_SCALE).asin() * (1.0 / SIN_SCALE)
    } else if y < 0.903249293595206 {
        let b = y - FRAC_PI_4;
        let u = b.abs().powf(2. / 3.).copysign(b);
        u * (9.0f64 / 8.).cbrt() + 1.0
    } else {
        let (u, v, w) = if y < 2.038857793595206 {
            const B: f64 = 0.5 * QUAD_B1 / QUAD_A1;
            (B * B - QUAD_C1 / QUAD_A1, 1.0 / QUAD_A1, B)
        } else {
            const B: f64 = 0.5 * QUAD_B2 / QUAD_A2;
            (B * B - QUAD_C2 / QUAD_A2, 1.0 / QUAD_A2, B)
        };
        (u + v * y).sqrt() - w
    };
    a.copysign(x)
}
