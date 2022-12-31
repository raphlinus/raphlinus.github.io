---
layout: post
title:  "Parallel curves of cubic Béziers"
date:   2022-09-09 10:45:42 -0700
categories: [curves]
---
<!-- I should figure out a cleaner way to do this include, rather than cutting and pasting. Ah well.-->
<script type="text/x-mathjax-config">
	MathJax.Hub.Config({
		tex2jax: {
			inlineMath: [['$', '$']]
		}
	});
</script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.0/MathJax.js?config=TeX-AMS-MML_HTMLorMML" type="text/javascript"></script>

<style>
    svg {
        touch-action: pinch-zoom;
        overflow: visible;
    }
    svg .handle {
        pointer-events: all;
    }
    svg .handle:hover {
        r: 6;
    }
    svg .quad {
        stroke-width: 1.5px;
        stroke: #222;
    }
    svg .hull {
        stroke: #a6c;
    }
    svg .approx_handle {
        stroke: #444;
    }
    svg .polyline {
    }
    svg text {
        font-family: sans-serif;
    }
    svg .button {
        fill: #aad;
        stroke: #44f;
    }
    svg .button:hover {
        fill: #bbf;
    }
    svg text {
        pointer-events: none;
    }
    svg #grid line {
        stroke: #e4e4e4;
    }
    svg .band {
        fill: #fda;
        opacity: 0.3;
    }
    img {
        margin: auto;
        margin: auto;
        display: block;
    }
    input#d {
        width: 300px;
    }
    input#tol {
        width: 4em;
    }
    input#alg {
        width: 4em;
    }
    .controls {
        display: grid;
        grid-template-columns: repeat(3, max-content);
        column-gap: 20px;
        margin-bottom: 15px;
    }
</style>
<svg id="s" width="700" height="500">
    <g id="grid"></g>
</svg>
<div class='controls'>
    <div>Distance</div>
    <div>Accuracy</div>
    <div>Method</div>
    <div><input type="range" min="1" max="100" value="40" id="d"></div>
    <div><input type="button" id="tol" value="1"></div>
    <div><input type="button" id="alg" value="Fit"></div>
</div>
<script>
// Copyright 2022 Google LLC

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     https://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

const svgNS = "http://www.w3.org/2000/svg";

class Point {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }

    lerp(p2, t) {
        return new Point(this.x + (p2.x - this.x) * t, this.y + (p2.y - this.y) * t);
    }

    dist(p2) {
        return Math.hypot(p2.x - this.x, p2.y - this.y);
    }

    hypot2() {
        return this.x * this.x + this.y * this.y;
    }

    hypot() {
        return Math.sqrt(this.hypot2());
    }

    dot(other) {
        return this.x * other.x + this.y * other.y;
    }

    cross(other) {
        return this.x * other.y - this.y * other.x;
    }

    plus(other) {
        return new Point(this.x + other.x, this.y + other.y);
    }

    minus(other) {
        return new Point(this.x - other.x, this.y - other.y);
    }

    atan2() {
        return Math.atan2(this.y, this.x);
    }
}

class Affine {
    constructor(c) {
        this.c = c;
    }

    apply_pt(p) {
        const c = this.c;
        const x = c[0] * p.x + c[2] * p.y + c[4];
        const y = c[1] * p.x + c[3] * p.y + c[5];
        return new Point(x, y);
    }

    apply_cubic(cu) {
        const c = this.c;
        const new_c = new Float64Array(8);
        for (let i = 0; i < 8; i += 2) {
            new_c[i] = c[0] * cu.c[i] + c[2] * cu.c[i + 1] + c[4];
            new_c[i + 1] = c[1] * cu.c[i] + c[3] * cu.c[i + 1] + c[5];
        }
        return new CubicBez(new_c);
    }

    static rotate(th) {
        const c = new Float64Array(6);
        c[0] = Math.cos(th);
        c[1] = Math.sin(th);
        c[2] = -c[1];
        c[3] = c[0];
        return new Affine(c);
    }
}

// Compute an approximation to int (1 + 4x^2) ^ -0.25 dx
// This isn't especially good but will do.
function approx_myint(x) {
   const d = 0.67; 
   return x / (1 - d + Math.pow(Math.pow(d, 4) + 0.25 * x * x, 0.25));
}

// Approximate the inverse of the function above.
// This is better.
function approx_inv_myint(x) {
    const b = 0.39;
    return x * (1 - b + Math.sqrt(b * b + 0.25 * x * x));
}

class QuadBez {
    constructor(x0, y0, x1, y1, x2, y2) {
        this.x0 = x0;
        this.y0 = y0;
        this.x1 = x1;
        this.y1 = y1;
        this.x2 = x2;
        this.y2 = y2;
    }

    to_svg_path() {
        return `M${this.x0} ${this.y0} Q${this.x1} ${this.y1} ${this.x2} ${this.y2}`
    }

    eval(t) {
        const mt = 1 - t;
        const x = this.x0 * mt * mt + 2 * this.x1 * t * mt + this.x2 * t * t;
        const y = this.y0 * mt * mt + 2 * this.y1 * t * mt + this.y2 * t * t;
        return new Point(x, y);
    }

    eval_deriv(t) {
        const mt = 1 - t;
        const x = 2 * (mt * (this.x1 - this.x0) + t * (this.x2 - this.x1));
        const y = 2 * (mt * (this.y1 - this.y0) + t * (this.y2 - this.y1));
        return new Point(x, y);
    }

    weightsum(c0, c1, c2) {
        const x = c0 * this.x0 + c1 * this.x1 + c2 * this.x2;
        const y = c0 * this.y0 + c1 * this.y1 + c2 * this.y2;
        return new Point(x, y);
    }

    subsegment(t0, t1) {
        const p0 = this.eval(t0);
        const p2 = this.eval(t1);
        const dt = t1 - t0;
        const p1x = p0.x + (this.x1 - this.x0 + t0 * (this.x2 - 2 * this.x1 + this.x0)) * dt;
        const p1y = p0.y + (this.y1 - this.y0 + t0 * (this.y2 - 2 * this.y1 + this.y0)) * dt;
        return new QuadBez(p0.x, p0.y, p1x, p1y, p2.x, p2.y);
    }
}

const GAUSS_LEGENDRE_COEFFS_8 = [
    0.3626837833783620, -0.1834346424956498,
    0.3626837833783620, 0.1834346424956498,
    0.3137066458778873, -0.5255324099163290,
    0.3137066458778873, 0.5255324099163290,
    0.2223810344533745, -0.7966664774136267,
    0.2223810344533745, 0.7966664774136267,
    0.1012285362903763, -0.9602898564975363,
    0.1012285362903763, 0.9602898564975363,
];

const GAUSS_LEGENDRE_COEFFS_8_HALF = [
    0.3626837833783620, 0.1834346424956498,
    0.3137066458778873, 0.5255324099163290,
    0.2223810344533745, 0.7966664774136267,
    0.1012285362903763, 0.9602898564975363,
];

const GAUSS_LEGENDRE_COEFFS_16_HALF = [
    0.1894506104550685, 0.0950125098376374,
    0.1826034150449236, 0.2816035507792589,
    0.1691565193950025, 0.4580167776572274,
    0.1495959888165767, 0.6178762444026438,
    0.1246289712555339, 0.7554044083550030,
    0.0951585116824928, 0.8656312023878318,
    0.0622535239386479, 0.9445750230732326,
    0.0271524594117541, 0.9894009349916499,
];

const GAUSS_LEGENDRE_COEFFS_24_HALF = [
    0.1279381953467522, 0.0640568928626056,
    0.1258374563468283, 0.1911188674736163,
    0.1216704729278034, 0.3150426796961634,
    0.1155056680537256, 0.4337935076260451,
    0.1074442701159656, 0.5454214713888396,
    0.0976186521041139, 0.6480936519369755,
    0.0861901615319533, 0.7401241915785544,
    0.0733464814110803, 0.8200019859739029,
    0.0592985849154368, 0.8864155270044011,
    0.0442774388174198, 0.9382745520027328,
    0.0285313886289337, 0.9747285559713095,
    0.0123412297999872, 0.9951872199970213,
];

const GAUSS_LEGENDRE_COEFFS_32 = [
    0.0965400885147278, -0.0483076656877383,
    0.0965400885147278, 0.0483076656877383,
    0.0956387200792749, -0.1444719615827965,
    0.0956387200792749, 0.1444719615827965,
    0.0938443990808046, -0.2392873622521371,
    0.0938443990808046, 0.2392873622521371,
    0.0911738786957639, -0.3318686022821277,
    0.0911738786957639, 0.3318686022821277,
    0.0876520930044038, -0.4213512761306353,
    0.0876520930044038, 0.4213512761306353,
    0.0833119242269467, -0.5068999089322294,
    0.0833119242269467, 0.5068999089322294,
    0.0781938957870703, -0.5877157572407623,
    0.0781938957870703, 0.5877157572407623,
    0.0723457941088485, -0.6630442669302152,
    0.0723457941088485, 0.6630442669302152,
    0.0658222227763618, -0.7321821187402897,
    0.0658222227763618, 0.7321821187402897,
    0.0586840934785355, -0.7944837959679424,
    0.0586840934785355, 0.7944837959679424,
    0.0509980592623762, -0.8493676137325700,
    0.0509980592623762, 0.8493676137325700,
    0.0428358980222267, -0.8963211557660521,
    0.0428358980222267, 0.8963211557660521,
    0.0342738629130214, -0.9349060759377397,
    0.0342738629130214, 0.9349060759377397,
    0.0253920653092621, -0.9647622555875064,
    0.0253920653092621, 0.9647622555875064,
    0.0162743947309057, -0.9856115115452684,
    0.0162743947309057, 0.9856115115452684,
    0.0070186100094701, -0.9972638618494816,
    0.0070186100094701, 0.9972638618494816,
];

const GAUSS_LEGENDRE_COEFFS_32_HALF = [
    0.0965400885147278, 0.0483076656877383,
    0.0956387200792749, 0.1444719615827965,
    0.0938443990808046, 0.2392873622521371,
    0.0911738786957639, 0.3318686022821277,
    0.0876520930044038, 0.4213512761306353,
    0.0833119242269467, 0.5068999089322294,
    0.0781938957870703, 0.5877157572407623,
    0.0723457941088485, 0.6630442669302152,
    0.0658222227763618, 0.7321821187402897,
    0.0586840934785355, 0.7944837959679424,
    0.0509980592623762, 0.8493676137325700,
    0.0428358980222267, 0.8963211557660521,
    0.0342738629130214, 0.9349060759377397,
    0.0253920653092621, 0.9647622555875064,
    0.0162743947309057, 0.9856115115452684,
    0.0070186100094701, 0.9972638618494816,
];

function tri_sign(x0, y0, x1, y1) {
    return x1 * (y0 - y1) - y1 * (x0 - x1);
}

// Return distance squared
function line_nearest_origin(x0, y0, x1, y1) {
    const dx = x1 - x0;
    const dy = y1 - y0;
    let dotp = -dx * x0 - dy * y0;
    let d_sq = dx * dx + dy * dy;
    if (dotp <= 0) {
        return x0 * x0 + y0 * y0;
    } else if (dotp >= d_sq) {
        return x1 * x1 + y1 * y1;
    } else {
        const t = dotp / d_sq;
        const x = x0 + t * (x1 - x0);
        const y = y0 + t * (y1 - y0);
        return x * x + y * y;
    }
}

class CubicBez {
    /// Argument is array of coordinate values [x0, y0, x1, y1, x2, y2, x3, y3].
    constructor(coords) {
        this.c = coords;
    }

    static from_pts(p0, p1, p2, p3) {
        const c = new Float64Array(8);
        c[0] = p0.x;
        c[1] = p0.y;
        c[2] = p1.x;
        c[3] = p1.y;
        c[4] = p2.x;
        c[5] = p2.y;
        c[6] = p3.x;
        c[7] = p3.y;
        return new CubicBez(c);
    }

    p0() {
        return new Point(this.c[0], this.c[1]);
    }

    p1() {
        return new Point(this.c[2], this.c[3]);
    }

    p2() {
        return new Point(this.c[4], this.c[5]);
    }

    p3() {
        return new Point(this.c[6], this.c[7]);
    }

    to_svg_path() {
        const c = this.c;
        return `M${c[0]} ${c[1]}C${c[2]} ${c[3]} ${c[4]} ${c[5]} ${c[6]} ${c[7]}`
    }

    weightsum(c0, c1, c2, c3) {
        const x = c0 * this.c[0] + c1 * this.c[2] + c2 * this.c[4] + c3 * this.c[6];
        const y = c0 * this.c[1] + c1 * this.c[3] + c2 * this.c[5] + c3 * this.c[7];
        return new Point(x, y);
    }

    eval(t) {
        const mt = 1 - t;
        const c0 = mt * mt * mt;
        const c1 = 3 * mt * mt * t;
        const c2 = 3 * mt * t * t;
        const c3 = t * t * t;
        return this.weightsum(c0, c1, c2, c3);
    }

    eval_deriv(t) {
        const mt = 1 - t;
        const c0 = -3 * mt * mt;
        const c3 = 3 * t * t;
        const c1 = -6 * t * mt - c0;
        const c2 = 6 * t * mt - c3;
        return this.weightsum(c0, c1, c2, c3);
    }

    // quadratic bezier with matching endpoints and minimum max vector error
    midpoint_quadbez() {
        const p1 = this.weightsum(-0.25, 0.75, 0.75, -0.25);
        return new QuadBez(this.c[0], this.c[1], p1.x, p1.y, this.c[6], this.c[7]);
    }

    subsegment(t0, t1) {
        let c = new Float64Array(8);
        const p0 = this.eval(t0);
        const p3 = this.eval(t1);
        c[0] = p0.x;
        c[1] = p0.y;
        const scale = (t1 - t0) / 3;
        const d1 = this.eval_deriv(t0);
        c[2] = p0.x + scale * d1.x;
        c[3] = p0.y + scale * d1.y;
        const d2 = this.eval_deriv(t1);
        c[4] = p3.x - scale * d2.x;
        c[5] = p3.y - scale * d2.y;
        c[6] = p3.x;
        c[7] = p3.y;
        return new CubicBez(c);
    }

    area() {
        const c = this.c;
        return (c[0] * (6 * c[3] + 3 * c[5] + c[7])
            + 3 * (c[2] * (-2 * c[1] + c[5] + c[7]) - c[4] * (c[1] + c[3] - 2 * c[7]))
            - c[6] * (c[1] + 3 * c[3] + 6 * c[5]))
            * 0.05;
    }

    chord() {
        return new Line([this.c[0], this.c[1], this.c[6], this.c[7]]);
    }

    deriv() {
        const c = this.c;
        return new QuadBez(
            3 * (c[2] - c[0]), 3 * (c[3] - c[1]),
            3 * (c[4] - c[2]), 3 * (c[5] - c[3]),
            3 * (c[6] - c[4]), 3 * (c[7] - c[5])
        );
    }

    // A pretty good algorithm; kurbo does more sophisticated error analysis.
    arclen(accuracy) {
        return this.arclen_rec(accuracy, 0);
    }

    arclen_rec(accuracy, depth) {
        const c = this.c;
        const d03x = c[6] - c[0];
        const d03y = c[7] - c[1];
        const d01x = c[2] - c[0];
        const d01y = c[3] - c[1];
        const d12x = c[4] - c[2];
        const d12y = c[5] - c[3];
        const d23x = c[6] - c[4];
        const d23y = c[7] - c[5];
        const lp_lc = Math.hypot(d01x, d01y) + Math.hypot(d12x, d12y)
            + Math.hypot(d23x, d23y) - Math.hypot(d03x, d03y);
        const dd1x = d12x - d01x;
        const dd1y = d12y - d01y;
        const dd2x = d23x - d12x;
        const dd2y = d23y - d12y;
        const dmx = 0.25 * (d01x + d23x) + 0.5 * d12x;
        const dmy = 0.25 * (d01y + d23y) + 0.5 * d12y;
        const dm1x = 0.5 * (dd2x + dd1x);
        const dm1y = 0.5 * (dd2y + dd1y);
        const dm2x = 0.25 * (dd2x - dd1x);
        const dm2y = 0.25 * (dd2y - dd1y);
        const co_e = GAUSS_LEGENDRE_COEFFS_8;
        let est = 0;
        for (let i = 0; i < co_e.length; i += 2) {
            const xi = co_e[i + 1];
            const dx = dmx + dm1x * xi + dm2x * (xi * xi);
            const dy = dmy + dm1y * xi + dm2y * (xi * xi);
            const ddx = dm1x + dm2x * (2 * xi);
            const ddy = dm1y + dm2y * (2 * xi);
            est += co_e[i] * (ddx * ddx + ddy * ddy) / (dx * dx + dy * dy);
        }
        const est3 = est * est * est;
        const est_gauss8_err = Math.min(est3 * 2.5e-6, 3e-2) * lp_lc;
        let co = null;
        if (Math.min(est3 * 2.5e-6, 3e-2) * lp_lc < accuracy) {
            co = GAUSS_LEGENDRE_COEFFS_8_HALF;
        } else if (Math.min(est3 * est3 * 1.5e-11, 9e-3) * lp_lc < accuracy) {
            co = GAUSS_LEGENDRE_COEFFS_16_HALF;
        } else if (Math.min(est3 * est3 * est3 * 3.5e-16, 3.5e-3) * lp_lc < accuracy
            || depth >= 20)
        {
            co = GAUSS_LEGENDRE_COEFFS_24_HALF;
        } else {
            const c0 = this.subsegment(0, 0.5);
            const c1 = this.subsegment(0.5, 1);
            return c0.arclen_rec(accuracy * 0.5, depth + 1)
                + c1.arclen_rec(accuracy * 0.5, depth + 1);
        }
        let sum = 0;
        for (let i = 0; i < co.length; i += 2) {
            const xi = co[i + 1];
            const wi = co[i];
            const dx = dmx + dm2x * (xi * xi);
            const dy = dmy + dm2y * (xi * xi);
            const dp = Math.hypot(dx + dm1x * xi, dy + dm1y * xi);
            const dm = Math.hypot(dx - dm1x * xi, dy - dm1y * xi);
            sum += wi * (dp + dm);
        }
        return 1.5 * sum;
    }

    inv_arclen(s, accuracy) {
        if (s <= 0) {
            return 0;
        }
        const total_arclen = this.arclen(accuracy);
        if (s >= total_arclen) {
            return 1;
        }
        // For simplicity, measure arclen from 0 rather than stateful delta.
        const f = t => this.subsegment(0, t).arclen(accuracy) - s;
        const epsilon = accuracy / total_arclen;
        return solve_itp(f, 0, 1, epsilon, 1, 2, -s, total_arclen -s);
    }

    find_offset_cusps(d) {
        const q = this.deriv();
        // x'' cross x' is a quadratic polynomial in t
        const d0x = q.x0;
        const d0y = q.y0;
        const d1x = 2 * (q.x1 - q.x0);
        const d1y = 2 * (q.y1 - q.y0);
        const d2x = q.x0 - 2 * q.x1 + q.x2;
        const d2y = q.y0 - 2 * q.y1 + q.y2;
        const c0 = d1x * d0y - d1y * d0x;
        const c1 = 2 * (d2x * d0y - d2y * d0x);
        const c2 = d2x * d1y - d2y * d1x;
        const cusps = new CuspAccumulator(d, q, c0, c1, c2);
        this.find_offset_cusps_rec(d, cusps, 0, 1, c0, c1, c2);
        return cusps.reap();
    }

    find_offset_cusps_rec(d, cusps, t0, t1, c0, c1, c2) {
        cusps.report(t0);
        const dt = t1 - t0;
        const q = this.subsegment(t0, t1).deriv();
        // compute interval for ds/dt, using convex hull of hodograph
        const d1 = tri_sign(q.x0, q.y0, q.x1, q.y1);
        const d2 = tri_sign(q.x1, q.y1, q.x2, q.y2);
        const d3 = tri_sign(q.x2, q.y2, q.x0, q.y0);
        const z = !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
        const ds0 = q.x0 * q.x0 + q.y0 * q.y0;
        const ds1 = q.x1 * q.x1 + q.y1 * q.y1;
        const ds2 = q.x2 * q.x2 + q.y2 * q.y2;
        const max_ds = Math.sqrt(Math.max(ds0, ds1, ds2)) / dt;
        const m1 = line_nearest_origin(q.x0, q.y0, q.x1, q.y1);
        const m2 = line_nearest_origin(q.x1, q.y1, q.x2, q.y2);
        const m3 = line_nearest_origin(q.x2, q.y2, q.x0, q.y0);
        const min_ds = z ? 0 : Math.sqrt(Math.min(m1, m2, m3)) / dt;
        //console.log('ds interval', min_ds, max_ds, 'iv', t0, t1);
        let cmin = Math.min(c0, c0 + c1 + c2);
        let cmax = Math.max(c0, c0 + c1 + c2);
        const t_crit = -0.5 * c1 / c2;
        const c_at_t = (c2 * t_crit + c1) * t_crit + c0;
        if (t_crit > 0 && t_crit < 1) {
            let c_at_t = (c2 * t_crit + c1) * t_crit + c0;
            cmin = Math.min(cmin, c_at_t);
            cmax = Math.max(cmax, c_at_t);
        }
        const min3 = min_ds * min_ds * min_ds;
        const max3 = max_ds * max_ds * max_ds;
        // TODO: signs are wrong, want min/max of c * d
        // But this is a suitable starting place for clipping.
        if (cmin * d > -min3 || cmax * d < -max3) {
            //return;
        }
        const rmax = solve_quadratic(c0 * d + max3, c1 * d, c2 * d);
        const rmin = solve_quadratic(c0 * d + min3, c1 * d, c2 * d);
        let ts;
        // TODO: length = 1 cases. Also maybe reduce case explosion?
        if (rmax.length == 2 && rmin.length == 2) {
            if (c2 > 0) {
                ts = [rmin[0], rmax[0], rmax[1], rmin[1]];
            } else {
                ts = [rmax[0], rmin[0], rmin[1], rmax[1]];
            }
        } else if (rmin.length == 2) {
            if (c2 > 0) {
                ts = rmin;
            } else {
                ts = [t0, rmin[0], rmin[1], t1];
            }
        } else if (rmax.length == 2) {
            if (c2 > 0) {
                ts = [t0, rmax[0], rmax[1], t1];
            } else {
                ts = rmax;
            }
        } else {
            const c_at_t0 = (c2 * t0 + c1) * t0 + c0;
            if (c_at_t0 * d < -min3 && c_at_t0 * d > -max3) {
                ts = [t0, t1];
            } else {
                ts = [];
            }
        }
        for (let i = 0; i < ts.length; i += 2) {
            const new_t0 = Math.max(t0, ts[i]);
            const new_t1 = Math.min(t1, ts[i + 1]);
            if (new_t1 > new_t0) {
                if (new_t1 - new_t0 < 1e-9) {
                    cusps.report(new_t0);
                    cusps.report(new_t1);
                } else if (new_t1 - new_t0 > 0.5 * dt) {
                    const tm = 0.5 * (new_t0 + new_t1);
                    this.find_offset_cusps_rec(d, cusps, new_t0, tm, c0, c1, c2);
                    this.find_offset_cusps_rec(d, cusps, tm, new_t1, c0, c1, c2);
                } else {
                    this.find_offset_cusps_rec(d, cusps, new_t0, new_t1, c0, c1, c2);
                }
                //console.log('iv', new_t0, new_t1);
            }
        }
        cusps.report(t1);
        //console.log(rmax);
        //console.log(rmin);
        //console.log('ts:', ts);
    }

    /*
    // This is a brute-force solution; a more robust one is started above.
    // Output is a partition of (0..1) into ranges, with signs.
    find_offset_cusps(d) {
        const result = [];
        const n = 100;
        const q = this.deriv();
        // x'' cross x' is a quadratic polynomial in t
        const d0x = q.x0;
        const d0y = q.y0;
        const d1x = 2 * (q.x1 - q.x0);
        const d1y = 2 * (q.y1 - q.y0);
        const d2x = q.x0 - 2 * q.x1 + q.x2;
        const d2y = q.y0 - 2 * q.y1 + q.y2;
        const c0 = d1x * d0y - d1y * d0x;
        const c1 = 2 * (d2x * d0y - d2y * d0x);
        const c2 = d2x * d1y - d2y * d1x;
        let ya;
        let last_t;
        let t0 = 0;
        for (let i = 0; i <= n; i++) {
            const t = i / n;
            const ds2 = q.eval(t).hypot2();
            const k = (((c2 * t + c1) * t) + c0) / (ds2 * Math.sqrt(ds2));
            const yb = k * d + 1;
            if (i != 0) {
                if (ya >= 0 != yb >= 0) {
                    let tx = (yb * last_t - ya * t) / (yb - ya);
                    const iv = {'t0': t0, 't1': tx, 'sign': Math.sign(ya)};
                    result.push(iv);
                    t0 = tx;
                }
            }
            ya = yb;
            last_t = t;
        }
        const last_iv = {'t0': t0, 't1': 1, 'sign': Math.sign(ya)};
        result.push(last_iv);
        return result;
    }
    */

    // Find intersections of ray from point p with tangent d
    intersect_ray(p, d) {
        const c = this.c
        const px0 = c[0];
        const px1 = 3 * c[2] - 3 * c[0];
        const px2 = 3 * c[4] - 6 * c[2] + 3 * c[0];
        const px3 = c[6] - 3 * c[4] + 3 * c[2] - c[0];
        const py0 = c[1];
        const py1 = 3 * c[3] - 3 * c[1];
        const py2 = 3 * c[5] - 6 * c[3] + 3 * c[1];
        const py3 = c[7] - 3 * c[5] + 3 * c[3] - c[1];
        const c0 = d.y * (px0 - p.x) - d.x * (py0 - p.y);
        const c1 = d.y * px1 - d.x * py1;
        const c2 = d.y * px2 - d.x * py2;
        const c3 = d.y * px3 - d.x * py3;
        return solve_cubic(c0, c1, c2, c3).filter(t => t > 0 && t < 1);
    }
}

class CuspAccumulator {
    constructor(d, q, c0, c1, c2) {
        this.d = d;
        this.q = q;
        this.c0 = c0;
        this.c1 = c1;
        this.c2 = c2;
        this.t0 = 0;
        this.last_t = 0;
        this.last_y = this.eval(0);
        this.result = [];
    }

    eval(t) {
        const ds2 = this.q.eval(t).hypot2();
        const k = (((this.c2 * t + this.c1) * t) + this.c0) / (ds2 * Math.sqrt(ds2));
        return k * this.d + 1;
    }

    report(t) {
        const yb = this.eval(t);
        const ya = this.last_y;
        if (ya >= 0 != yb >= 0) {
            // More wired: use ITP
            let tx = (yb * this.last_t - ya * t) / (yb - ya);
            const iv = {'t0': this.t0, 't1': tx, 'sign': Math.sign(ya)};
            this.result.push(iv);
            this.t0 = tx;
        }
        this.last_t = t;
        this.last_y = yb;
    }

    reap() {
        const last_iv = {'t0': this.t0, 't1': 1, 'sign': Math.sign(this.last_y)};
        this.result.push(last_iv);
        return this.result;
    }
}

class Line {
    /// Argument is array of coordinate values [x0, y0, x1, y1].
    constructor(coords) {
        this.c = coords;
    }

    area() {
        return (this.c[0] * this.c[3] - this.c[1] * this.c[2]) * 0.5;
    }
}

function copysign(x, y) {
    const a = Math.abs(x);
    return y < 0 ? -a : a;
}

function solve_quadratic(c0, c1, c2) {
    const sc0 = c0 / c2;
    const sc1 = c1 / c2;
    if (!(isFinite(sc0) && isFinite(sc1))) {
        const root = -c0 / c1;
        if (isFinite(root)) {
            return [root];
        } else if (c0 == 0 && c1 == 0) {
            return [0];
        } else {
            return [];
        }
    }
    const arg = sc1 * sc1 - 4 * sc0;
    let root1 = 0;
    if (isFinite(arg)) {
        if (arg < 0) {
            return [];
        } else if (arg == 0) {
            return [-0.5 * sc1];
        }
        root1 = -.5 * (sc1 + copysign(Math.sqrt(arg), sc1));
    } else {
        root1 = -sc1;
    }
    const root2 = sc0 / root1;
    if (isFinite(root2)) {
        if (root2 > root1) {
            return [root1, root2];
        } else {
            return [root2, root1];
        }
    }
    return [root1];
}

// See kurbo common.rs
function solve_cubic(in_c0, in_c1, in_c2, in_c3) {
    const c2 = in_c2 / (3 * in_c3);
    const c1 = in_c1 / (3 * in_c3);
    const c0 = in_c0 / in_c3;
    if (!(isFinite(c0) && isFinite(c1) && isFinite(c2))) {
        return solve_quadratic(in_c0, in_c1, in_c2);
    }
    const d0 = -c2 * c2 + c1;
    const d1 = -c1 * c2 + c0;
    const d2 = c2 * c0 - c1 * c1;
    const d = 4 * d0 * d2 - d1 * d1;
    const de = -2 * c2 * d0 + d1;
    if (d < 0) {
        const sq = Math.sqrt(-0.25 * d);
        const r = -0.5 * de;
        const t1 = Math.cbrt(r + sq) + Math.cbrt(r - sq);
        return [t1 - c2];
    } else if (d == 0) {
        const t1 = copysign(Math.sqrt(-d0), de);
        return [t1 - c2, -2 * t1 - c2];
    } else {
        const th = Math.atan2(Math.sqrt(d), -de) / 3;
        const r0 = Math.cos(th);
        const ss3 = Math.sin(th) * Math.sqrt(3);
        const r1 = 0.5 * (-r0 + ss3);
        const r2 = 0.5 * (-r0 - ss3);
        const t = 2 * Math.sqrt(-d0);
        return [t * r0 - c2, t * r1 - c2, t * r2 - c2];
    }
}

// Factor a quartic polynomial into two quadratics. Based on Orellana and De Michele
// and very similar to the version in kurbo.
function solve_quartic(c0, c1, c2, c3, c4) {
    // This doesn't special-case c0 = 0.
    if (c4 == 0) {
        return solve_cubic(c0, c1, c2, c3);
    }
    const a = c3 / c4;
    const b = c2 / c4;
    const c = c1 / c4;
    const d = c0 / c4;
    let result = solve_quartic_inner(a, b, c, d, false);
    if (result !== null) {
        return result;
    }
    const K_Q = 7.16e76;
    for (let i = 0; i < 2; i++) {
        result = solve_quartic_inner(a / K_Q, b / (K_Q * K_Q), c / (K_Q * K_Q * K_Q),
            d / (K_Q * K_Q * K_Q * K_Q), i != 0);
        if (result !== null) {
            for (let j = 0; j < result.length; j++) {
                result[j] *= K_Q;
            }
            return result;
        }
    }
    // Really bad overflow happened.
    return [];
}

function eps_rel(raw, a) {
    return a == 0 ? Math.abs(raw) : Math.abs((raw - a) / a);
}

function solve_quartic_inner(a, b, c, d, rescale) {
    let result = factor_quartic_inner(a, b, c, d, rescale);
    if (result !== null && result.length == 4) {
        let roots = [];
        for (let i = 0; i < 2; i++) {
            const a = result[i * 2];
            const b = result[i * 2 + 1];
            roots = roots.concat(solve_quadratic(b, a, 1));
        }
        return roots;
    }
}

function factor_quartic_inner(a, b, c, d, rescale) {
    function calc_eps_q(a1, b1, a2, b2) {
        const eps_a = eps_rel(a1 + a2, a);
        const eps_b = eps_rel(b1 + a1 * a2 + b2, b);
        const eps_c = eps_rel(b1 * a2 + a1 * b2, c);
        return eps_a + eps_b + eps_c;
    }
    function calc_eps_t(a1, b1, a2, b2) {
        return calc_eps_q(a1, b1, a2, b2) + eps_rel(b1 * b2, d);
    }
    const disc = 9 * a * a - 24 * b;
    const s = disc >= 0 ? -2 * b / (3 * a + copysign(Math.sqrt(disc), a)) : -0.25 * a;
    const a_prime = a + 4 * s;
    const b_prime = b + 3 * s * (a + 2 * s);
    const c_prime = c + s * (2 * b + s * (3 * a + 4 * s));
    const d_prime = d + s * (c + s * (b + s * (a + s)));
    let g_prime = 0;
    let h_prime = 0;
    const K_C = 3.49e102;
    if (rescale) {
        const a_prime_s = a_prime / K_C;
        const b_prime_s = b_prime / K_C;
        const c_prime_s = c_prime / K_C;
        const d_prime_s = d_prime / K_C;
        g_prime = a_prime_s * c_prime_s - (4 / K_C) * d_prime_s - (1. / 3) * b_prime_s * b_prime_s;
        h_prime = (a_prime_s * c_prime_s - (8 / K_C) * d_prime_s - (2. / 9) * b_prime_s * b_prime_s)
            * (1. / 3) * b_prime_s
            - c_prime_s * (c_prime_s / K_C)
            - a_prime_s * a_prime_s * d_prime_s;
    } else {
        g_prime = a_prime * c_prime - 4 * d_prime - (1. / 3) * b_prime * b_prime;
        h_prime = (a_prime * c_prime + 8 * d_prime - (2. / 9) * b_prime * b_prime) * (1. / 3) * b_prime
            - c_prime * c_prime
            - a_prime * a_prime * d_prime;
    }
    if (!isFinite(g_prime) && isFinite(h_prime)) {
        return null;
    }
    let phi = depressed_cubic_dominant(g_prime, h_prime);
    if (rescale) {
        phi *= K_C;
    }
    const l_1 = a * 0.5;
    const l_3 = (1. / 6) * b + 0.5 * phi;
    const delt_2 = c - a * l_3;
    const d_2_cand_1 = (2. / 3) * b - phi - l_1 * l_1;
    const l_2_cand_1 = 0.5 * delt_2 / d_2_cand_1;
    const l_2_cand_2 = 2 * (d - l_3 * l_3) / delt_2;
    const d_2_cand_2 = 0.5 * delt_2 / l_2_cand_2;
    let d_2_best = 0;
    let l_2_best = 0;
    for (let i = 0; i < 3; i++) {
        const d_2 = i == 1 ? d_2_cand_2 : d_2_cand_1;
        const l_2 = i == 0 ? l_2_cand_1 : l_2_cand_2;
        const eps_0 = eps_rel(d_2 + l_1 * l_1 + 2 * l_3, b);
        const eps_1 = eps_rel(2 * (d_2 * l_2 + l_1 * l_3), c);
        const eps_2 = eps_rel(d_2 * l_2 * l_2 + l_3 * l_3, d);
        const eps_l = eps_0 + eps_1 + eps_2;
        if (i == 0 || eps_l < eps_l_best) {
            d_2_best = d_2;
            l_2_best = l_2;
            eps_l_best = eps_l;
        }
    }
    const d_2 = d_2_best;
    const l_2 = l_2_best;
    let alpha_1 = 0;
    let beta_1 = 0;
    let alpha_2 = 0;
    let beta_2 = 0;
    if (d_2 < 0.0) {
        const sq = Math.sqrt(-d_2);
        alpha_1 = l_1 + sq;
        beta_1 = l_3 + sq * l_2;
        alpha_2 = l_1 - sq;
        beta_2 = l_3 - sq * l_2;
        if (Math.abs(beta_2) < Math.abs(beta_1)) {
            beta_2 = d / beta_1;
        } else if (Math.abs(beta_2) > Math.abs(beta_1)) {
            beta_1 = d / beta_2;
        }
        if (Math.abs(alpha_1) != Math.abs(alpha_2)) {
            let a1_cands = null;
            let a2_cands = null;
            if (Math.abs(alpha_1) < Math.abs(alpha_2)) {
                const a1_cand_1 = (c - beta_1 * alpha_2) / beta_2;
                const a1_cand_2 = (b - beta_2 - beta_1) / alpha_2;
                const a1_cand_3 = a - alpha_2;
                a1_cands = [a1_cand_3, a1_cand_1, a1_cand_2];
                a2_cands = [alpha_2, alpha_2, alpha_2];
            } else {
                const a2_cand_1 = (c - alpha_1 * beta_2) / beta_1;
                const a2_cand_2 = (b - beta_2 - beta_1) / alpha_1;
                const a2_cand_3 = a - alpha_1;
                a1_cands = [alpha_1, alpha_1, alpha_1];
                a2_cands = [a2_cand_3, a2_cand_1, a2_cand_2];
            }
            let eps_q_best = 0;
            for (let i = 0; i < 3; i++) {
                const a1 = a1_cands[i];
                const a2 = a2_cands[i];
                if (isFinite(a1) && isFinite(a2)) {
                    const eps_q = calc_eps_q(a1, beta_1, a2, beta_2);
                    if (i == 0 || eps_q < eps_q_best) {
                        alpha_1 = a1;
                        alpha_2 = a2;
                        eps_q_best = eps_q;
                    }
                }
            }
        }
    } else if (d_2 == 0) {
        const d_3 = d - l_3 * l_3;
        alpha_1 = l_1;
        beta_1 = l_3 + Math.sqrt(-d_3);
        alpha_2 = l_1;
        beta_2 = l_3 - Math.sqrt(-d_3);
        if (Math.abs(beta_1) > Math.abs(beta_2)) {
            beta_2 = d / beta_1;
        } else if (Math.abs(beta_2) > Math.abs(beta_1)) {
            beta_1 = d / beta_2;
        }
    } else {
        // No real solutions
        return [];
    }
    let eps_t = calc_eps_t(alpha_1, beta_1, alpha_2, beta_2);
    for (let i = 0; i < 8; i++) {
        if (eps_t == 0) {
            break;
        }
        const f_0 = beta_1 * beta_2 - d;
        const f_1 = beta_1 * alpha_2 + alpha_1 * beta_2 - c;
        const f_2 = beta_1 + alpha_1 * alpha_2 + beta_2 - b;
        const f_3 = alpha_1 + alpha_2 - a;
        const c_1 = alpha_1 - alpha_2;
        const det_j = beta_1 * beta_1 - beta_1 * (alpha_2 * c_1 + 2 * beta_2)
            + beta_2 * (alpha_1 * c_1 + beta_2);
        if (det_j == 0) {
            break;
        }
        const inv = 1 / det_j;
        const c_2 = beta_2 - beta_1;
        const c_3 = beta_1 * alpha_2 - alpha_1 * beta_2;
        const dz_0 = c_1 * f_0 + c_2 * f_1 + c_3 * f_2 - (beta_1 * c_2 + alpha_1 * c_3) * f_3;
        const dz_1 = (alpha_1 * c_1 + c_2) * f_0
            - beta_1 * (c_1 * f_1 + c_2 * f_2 + c_3 * f_3);
        const dz_2 = -c_1 * f_0 - c_2 * f_1 - c_3 * f_2 + (alpha_2 * c_3 + beta_2 * c_2) * f_3;
        const dz_3 = -(alpha_2 * c_1 + c_2) * f_0
            + beta_2 * (c_1 * f_1 + c_2 * f_2 + c_3 * f_3);
        const a1 = alpha_1 - inv * dz_0;
        const b1 = beta_1 - inv * dz_1;
        const a2 = alpha_2 - inv * dz_2;
        const b2 = beta_2 - inv * dz_3;
        const new_eps_t = calc_eps_t(a1, b1, a2, b2);
        if (new_eps_t < eps_t) {
            alpha_1 = a1;
            beta_1 = b1;
            alpha_2 = a2;
            beta_2 = b2;
            eps_t = new_eps_t;
        } else {
            break;
        }
    }
    return [alpha_1, beta_1, alpha_2, beta_2];
}

function depressed_cubic_dominant(g, h) {
    const q = (-1. / 3) * g;
    const r = 0.5 * h;
    let phi_0;
    let k = null;
    if (Math.abs(q) >= 1e102 || Math.abs(r) >= 1e164) {
        if (Math.abs(q) < Math.abs(r)) {
            k = 1 - q * (q / r) * (q / r);
        } else {
            k = Math.sign(q) * ((r / q) * (r / q) / q - 1);
        }
    }
    if (k !== null && r == 0) {
        if (g > 0) {
            phi_0 = 0;
        } else {
            phi_0 = Math.sqrt(-g);
        }
    } else if (k !== null ? k < 0 : r * r < q * q * q) {
        const t = k !== null ? r / q / Math.sqrt(q) : r / Math.sqrt(q * q * q);
        phi_0 = -2 * Math.sqrt(q) * copysign(Math.cos(Math.acos(Math.abs(t)) * (1. / 3)), t);
    } else {
        let a;
        if (k !== null) {
            if (Math.abs(q) < Math.abs(r)) {
                a = -r * (1 + Math.sqrt(k));
            } else {
                a = -r - copysign(Math.sqrt(Math.abs(q)) * q * Math.sqrt(k), r);
            }
        } else {
            a = Math.cbrt(-r - copysign(Math.sqrt(r * r - q * q * q), r));
        }
        const b = a == 0 ? 0 : q / a;
        phi_0 = a + b;
    }
    let x = phi_0;
    let f = (x * x + g) * x + h;
    const EPS_M = 2.22045e-16;
    if (Math.abs(f) < EPS_M * Math.max(x * x * x, g * x, h)) {
        return x;
    }
    for (let i = 0; i < 8; i++) {
        const delt_f = 3 * x * x + g;
        if (delt_f == 0) {
            break;
        }
        const new_x = x - f / delt_f;
        const new_f = (new_x * new_x + g) * new_x + h;
        if (new_f == 0) {
            return new_x;
        }
        if (Math.abs(new_f) >= Math.abs(f)) {
            break;
        }
        x = new_x;
        f = new_f;
    }
    return x;
}

// For testing.
function vieta(x1, x2, x3, x4) {
    const a = -(x1 + x2 + x3 + x4);
    const b = x1 * (x2 + x3) + x2 * (x3 + x4) + x4 * (x1 + x3);
    const c = -x1 * x2 * (x3 + x4) - x3 * x4 * (x1 + x2);
    const d = x1 * x2 * x3 * x4;
    const roots = solve_quartic(d, c, b, a, 1);
    return roots;
}

// See common.rs in kurbo
function solve_itp(f, a, b, epsilon, n0, k1, ya, yb) {
    const n1_2 = Math.max(Math.ceil(Math.log2((b - a) / epsilon)) - 1, 0);
    const nmax = n0 + n1_2;
    let scaled_epsilon = epsilon * Math.exp(nmax * Math.LN2);
    while (b - a > 2 * epsilon) {
        const x1_2 = 0.5 * (a + b);
        const r = scaled_epsilon - 0.5 * (b - a);
        const xf = (yb * a - ya * b) / (yb - ya);
        const sigma = x1_2 - xf;
        const delta = k1 * (b - a) * (b - a);
        const xt = delta <= Math.abs(x1_2 - xf) ? xf + copysign(delta, sigma) : x1_2;
        const xitp = Math.abs(xt - x1_2) <= r ? xt : x1_2 - copysign(r, sigma);
        const yitp = f(xitp);
        if (yitp > 0) {
            b = xitp;
            yb = yitp;
        } else if (yitp < 0) {
            a = xitp;
            ya = yitp;
        } else {
            return xitp;
        }
        scaled_epsilon *= 0.5
    }
    return 0.5 * (a + b);
}

function ray_intersect(p0, d0, p1, d1) {
    const det = d0.x * d1.y - d0.y * d1.x;
    const t = (d0.x * (p0.y - p1.y) - d0.y * (p0.x - p1.x)) / det;
    return new Point(p1.x + d1.x * t, p1.y + d1.y * t);
}

class CubicOffset {
    constructor(c, d) {
        this.c = c;
        this.q = c.deriv();
        this.d = d;
    }

    eval_offset(t) {
        const dp = this.q.eval(t);
        const s = this.d / dp.hypot();
        return new Point(-s * dp.y, s * dp.x);
    }

    eval(t) {
        return this.c.eval(t).plus(this.eval_offset(t));
    }

    eval_deriv(t) {
        const dp = this.q.eval(t);
        const ddp = this.q.eval_deriv(t);
        const h = dp.hypot2();
        const turn = ddp.cross(dp) * this.d / (h * Math.sqrt(h));
        const s = 1 + turn;
        return new Point(s * dp.x, s * dp.y);
    }

    // Compute area and x moment
    calc() {
        let arclen = 0;
        let area = 0;
        let moment_x = 0;
        const co = GAUSS_LEGENDRE_COEFFS_32;
        for (let i = 0; i < co.length; i += 2) {
            const t = 0.5 * (1 + co[i + 1]);
            const wi = co[i];
            const dp = this.eval_deriv(t);
            const p = this.eval(t);
            const d_area = wi * dp.x * p.y;
            arclen += wi * dp.hypot();
            area += d_area;
            moment_x += p.x * d_area; 
        }
        return {'arclen': 0.5 * arclen, 'area': 0.5 * area, 'mx': 0.5 * moment_x };
    }

    sample_pts(n) {
        const result = [];
        let arclen = 0;
        // Probably overkill, but keep it simple
        const co = GAUSS_LEGENDRE_COEFFS_32;
        const dt = 1 / (n + 1);
        for (let i = 0; i < n; i++) {
            for (let j = 0; j < co.length; j += 2) {
                const t = dt * (i + 0.5 + 0.5 * co[j + 1]);
                arclen += co[j] * this.eval_deriv(t).hypot();
            }
            const t = dt * (i + 1);
            const d = this.eval_offset(t);
            const p = this.c.eval(t).plus(d);
            result.push({'arclen': arclen * 0.5 * dt, 'p': p, 'd': d});
        }
        return result;
    }

    rotate_to_x() {
        const p0 = this.c.p0().plus(this.eval_offset(0));
        const p1 = this.c.p3().plus(this.eval_offset(1));
        const th = p1.minus(p0).atan2();
        const a = Affine.rotate(-th);
        const ct = CubicBez.from_pts(
            a.apply_pt(this.c.p0().minus(p0)),
            a.apply_pt(this.c.p1().minus(p0)),
            a.apply_pt(this.c.p2().minus(p0)),
            a.apply_pt(this.c.p3().minus(p0))
        );
        const co = new CubicOffset(ct, this.d);
        return {'c': co, 'th': th, 'p0': p0};
    }

    // Error evaluation logic from Tiller and Hanson.
    est_cubic_err(cu, samples, tolerance) {
        let err = 0;
        let tol2 = tolerance * tolerance;
        for (let sample of samples) {
            let best_err = null;
            // Project sample point onto approximate curve along normal.
            let samples = cu.intersect_ray(sample.p, sample.d);
            if (samples.length == 0) {
                // In production, if no rays intersect we probably want
                // to reject this candidate altogether. But we sample the
                // endpoints so you can get a plausible number.
                samples = [0, 1];
            }
            for (let t of samples) {
                const p_proj = cu.eval(t);
                const this_err = sample.p.minus(p_proj).hypot2();
                if (best_err === null || this_err < best_err) {
                    best_err = this_err;
                }
            }
            err = Math.max(err, best_err);
            if (err > tol2) {
                break;
            }
        }
        return Math.sqrt(err);
    }

    cubic_approx(tolerance, sign) {
        const r = this.rotate_to_x();
        const end_x = r.c.c.c[6] + r.c.eval_offset(1).x;
        const metrics = r.c.calc();
        const arclen = metrics.arclen;
        const th0 = Math.atan2(sign * r.c.q.y0, sign * r.c.q.x0);
        const th1 = -Math.atan2(sign * r.c.q.y2, sign * r.c.q.x2);
        const ex2 = end_x * end_x;
        const ex3 = ex2 * end_x;
        const cands = cubic_fit(th0, th1, metrics.area / ex2, metrics.mx / ex3);
        const c = new Float64Array(6);
        const cx = end_x * Math.cos(r.th);
        const sx = end_x * Math.sin(r.th);
        c[0] = cx;
        c[1] = sx;
        c[2] = -sx;
        c[3] = cx;
        c[4] = r.p0.x;
        c[5] = r.p0.y;
        const a = new Affine(c);
        const samples = this.sample_pts(10);
        let best_c = null;
        let best_err;
        let errs = [];
        for (let raw_cand of cands) {
            const cand = a.apply_cubic(raw_cand);
            const err = this.est_cubic_err(cand, samples, tolerance);
            errs.push(err);
            if (best_c === null || err < best_err) {
                best_err = err;
                best_c = cand;
            }
        }
        //console.log(errs);
        if (best_c === null) {
            return null;
        }
        return {'c': best_c, 'err': best_err};
    }

    cubic_approx_other(conf, sign) {
        let c;
        if (conf.method == 'T-H') {
            c = this.tiller_hanson();
        } else if (conf.method == 'Shape') {
            c = this.shape_control();
        }
        if (c === null) {
            return null;
        }
        const samples = this.sample_pts(10);
        const err = this.est_cubic_err(c, samples, conf.tolerance);
        return {'c': c, 'err': err};
    }

    cubic_approx_seq(conf, sign) {
        let approx;
        if (conf.method == 'Fit') {
            approx = this.cubic_approx(conf.tolerance, sign);
        } else {
            approx = this.cubic_approx_other(conf, sign);
        }
        if (approx !== null && approx.err <= conf.tolerance) {
            return [approx.c];
        } else {
            const co0 = this.subsegment(0, 0.5);
            const co1 = this.subsegment(0.5, 1);
            const seq0 = co0.cubic_approx_seq(conf, sign);
            const seq1 = co1.cubic_approx_seq(conf, sign);
            return seq0.concat(seq1);
        }
    }

    subsegment(t0, t1) {
        const cu = this.c.subsegment(t0, t1);
        return new CubicOffset(cu, this.d);
    }

    tiller_hanson() {
        const q = this.c.deriv();
        const d0 = this.eval_offset(0);
        const d1 = this.eval_offset(1);
        const p0 = this.c.p0().plus(d0);
        const p3 = this.c.p3().plus(d1);
        const c_p1 = this.c.p1();
        const c_p2 = this.c.p2();
        const d12 = c_p2.minus(c_p1);
        const s = this.d / d12.hypot();
        const pm = new Point(c_p1.x - s * d12.y, c_p1.y + s * d12.x);
        const pm2 = new Point(c_p2.x - s * d12.y, c_p2.y + s * d12.x);
        const p1 = ray_intersect(p0, q.eval(0), pm, d12);
        const p2 = ray_intersect(p3, q.eval(1), pm, d12);
        return CubicBez.from_pts(p0, p1, p2, p3);
    }

    shape_control() {
        const c = this.c.c;
        const q = this.c.deriv();
        const p0 = this.c.p0().plus(this.eval_offset(0));
        const p3 = this.c.p3().plus(this.eval_offset(1));
        const p = this.eval(0.5);
        const a11 = c[2] - c[0];
        const a12 = c[4] - c[6];
        const a21 = c[3] - c[1];
        const a22 = c[5] - c[7];
        const b1 = (8. / 3) * (p.x - 0.5 * (p0.x + p3.x));
        const b2 = (8. / 3) * (p.y - 0.5 * (p0.y + p3.y));
        const det = a11 * a22 - a12 * a21;
        if (det == 0) {
            return null;
        }
        const a = (b1 * a22 - a12 * b2) / det;
        const b = (a11 * b2 - b1 * a21) / det;
        const p1 = new Point(p0.x + a * a11, p0.y + a * a21);
        const p2 = new Point(p3.x + b * a12, p3.y + b * a22);
        return CubicBez.from_pts(p0, p1, p2, p3);
    }
}

function cubic_seq_to_svg(cu_seq) {
    const c0 = cu_seq[0].c;
    let str = `M${c0[0]} ${c0[1]}`;
    for (cu of cu_seq) {
        const ci = cu.c;
        str += `C${ci[2]} ${ci[3]} ${ci[4]} ${ci[5]} ${ci[6]} ${ci[7]}`;
    }
    return str;
}

function cubic_seq_to_svg_handles(cu_seq) {
    let str = '';
    for (cu of cu_seq) {
        const ci = cu.c;
        str += `M${ci[0]} ${ci[1]}L${ci[2]} ${ci[3]}M${ci[4]} ${ci[5]}L${ci[6]} ${ci[7]}`;
    }
    return str;
}

/// Returns an array of candidate cubics matching given metrics.
function cubic_fit(th0, th1, area, mx) {
    //console.log(th0, th1, area, mx);
    const c0 = Math.cos(th0);
    const s0 = Math.sin(th0);
    const c1 = Math.cos(th1);
    const s1 = Math.sin(th1);
    const a4 = -9
        * c0
        * (((2 * s1 * c1 * c0 + s0 * (2 * c1 * c1 - 1)) * c0 - 2 * s1 * c1) * c0
            - c1 * c1 * s0);
    const a3 = 12
        * ((((c1 * (30 * area * c1 - s1) - 15 * area) * c0 + 2 * s0
            - c1 * s0 * (c1 + 30 * area * s1))
            * c0
            + c1 * (s1 - 15 * area * c1))
            * c0
            - s0 * c1 * c1);
    const a2 = 12
        * ((((70 * mx + 15 * area) * s1 * s1 + c1 * (9 * s1 - 70 * c1 * mx - 5 * c1 * area))
            * c0
            - 5 * s0 * s1 * (3 * s1 - 4 * c1 * (7 * mx + area)))
            * c0
            - c1 * (9 * s1 - 70 * c1 * mx - 5 * c1 * area));
    const a1 = 16
        * (((12 * s0 - 5 * c0 * (42 * mx - 17 * area)) * s1
            - 70 * c1 * (3 * mx - area) * s0
            - 75 * c0 * c1 * area * area)
            * s1
            - 75 * c1 * c1 * area * area * s0);
    const a0 = 80 * s1 * (42 * s1 * mx - 25 * area * (s1 - c1 * area));
    //console.log(a0, a1, a2, a3, a4);
    let roots;
    const EPS = 1e-12;
    if (Math.abs(a4) > EPS) {
        const a = a3 / a4;
        const b = a2 / a4;
        const c = a1 / a4;
        const d = a0 / a4;
        const quads = factor_quartic_inner(a, b, c, d, false);
        /*
        const solved = solve_quartic(a0, a1, a2, a3, a4);
        for (let x of solved) {
            const y = (((a4 * x + a3) * x + a2) * x + a1) * x + a0;
            console.log(x, y);
        }
        */
        roots = [];
        for (let i = 0; i < quads.length; i += 2) {
            const c1 = quads[i];
            const c0 = quads[i + 1];
            const q_roots = solve_quadratic(c0, c1, 1);
            if (q_roots.length > 0) {
                roots = roots.concat(q_roots)
            } else {
                // Real part of pair of complex roots
                roots.push(-0.5 * c1);
            }
        }
    } else {
        // Question: do we ever care about complex roots in these cases?
        if (Math.abs(a3) > EPS) {
            roots = solve_cubic(a0, a1, a2, a3)
        } else {
            roots = solve_quadratic(a0, a1, a2);
        }
    }
    const s01 = s0 * c1 + s1 * c0;
    //console.log(roots);
    const cubics = [];
    for (let d0 of roots) {
        let d1 = (2 * d0 * s0 - area * (20 / 3.)) / (d0 * s01 - 2 * s1);
        if (d0 < 0) {
            d0 = 0;
            d1 = s0 / s01;
        } else if (d1 < 0) {
            d0 = s1 / s01;
            d1 = 0;
        }
        if (d0 >= 0 && d1 >= 0) {
            const c = new Float64Array(8);
            c[2] = d0 * c0;
            c[3] = d0 * s0;
            c[4] = 1 - d1 * c1;
            c[5] = d1 * s1;
            c[6] = 1;
            cubics.push(new CubicBez(c));
        }
    }
    return cubics;
}

// One manipulable cubic bezier
class CubicUi {
    constructor(ui, pts) {
        this.ui = ui
        this.pts = pts;
        this.curve = ui.make_stroke();
        this.curve.classList.add("quad");
        this.hull = ui.make_stroke();
        this.hull.classList.add("hull");
        this.handles = [];
        for (let pt of pts) {
            this.handles.push(ui.make_handle(pt));
        }
    }

    onPointerDown(e) {
        const pt = this.ui.getCoords(e);
        const x = pt.x;
        const y = pt.y;
        for (let i = 0; i < this.pts.length; i++) {
            if (Math.hypot(x - this.pts[i].x, y - this.pts[i].y) < 10) {
                this.current_obj = i;
                return true;
            }
        }
        return false;
    }

    onPointerMove(e) {
        const i = this.current_obj;
        const pt = this.ui.getCoords(e);
        this.pts[i] = pt;
        this.handles[i].setAttribute("cx", pt.x);
        this.handles[i].setAttribute("cy", pt.y);
    }

    getCubic() {
        const p0 = this.pts[0];
        const p1 = this.pts[1];
        const p2 = this.pts[2];
        const p3 = this.pts[3];
        let c = new Float64Array(8);
        c[0] = p0.x;
        c[1] = p0.y;
        c[2] = p1.x;
        c[3] = p1.y;
        c[4] = p2.x;
        c[5] = p2.y;
        c[6] = p3.x;
        c[7] = p3.y;
        return new CubicBez(c);
    }

    update() {
        const cb = this.getCubic();
        const pts = this.pts;
        this.curve.setAttribute("d", cb.to_svg_path());
        const h = `M${pts[0].x} ${pts[0].y}L${pts[1].x} ${pts[1].y}M${pts[2].x} ${pts[2].y}L${pts[3].x} ${pts[3].y}`;
        this.hull.setAttribute("d", h);
    }
}

class OffsetUi {
    constructor(id) {
        const n_cubics = 2;
        this.root = document.getElementById(id);

        this.root.addEventListener("pointerdown", e => {
            this.root.setPointerCapture(e.pointerId);
            this.onPointerDown(e);
            e.preventDefault();
            e.stopPropagation();
        });
        this.root.addEventListener("pointermove", e => {
            this.onPointerMove(e);
            e.preventDefault();
            e.stopPropagation();
        });
        this.root.addEventListener("pointerup", e => {
            this.root.releasePointerCapture(e.pointerId);
            this.onPointerUp(e);
            e.preventDefault();
            e.stopPropagation();
        });
        document.getElementById('d').addEventListener('input', e => this.update());
        document.getElementById('tol').addEventListener('click', e => this.click_tol());
        document.getElementById('alg').addEventListener('click', e => this.click_alg());
        window.addEventListener("keydown", e => this.onKeyDown(e));

        const pts_foo = [new Point(67, 237), new Point(374, 471), new Point(321, 189), new Point(633, 65)];
        this.cubic_foo = new CubicUi(this, pts_foo);
        this.xs = [200, 600];
        this.quad = this.make_stroke();
        this.quad.classList.add("quad");
        this.approx_offset = this.make_stroke();
        this.approx_handles = this.make_stroke();
        this.approx_handles.classList.add("approx_handle");
        this.n_label = this.make_text(500, 55);
        this.type_label = this.make_text(90, 55);
        this.type_label.setAttribute("text-anchor", "middle");
        this.thresh_label = this.make_text(210, 55);
        this.pips = [];
        this.method = 'Fit';
        this.grid = 20;
        this.tolerance = 1;
        this.renderGrid(true);
        this.update();

        this.current_obj = null;
    }

    getCoords(e) {
        const rect = this.root.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;
        return new Point(x, y);
    }

    onPointerDown(e) {
        const pt = this.getCoords(e);
        const x = pt.x;
        const y = pt.y;
        if (this.cubic_foo.onPointerDown(e)) {
            this.current_obj = 'cubic';
            return;
        }
    }

    onPointerMove(e) {
        // Maybe use object oriented dispatch?
        if (this.current_obj == 'cubic') {
            this.cubic_foo.onPointerMove(e);
            this.update();
        }
        const pt = this.getCoords(e);
    }

    onPointerUp(e) {
        this.current_obj = null;
    }

    onKeyDown(e) {
        if (e.key == 's') {
            this.method = "sederberg";
            this.update();
        } else if (e.key == 'r') {
            this.method = "recursive";
            this.update();
        } else if (e.key == 'a') {
            this.method = "analytic";
            this.update();
        } else if (e.key == 'w') {
            this.method = "wang";
            this.update();
        }
    }

    renderGrid(visible) {
        let grid = document.getElementById("grid");
        //this.ui.removeAllChildren(grid);
        if (!visible) return;
        let w = 700;
        let h = 500;
        for (let i = 0; i < w; i += this.grid) {
            let line = document.createElementNS(svgNS, "line");
            line.setAttribute("x1", i);
            line.setAttribute("y1", 0);
            line.setAttribute("x2", i);
            line.setAttribute("y2", h);
            grid.appendChild(line);
        }
        for (let i = 0; i < h; i += this.grid) {
            let line = document.createElementNS(svgNS, "line");
            line.setAttribute("x1", 0);
            line.setAttribute("y1", i);
            line.setAttribute("x2", w);
            line.setAttribute("y2", i);
            grid.appendChild(line);
        }
    }

    make_handle(p) {
        const circle = this.plot(p.x, p.y, "blue", 4);
        circle.classList.add("handle");
        return circle;
    }

    make_stroke() {
        const path = document.createElementNS(svgNS, "path");
        path.setAttribute("fill", "none");
        path.setAttribute("stroke", "blue");
        this.root.appendChild(path);
        return path;
    }

    make_clip_path(id) {
        const clip_path = document.createElementNS(svgNS, "clipPath");
        clip_path.setAttribute("id", id)
        const path = document.createElementNS(svgNS, "path");
        this.root.appendChild(clip_path);
        clip_path.appendChild(path);
        return path;
    }

    make_text(x, y) {
        const text = document.createElementNS(svgNS, "text");
        text.setAttribute("x", x);
        text.setAttribute("y", y);
        this.root.appendChild(text);
        return text;
    }

    plot(x, y, color = "black", r = 2) {
        let circle = document.createElementNS(svgNS, "circle");
        circle.setAttribute("cx", x);
        circle.setAttribute("cy", y);
        circle.setAttribute("r", r);
        circle.setAttribute("fill", color)
        this.root.appendChild(circle);
        return circle;
    }

    click_tol() {
        const vals = [1, 0.1, 0.01, 0.001, 1e9, 10];
        let tol = 1;
        for (let i = 0; i < vals.length - 1; i++) {
            if (this.tolerance == vals[i]) {
                tol = vals[i + 1];
                break;
            }
        }
        this.tolerance = tol;
        document.getElementById('tol').value = tol == 1e9 ? '\u221e' : `${tol}`;
        this.update();
    }

    click_alg() {
        let alg = 'Fit';
        if (this.method == 'Fit') {
            alg = 'T-H';
        } else if (this.method == 'T-H') {
            alg = 'Shape';
        }
        this.method = alg;
        document.getElementById('alg').value = alg;
        this.update();
    }

    update() {
        for (let pip of this.pips) {
            pip.remove();
        }
        this.pips = [];

        const cb = this.cubic_foo.getCubic();
        const conf = {
            'd': document.getElementById('d').value,
            'tolerance': this.tolerance,
            'method': this.method,
        };
        const cusps = cb.find_offset_cusps(conf.d);
        this.cubic_foo.update();
        const c_off = new CubicOffset(cb, conf.d);
        //console.log(c_off.sample_pts(10));
        /*
        const approx = c_off.cubic_approx();
        const c = approx.c;
        this.approx_offset.setAttribute('d', approx.c.to_svg_path());
        this.type_label.textContent = `${approx.err}`;
        const z = c.c;
        const h = `M${z[0]} ${z[1]}L${z[2]} ${z[3]}M${z[6]} ${z[7]}L${z[4]} ${z[5]}`;
        this.approx_handles.setAttribute('d', h);
        */
        let seq = [];
        for (let cusp of cusps) {
            const co_seg = c_off.subsegment(cusp.t0, cusp.t1);
            seq = seq.concat(co_seg.cubic_approx_seq(conf, cusp.sign));
        }
        this.approx_offset.setAttribute('d', cubic_seq_to_svg(seq));
        this.approx_handles.setAttribute('d', cubic_seq_to_svg_handles(seq));
        this.type_label.textContent = `subdivisions: ${seq.length}`;
    }
}

new OffsetUi("s");
</script>

The problem of [parallel][parallel curve] or offset curves has remained challenging for a long time. Parallel curves have applications in 2D graphics (for drawing strokes and also adding weight to fonts), and also robotic path planning and manufacturing, among others. The exact offset curve of a cubic Bézier can be described (it is an analytic curve of degree 10) but it not tractable to work with. Thus, in practice the approach is almost always to compute an approximation to the true parallel curve. A single cubic Bézier might not be a good enough approximation to the parallel curve of the source cubic Bézier, so in those cases it is sudivided into multiple Bézier segments.

A number of algorithms have been published, of varying quality. Many popular algorithms aren't very accurate, yielding either visually incorrect results or excessive subdivision, depending on how carefully the error metric has been implemented. This blogpost gives a practical implementation of a nearly optimal result. Essentially, it tries to find *the* cubic Bézier that's closest to the desired curve. To this end, we take a curve-fitting approach and apply an array of numerical techniques to make it work. The result is a visibly more accurate curve even when only one Bézier is used, and a minimal number of subdivisions when a tighter tolerance is applied. In fact, we claim $O(n^6)$ scaling: if a curve is divided in half, the error of the approximation will decrease by a factor of 64. I suggested a previous approach, [Cleaner parallel curves with Euler spirals], with $O(n^4)$ scaling, in other words only a 16-fold reduction of error.

Though there are quite a number of published algorithms, the need for a really good solution remains strong. Some really good reading is the [Paper.js issue] on adding an offset function. After much discussion and prototyping, there is still no consensus on the best approach, and the feature has not landed in Paper.js despite obvious demand. There's also some interesting discussion of stroking in an [issue in the COLRv1 spec repo](https://github.com/googlefonts/colr-gradients-spec/issues/276).

## Outline of approach

The fundamental concept is *curve fitting,* or finding the parameters for a cubic Bézier that most closely approximate the desired curve. We also employ a sequence of numerical techniques in support of that basic concept:

* Finding the cusps and subdividing the curve at the cusp points.
* Computing area and moment of the target curve
  + Green's theorem to convert double integral into a single integral
  + Gauss-Legendre quadrature for efficient numerical integration
* Quartic root finding to solve for cubic Béziers with desired area and moment
* Measure error to choose best candidate and decide whether to subdivide
  + Cubic Bézier/ray intersection

Each of these numeric techniques has its own subtleties.

## Cusp finding

One of the challenges of parallel curves in general is *cusps.* These happen when the curvature of the source curve is equal to one over the offset distance. Cubic Béziers have fairly complex curvature profiles, so there can be a number of cusps - it's easy to find examples with four, and it wouldn't be surprising to me if there were more. By contrast, Euler spirals have simple curvature profiles, and the location of the cusp is extremely simple to determine.

The general equation for curvature of a parametric curve is as follows:

$$
\kappa = \frac{\mathbf{x}''(t) \times \mathbf{x}'(t)}{|\mathbf{x}'(t)|^3}
$$

The cusp happens when $\kappa d + 1 = 0$. With a bit of rewriting, we get

$$
(\mathbf{x}''(t) \times \mathbf{x}'(t))d + |\mathbf{x}'(t)|^3 = 0
$$

As with many such numerical root-finding approaches, missing a cusp is a risk. The approach *currently* used in the code in this blog post is a form of interval arithmetic: over the (t0..t1) interval, a minimum and maximum value of $\|\mathbf{x}'\|$ is computed, while the cross product is quadratic in t. Solving that partitions the interval into ranges where the curvature is definitely above or below the threshold for a cusp, and a (hopefully) smaller interval where it's possible.

This algorithm is robust, but convergence is not super-fast - it often hits the case where it has to subdivide in half, so convergence is similar to a bisection approach for root-finding. I'm exploring another approach of computing bounding parabolas, and that seems to have cubic convergence, but is a bit more complicated and fiddly.

In cases where you *know* you have one simple cusp, a simple and generic root-finding method like ITP (about more which below) would be effective. But that leaves the problem of detecting when that's the case. Robust detection of possible cusps generally also gives the locations of the cusps when iterated.

## Computing area and moment of the target curve

The primary input to the curve fitting algorithm is a set of parameters for the curve. Not control points of a Bézier, but other measurements of the curve. The position of the endpoints and the tangents can be determined directly, which, just counting parameters, leaves two free. Those are the area and x-moment. These are generally described as integrals. For an arbitrary parametric curve (a family which easily includes offsets of Béziers), Green's theorem is a powerful and efficient technique for approximating these integrals.

For area, the specific instance of Green's theorem we're looking for is this. Let the curve be defined as x(t) and y(t), where t goes from 0 to 1. Let D be the region enclosed by the curve. If the curve is closed, then we have this relation:

$$
\iint_D dx \,dy = \int_0^1 y(t)\, x'(t)\, dt
$$

I won't go into the details here, but all this still works even when the curve is open (one way to square up the accounting is to add the return path of the chord, from the end point back to the start), and when the area contains regions of both positive and negative signs, which can be the case for S-shaped curves. The x moment is also very similar and just involves an additional $x$ term:

$$
\iint_D x \, dx \,dy = \int_0^1 x(t)\, y(t)\, x'(t)\, dt
$$

Especially given that the function being integrated is (mostly) smooth, the best way to compute the approximate integral is [Gauss-Legendre quadrature], which has an extremely simple implementation: it's just the dot product between a vector of weights and a vector of the function sampled at certain points, where the weights and points are carefully chosen to minimize error; in particular they result in zero error when the function being integrated is a polynomial of order up to that of the number of samples. The JavaScript code on this page just uses a single quadrature of order 32, but a more sophisticated approach (as is used for arc length computation) would be to first estimate the error and then choose a number of samples based on that.

Note that the area and moments of a cubic Bézier curve can be efficiently computed analytically and don't need an approximate numerical technique. Adding in the offset term is numerically similar to an arc length computation, bringing it out of the range where analytical techniques are effective, but fortunately similar numerical techniques as for computing arc length are effective.

### Refinement of curve fitting approach

The basic approach to curve fitting was described in [Fitting cubic Bézier curves]. Those ideas are good, but there were some rough edges to be filled in and other refinements.

To recap, the goal is to find the closest Bézier, in the space of all cubic Béziers, to the desired curve (in this case the parallel curve of a source Bézier, but the curve fitting approach is general). That's a large space to search, but we can immediately nail down some of the parameters. The endpoints should definitely be fixed, and we'll also set the tangent angles at the endpoints to match the desired curve.

One loose end was the solving technique. My prototype code used numerical methods, but I've now settled on root finding of the quartic equation. A major reason for that is that I've found that the quartic solver in the [Orellana and De Michele] paper works well - it is fast, robust, and stable. The JavaScript code on this page uses a fairly direct implementation of that (which I expect may be useful for other applications - all the code on this blog is licensed under Apache 2, so feel free to adapt it within the terms of that license).

Another loose end was the treatment of "near misses." Those happen when the function comes close to zero but doesn't quite cross it. In terms of roots of a polynomial, those are a conjugate pair of complex roots, and I take the real part of that as a candidate. It would certainly be possible to express this logic by having the quartic solver output complex roots as well as real ones, but I found an effective shortcut: the algorithm actually factors the original quartic equation into two quadratics, one of which always has real roots and the other some of the time, and finding the real part of the roots of a quadratic is trivial (it's just -b/2a).

Recently, Cem Yuksel has proposed a variation of Newton-style [polynomial solving]. It's likely this could be used, but there were a few reasons I went with the more analytic approach. For one, I want multiple roots and this works best when only one is desired. Second, it's hard to bound a priori the interval to search for roots. Third, it's not easy to get the complex roots (if you did want to do this, the best route is probably deflation). Lastly, the accuracy numbers don't seem as good (the Orellana and De Michele paper presents the results of very careful testing), and in empirical testing I have found that accuracy in root finding is a real problem that can affect the quality of the final results. A Rust implementation of the Orellana and De Michele technique clocks in at 390ns on an M1 Max, which certainly makes it competitive with the fastest techniques out there.

The last loose end was the treatment of near-zero slightly negative arm lengths. These are roots of the polynomial but are not acceptable candidate curves, as the tangent would end up pointing the wrong way. My original thought was to clamp the relevant length to zero (on the basis that it is an acceptable curve that is "nearby" the numerical solution), but that also doesn't give ideal results. In particular, if you set one length to zero and set the other one based on exact signed area, the tangent at the zero-length side might be wrong (enough to be visually objectionable). After some experimentation, I've decided to set the other control point to be the intersection of the tangents, which gets tangents right but possibly results in an error in area, depending on the exact parameters. The general approach is to throw these as candidates into the mix, and let the error measurement sort it out.

### Error measurement

A significant amount of total time spent in the algorithm is measuring the distance between the exact curve and the cubic approximation, both to decide when to subdivide and also to choose between multiple candidates from the Bézier fitting. I implemented the technique from Tiller and Hanson and found it to work well. They sample the exact curve at a sequence of points, then for each of those points project that point onto the approximation along the normal. That is equivalent to computing the intersection of a ray and a cubic Bézier. The maximum distance between the projected and true point is the error. This is a fairly good approximation to the [Fréchet distance] but significantly cheaper to compute.

Computing the intersection of a ray and a cubic Bézier is equivalent to finding the root of a cubic polynomial, a challenging numerical problem in its own right. In the course of working on this, I found that the cubic solver in kurbo would sometimes report inaccurate results (especially when the coefficient on the $x^3$ term was near-zero, which can easily happen when cubic Bézier segments are near raised quadratics), and so implemented a [better cubic solver] based on a blog post on [cubics by momentsingraphics]. That's still not perfect, and there is more work to be done to arrive at a gold-plated cubic solver. The Yuksel [polynomial solving] approach might be a good fit for this, especially as you only care about results for t strictly within the (0..1) range. It might also be worth pointing out that the fma instruction used in the Rust implementation is not available in JavaScript, so the accuracy of the solver here won't be quite as good.

The error metric is a critical component of a complete offset curve algorithm. It accounts for a good part of the total CPU time, and also must be accurate. If it underestimates true error, it risks letting inaccurate results slip through. If it overestimates error, it creates excessive subdivision. Incidentally, I suspect that the error measurement technique in the Elber, Lee and Kim paper (cited below) may be flawed; it seems like it may overestimate error in the case where the two curves being compared differ in parametrization, which will happen commonly with offset problems, particularly near cusps. The Tiller-Hanson technique is largely insensitive to parametrization (though perhaps more care should be taken to ensure that the sample points are actually evenly spaced).

### Subdivision

Right now the subdivision approach is quite simple: if none of the candidate cubic Béziers meet the error bound, then the curve is subdivided at t = 0.5 and each half is fit. The scaling is n^6, so in general that reduces the error by a factor of 64.

If generation of an absolute minimum number of output segments is the goal, then a smarter approach to choosing subdivisions would be in order. For absolutely optimal results, in general what you want to do is figure out the minimum number of subdivisions, then adjust the subdivision points so the error of all segments are equal. This technique is described in section 9.6.3 of my [thesis]. In the limit, it can be expected to reduce the number of subdivisions by a factor of 1.5 compared with "subdivide in half," but not a significant improvement when most curves can be rendered with one or two cubic segments.

## Evaluation

Somebody evaluating this work for use in production would care about several factors: accuracy of result, robustness, and performance. The interactive demo on this page speaks for itself: the results are accurate, the performance is quite good for interactive use, and it is robust (though I make no claims it handles all adversarial inputs correctly; that always tends to require extra work).

In terms of accuracy of result, this work is a dramatic advance over anything in the literature. I've implemented and compared it against two other techniques that are widely cited as reasonable approaches to this problem: [Tiller-Hanson] and the "shape control" approach of Yang and Huang. For generating a single segment, it can be considerably more accurate than either.

<img src="/assets/parallel-compare.png" width="870" alt="comparison against other approaches">

In addition to the accuracy for generating a single line segment, it is interesting to compare the scaling as the number of subdivisions increases, or as the error tolerance decreases. These tend to follow a power law. For this technique, it is $O(n^6)$, meaning that subdividing a curve in half reduces the error by a factor of 64. For the shape control approach, it is $O(n^5)$, and for Tiller-Hanson is is $O(n^2)$. That last is a surprisingly poor result, suggesting that it is only a constant factor better than subdividing the curves into lines.

<img src="/assets/parallel-scaling.svg" width="683" alt="chart showing scaling behavior">

The shape control technique has good scaling, but stability issues when the tangents are nearly parallel. That can happen for an S-shaped curve, and also for a U with nearly 180 degrees of arc.

The Tiller-Hanson technique is geometrically intuitive; it offsets each edge of the control polygon by the offset amount, as illustrated in the diagram below. It doesn't have the stability issues with nearly-parallel tangents and can produce better results for those "S" curves, but the scaling is much worse.

<img src="/assets/parallel-tiller-hanson.png" width="560" height="250" alt="diagram showing Tiller-Hanson technique">

Regarding performance, I have preliminary numbers from the JavaScript implementation, about 12µs per curve segment generated on an M1 Max running Chrome. I am quite happy with this result, and of course expect the Rust implementation to be even faster when it's done. There are also significant downstream performance improvements from generating highly accurate results; every cubic segment you generate has some cost to process and render, so the fewer of those, the better.

I haven't implemented all the techniques in the Elber, Lee and Kim paper, but it is possible to draw some tentative conclusions from the literature. I expect the Klass technique (and its numerical refinement by Sakai and Suenaga) to have good scaling but relatively poor acccuracy for a single segment. The Klass technique is also documented to have poor numerical stability, thanks in part to its reliance on Newton solving techniques. The Hoschek and related (least-squares) approaches will likely produce good results but are quite slow (the Yang and Huang paper reports an eye-popping 49s for calculating a simple case with .001 tolerance, of course on older hardware).

The Euler spiral technique in my previous blog post will in general produce considerably more subdivision (with $O(n^4)$ scaling), but perhaps it would be premature to write it off completely. Once the curve is in piecewise Euler spiral form, a result within the given error bounds can be computed directly, with no need to explicitly evaluate an error metric. In addition, the cusps are located robustly with trivial calculation. That said, getting a curve *into* piecewise Euler spiral form is still challenging, and my prototype code uses a rather expensive error metric to achieve that.

## Discussion

This post presents a significantly better solution to the parallel curve problem than the current state of the art. It is accurate, robust, and fast. It should be suitable to implement in interactive vector graphics applications, font compilation pipelines, and other contexts.

While parallel curve is an important application, the curve fitting technique is quite general. It can be adapted to generalized strokes, for example where the stroke width is variable, path simplification, distortions and other transforms, conversion from other curve representations, accurate plotting of functions, and I'm sure there are other applications. Basically the main thing that's required is the ability to evaluate area and moment of the source curve, and ability to evaluate the distance to that curve (which can be done readily enough by sampling a series of points with their arc lengths).

This work also provides a bit of insight into the nature of cubic Bézier curves. The $O(n^6)$ scaling provides quantitative support to the idea that cubic Bézier curves are extremely expressive; with skillful placement of the control points, they can extremely accurately approximate a wide variety of curves. Parallel curves are challenging for a variety of reasons, including cusps and sudden curvature variations. That said, they do require skill, as geometrically intuitive but unoptimized approaches to setting control points (such as Tiller-Hanson) perform poorly.

There's clearly more work that could be done to make the evalation more rigorous, including more optimization of the code. I believe this result would make a good paper, but my bandwidth for writing papers is limited right now. I would be more than open to collaboration, and invite interested people to get in touch.

Thanks to Linus Romer for helpful discussion and refinement of the polynomial equations regarding quartic solving of the core curve fitting algorithm.

Discuss on [Hacker News](https://news.ycombinator.com/item?id=32784491).

## References

Here is a bibliography of some relevant academic papers on the topic.

* [An offset spline approximation for plane cubic splines](https://www.sciencedirect.com/science/article/abs/pii/0010448583900192), Klass, 1983
* [Offsets of Two-Dimensional Profiles](https://ieeexplore.ieee.org/iel5/38/4055906/04055919), Tiller and Hanson, 1984
* [High Accuracy Geometric Hermite Interpolation](https://www.sciencedirect.com/science/article/abs/pii/0167839687900021), de Boor, Höllig, Sabin, 1987 ([PDF cache](https://minds.wisconsin.edu/bitstream/handle/1793/58822/TR692.pdf))
* [Optimal approximate conversion of spline curves and spline approximation of offset curves](https://www.sciencedirect.com/science/article/abs/pii/0010448588900061), Hoschek and Wissel, 1988 ([PDF cache](http://www.norbert-wissel.de/Diplom.pdf))
* [A New Shape Control and Classification for Cubic Bézier Curves](https://link.springer.com/chapter/10.1007/978-4-431-68456-5_17), Yang and Huang, 1993 ([PDF cache](https://github.com/paperjs/paper.js/files/752955/A.New.Shape.Control.and.Classification.for.Cubic.Bezier.Curves.pdf))
* [Comparing offset curve approximation methods](https://ieeexplore.ieee.org/document/586019/), Elber, Lee, Kim, 1997 ([PDF cache](http://3map.snu.ac.kr/mskim/ftp/comparing.pdf))
* [Cubic spline approximation of offset curves of planar cubic splines](https://www.tandfonline.com/doi/abs/10.1080/00207160108805122), Sakai and Suenaga, 2001 ([PDF cache](https://www.kurims.kyoto-u.ac.jp/~kyodo/kokyuroku/contents/pdf/1198-30.pdf))
* [Boosting Efficiency in Solving Quartic Equations with No Compromise in Accuracy](https://dl.acm.org/doi/10.1145/3386241), Orellana and De Michele, 2020 ([PDF cache][Orellana and De Michele])
* [High-Performance Polynomial Root Finding for Graphics](https://dl.acm.org/doi/10.1145/3543865), Yuksel, 2022 ([PDF cache](http://www.cemyuksel.com/research/polynomials/polynomial_roots_hpg2022.pdf))

[parallel curve]: https://en.wikipedia.org/wiki/Parallel_curve
[Paper.js issue]: https://github.com/paperjs/paper.js/issues/371
[Green's theorem]: https://en.wikipedia.org/wiki/Green%27s_theorem
[Orellana and De Michele]: https://cristiano-de-michele.netlify.app/publication/orellana-2020/
[ITP method]: https://en.wikipedia.org/wiki/ITP_method
[Gauss-Legendre quadrature]: https://en.wikipedia.org/wiki/Gauss%E2%80%93Legendre_quadrature
[polynomial solving]: http://www.cemyuksel.com/research/polynomials/
[Fréchet distance]: https://en.wikipedia.org/wiki/Fr%C3%A9chet_distance
[Bézier primer]: https://pomax.github.io/bezierinfo/#offsetting
[Cleaner parallel curves with Euler spirals]: https://raphlinus.github.io/curves/2021/02/19/parallel-curves.html
[Fitting cubic Bézier curves]: https://raphlinus.github.io/curves/2021/03/11/bezier-fitting.html
[Tiller-Hanson]: https://math.stackexchange.com/questions/465782/control-points-of-offset-bezier-curve
[thesis]: https://levien.com/phd/thesis.pdf
[cubics by momentsingraphics]: https://momentsingraphics.de/CubicRoots.html
[better cubic solver]: https://github.com/linebender/kurbo/pull/224
