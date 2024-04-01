// Copyright 2023 the raphlinus.github.io Authors
// SPDX-License-Identifier: Apache-2.0

//! Convert an Euler spiral to a series of arcs.

use xilem_web::svg::kurbo::{SvgArc, Vec2};

use crate::euler::EulerSeg;

pub fn euler_to_arcs(es: &EulerSeg, tol: f64) -> Vec<SvgArc> {
    let arclen = es.p0.distance(es.p1) / es.params.ch;
    let n_subdiv = ((1. / 120.) * arclen / tol * es.params.k1.abs()).cbrt();
    web_sys::console::log_1(&format!("n_subdiv = {n_subdiv}").into());
    let n = (n_subdiv.ceil() as usize).max(1);
    let dt = 1.0 / n as f64;
    let mut p0 = es.p0;
    (0..n)
        .map(|i| {
            let t0 = i as f64 * dt;
            let t1 = t0 + dt;
            let p1 = if i + 1 == n { es.p1 } else { es.eval(t1) };
            let t = t0 + 0.5 * dt - 0.5;
            let k = es.params.k0 + t * es.params.k1;
            web_sys::console::log_1(&format!("{i}: k = {k} t = {t}").into());
            let r = arclen / k;
            let arc = SvgArc {
                from: p0,
                to: p1,
                radii: Vec2::new(r, r),
                x_rotation: 0.0,
                large_arc: false,
                sweep: k < 0.0,
            };
            p0 = p1;
            arc
        })
        .collect()
}

pub fn espc_to_arcs(es: &EulerSeg, d: f64, tol: f64) -> Vec<SvgArc> {
    let arclen = es.p0.distance(es.p1) / es.params.ch;
    // TODO: determine if there needs to be a scaling parameter on d. But this
    // seems to work well empirically.
    let est_err = (1. / 120.) / tol * es.params.k1.abs() * (arclen + d.abs());
    let n_subdiv = est_err.cbrt();
    web_sys::console::log_1(&format!("n_subdiv = {n_subdiv}").into());
    let n = (n_subdiv.ceil() as usize).max(1);
    let dt = 1.0 / n as f64;
    let mut p0 = es.eval_with_offset(0.0, d);
    (0..n)
        .map(|i| {
            let t0 = i as f64 * dt;
            let t1 = t0 + dt;
            let p1 = es.eval_with_offset(t1, d);
            let t = t0 + 0.5 * dt - 0.5;
            let k = es.params.k0 + t * es.params.k1;
            let arclen_offset = arclen + d * k;
            let r = arclen_offset / k;
            let arc = SvgArc {
                from: p0,
                to: p1,
                radii: Vec2::new(r, r),
                x_rotation: 0.0,
                large_arc: false,
                sweep: k < 0.0,
            };
            p0 = p1;
            arc
        })
        .collect()
}
