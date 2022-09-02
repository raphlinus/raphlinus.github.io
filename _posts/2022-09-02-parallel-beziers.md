---
layout: post
title:  "Parallel curves of cubic Béziers"
date:   2022-09-02 15:02:42 -0700
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
	svg .handle {
		pointer-events: all;
	}
	svg .handle:hover {
		r: 6;
	}
	svg .quad {
		stroke-width: 2px;
		stroke: #222;
	}
	svg .hull {
		stroke: #c44;
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
		stroke: #ddd;
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
</style>
<svg id="s" width="700" height="500">
	<g id="grid"></g>
</svg>
<script>
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
    arclen() {
        const c = this.c;
        const d01x = c[2] - c[0];
        const d01y = c[3] - c[1];
        const d12x = c[4] - c[2];
        const d12y = c[5] - c[3];
        const d23x = c[6] - c[4];
        const d23y = c[7] - c[5];
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
        const co = GAUSS_LEGENDRE_COEFFS_32_HALF;
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
        return Math.sqrt(2.25) * sum;
    }

    inv_arclen(s, accuracy) {
        if (s <= 0) {
            return 0;
        }
        const total_arclen = this.arclen();
        if (s >= total_arclen) {
            return 1;
        }
        // For simplicity, measure arclen from 0 rather than stateful delta.
        const f = t => this.subsegment(0, t).arclen() - s;
        const epsilon = accuracy / total_arclen;
        return solve_itp(f, 0, 1, epsilon, 1, 2, -s, total_arclen -s);
    }

    /*
    find_offset_cusps(d) {
        const cusps = [];
        this.find_offset_cusps_rec(d, cusps, 0, 1);
    }

    find_offset_cusps_rec(d, cusps, t0, t1) {
        const q = this.deriv();
        // compute interval for ds/dt, using convex hull of hodograph
        const d1 = tri_sign(q.x0, q.y0, q.x1, q.y1);
        const d2 = tri_sign(q.x1, q.y1, q.x2, q.y2);
        const d3 = tri_sign(q.x2, q.y2, q.x0, q.y0);
        const z = !((d1 < 0 || d2 < 0 || d3 < 0) && (d1 > 0 || d2 > 0 || d3 > 0));
        const ds0 = q.x0 * q.x0 + q.y0 * q.y0;
        const ds1 = q.x1 * q.x1 + q.y1 * q.y1;
        const ds2 = q.x2 * q.x2 + q.y2 * q.y2;
        const max_ds = Math.sqrt(Math.max(ds0, ds1, ds2));
        const min_ds = z ? 0 : Math.sqrt(Math.min(ds0, ds1, ds2));
        console.log('ds interval', min_ds, max_ds);
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
        let cmin = Math.min(c0, c0 + c1 + c2);
        let cmax = Math.max(c0, c0 + c1 + c2);
        const t_crit = -0.5 * c1 / c2;
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
            return;
        }
    }
    */

    // This is a brute-force solution; a more robust one is started above.
    find_offset_cusps(d) {
        const result = [];
        const d_k = 1 / d;
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
        for (let i = 0; i <= n; i++) {
            const t = i / n;
            const ds2 = q.eval(t).hypot2();
            const k = (((c2 * t + c1) * t) + c0) / (ds2 * Math.sqrt(ds2));
            const yb = k + d_k;
            if (i != 0) {
                if (ya * yb < 0) {
                    let tx = (yb * last_t - ya * t) / (yb - ya);
                    result.push(tx);
                }
            }
            ya = yb;
            last_t = t;
        }
        return result;
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
            const p = this.eval(dt * (i + 1));
            result.push({'arclen': arclen * 0.5 * dt, 'p': p});
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

    // This is a technique that might be made to work (it is almost certainly
    // faster than the inverse arclength approach), but doesn't perform well
    // in edge cases.
    est_cubic_err(cu) {
        const q = cu.deriv();
        const dx0 = q.x0;
        const dy0 = q.y0;
        const dx1 = 2 * (q.x1 - q.x0);
        const dy1 = 2 * (q.y1 - q.y0);
        const dx2 = (q.x0 - 2 * q.x1 + q.x2);
        const dy2 = (q.y0 - 2 * q.y1 + q.y2);
        const n = 10;
        let err = 0;
        // We're calculating an l2 norm mostly because it should be less
        // sensitive to smaller, but l_inf norm is closer to hausdorff
        let l_inf_err = 0;
        for (let i = 1; i < n; i++) {
            const t = i / n;
            const actual = this.eval(t);
            const dx = this.q.eval(t);
            const c0 = dx0 * dx.y - dy0 * dx.x;
            const c1 = dx1 * dx.y - dy1 * dx.x;
            const c2 = dx2 * dx.y - dy2 * dx.x;
            let roots = solve_quadratic(c0, c1, c2);
            if (roots.length == 0) {
                roots = [-0.5 * c1 / c2];
            }
            let best_err = 0;
            let first = true;
            for (let j = 0; j < roots.length; j++) {
                const u = roots[j];
                if (u >= 0 && u <= 1) {
                    const approx = cu.eval(u);
                    //const this_err = actual.minus(approx).hypot2();
                    const d = this.c.eval(t).minus(approx).cross(dx) / dx.hypot();
                    const d_err = this.d - d;
                    const this_err = d_err * d_err;
                    if (first || this_err < best_err) {
                        best_err = this_err;
                        first = false;
                    }
                }
            }
            // Note: if we're really going L_inf, use abs.
            l_inf_err = Math.max(l_inf_err, Math.sqrt(best_err));
            err += best_err;
        }
        //console.log(Math.sqrt(err / (n - 1)), l_inf_err);
        return Math.sqrt(err / (n - 1));
    }

    est_cubic_err_arclen(cu, arclen, samples) {
        const scale = cu.arclen() / arclen;
        let err = 0;
        for (let sample of samples) {
            const s = sample.arclen * scale;
            const t = cu.inv_arclen(s, 1e-3);
            const this_err = cu.eval(t).minus(sample.p).hypot2();
            err = Math.max(err, this_err);
        }
        return Math.sqrt(err);
    }

    cubic_approx(max_arclen_err) {
        const r = this.rotate_to_x();
        const end_x = r.c.c.c[6] + r.c.eval_offset(1).x;
        const metrics = r.c.calc();
        const arclen = metrics.arclen;
        const th0 = Math.atan2(r.c.q.y0, r.c.q.x0);
        const th1 = -Math.atan2(r.c.q.y2, r.c.q.x2);
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
            const cu_arclen = cand.arclen();
            if (Math.abs(cu_arclen - arclen) < max_arclen_err) {
                const err = this.est_cubic_err_arclen(cand, arclen, samples);
                //const err = this.est_cubic_err(cand);
                //console.log(Math.abs(cu_arclen - arclen), err);
                errs.push(err);
                if (best_c === null || err < best_err) {
                    best_err = err;
                    best_c = cand;
                }
            }
        }
        //console.log(errs);
        if (best_c === null) {
            return null;
        }
        return {'c': best_c, 'err': best_err};
    }

    cubic_approx_seq(tolerance) {
        // Todo: empirically determine scaling factor; I think 1 is probably ok.
        const approx = this.cubic_approx(tolerance * 2);
        if (approx !== null && approx.err <= tolerance) {
            return [approx.c];
        } else {
            const co0 = this.subsegment(0, 0.5);
            const co1 = this.subsegment(0.5, 1);
            return co0.cubic_approx_seq(tolerance).concat(co1.cubic_approx_seq(tolerance));
        }
    }

    subsegment(t0, t1) {
        const cu = this.c.subsegment(t0, t1);
        return new CubicOffset(cu, this.d);
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
    roots.push(0);
    if (s0 != 0) {
        roots.push(area * (10. / 3) / s0);
    }
    const s01 = s0 * c1 + s1 * c0;
    //console.log(roots);
    const cubics = [];
    for (let d0 of roots) {
        let d1 = (2 * d0 * s0 - area * (20 / 3.)) / (d0 * s01 - 2 * s1);
        if (Math.abs(d1) < EPS) {
            d1 = 0;
        }
        const c = new Float64Array(8);
        c[2] = d0 * c0;
        c[3] = d0 * s0;
        c[4] = 1 - d1 * c1;
        c[5] = d1 * s1;
        c[6] = 1;
        cubics.push(new CubicBez(c));
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
        window.addEventListener("keydown", e => this.onKeyDown(e));

        const pts_foo = [new Point(200, 450), new Point(400, 450), new Point(500, 100), new Point(600, 50)];
        this.cubic_foo = new CubicUi(this, pts_foo);
        this.xs = [200, 600];
        this.quad = this.make_stroke();
        this.quad.classList.add("quad");
        this.approx_offset = this.make_stroke();
        this.approx_handles = this.make_stroke();
        this.approx_handles.classList.add("approx_handle");
        this.n_label = this.make_text(500, 50);
        this.type_label = this.make_text(90, 50);
        this.type_label.setAttribute("text-anchor", "middle");
        this.thresh_label = this.make_text(210, 50);
        this.pips = [];
        this.method = "analytic";
        this.grid = 20;
        this.thresh = 1.0;
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
        if (x >= 50 && x <= 130 && y >= 30 && y <= 60) {
            if (this.method == "analytic") {
                this.method = "recursive";
            } else {
                this.method = "analytic";
            }
            this.update();
            return;
        }
        if (x >= 200 && x <= 310 && y >= 30 && y <= 60) {
            if (this.thresh == 1.0) {
                this.thresh = 0.5;
            } else if (this.thresh == 0.5) {
                this.thresh = 0.2;
            } else if (this.thresh == 0.2) {
                this.thresh = 0.1;
            } else if (this.thresh == 0.1) {
                this.thresh = 10.0;
            } else if (this.thresh == 10.0) {
                this.thresh = 5.0;
            } else if (this.thresh == 5.0) {
                this.thresh = 2.0;
            } else if (this.thresh == 2.0) {
                this.thresh = 1.0;
            }
            this.update();
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
        const circle = this.plot(p.x, p.y, "blue", 5);
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

    update() {
        for (let pip of this.pips) {
            pip.remove();
        }
        this.pips = [];

        const cb = this.cubic_foo.getCubic();
        const d = 40;
        const cusps = cb.find_offset_cusps(d);
        cusps.splice(0, 0, 0);
        cusps.push(1);
        this.cubic_foo.update();
        const c_off = new CubicOffset(cb, d);
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
        const tolerance = 1;
        let seq = [];
        for (let i = 0; i < cusps.length - 1; i++) {
            const co_seg = c_off.subsegment(cusps[i], cusps[i + 1]);
            seq = seq.concat(co_seg.cubic_approx_seq(tolerance));
        }
        this.approx_offset.setAttribute('d', cubic_seq_to_svg(seq));
        this.approx_handles.setAttribute('d', cubic_seq_to_svg_handles(seq));
        this.type_label.textContent = `subdivisions: ${seq.length}`;
    }
}

new OffsetUi("s");
</script>

This is placeholder text for a blog post to be written on the topic. See [issue 80](https://github.com/raphlinus/raphlinus.github.io/issues/80) in the repo for more info.

