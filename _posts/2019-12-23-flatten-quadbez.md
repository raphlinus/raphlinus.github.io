---
layout: post
title:  "Flattening quadratic Béziers"
date:   2019-12-23 11:05:42 -0800
categories: [graphics, curves]
---
<style>
  svg {
    touch-action: none;
  }
</style>
<script type="text/x-mathjax-config">
        MathJax.Hub.Config({
                tex2jax: {
                        inlineMath: [['$', '$']]
                }
        });
</script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.0/MathJax.js?config=TeX-AMS-MML_HTMLorMML" type="text/javascript"></script>

A classic approach to rendering Bézier curves is to *flatten* them to polylines. There are other possibilities, including working with the Bézier curves analytically, as is done for example in [Random Access Vector Graphics], but converting to polylines still has legs, largely because it's easier to build later stages of a rendering pipeline (especially on a GPU) that work with polylines.

A similarly classic approach to flattening Béziers to polylines is recursive subdivision. Briefly stated, the algorithm measures the error between the chord connecting the endpoints and the curve. If this is within tolerance, it returns the chord. Otherwise, it splits the Bézier in half (using de Casteljau subdivision) and recursively applies the algorithm to the two halves. This process is described in more detail in the paper [Piecewise Linear Approximation]. However, there are several reasons to be dissatisfied with this approach. For one, while it's pretty good, it's not particularly close to optimum in the number of curve segments. Perhaps of greater concern to a modern audience, the recursive approach adapts poorly any form of parallel evaluation, including GPU and SIMD. Allocation is also difficult, as the approach doesn't tell you in advance how many subdivisions will be needed to achieve the specified tolerance.

The interactive demo below lets you switch between the recursive subdivision method and the new technique described in this blog, so you can see how the new technique achieves the error tolerance with significantly fewer segments.

<style>
svg .handle {
    pointer-events: all;
}
svg .handle:hover {
    r: 6;
}
svg .quad {
    stroke-width: 6px;
    stroke: #bbb;
}
svg .hull {
    stroke: #c44;
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
    stroke: #ccc;
}
img {
  margin: auto;
  margin: auto;
  display: block;
}
</style>
<svg id="s" width="700" height="500">
    <g id="grid"></g>
    <rect class="button" x="50" y="30" width="80" height="30" />
    <rect class="button" x="200" y="30" width="110" height="30" />
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

    subsegment(t0, t1) {
        const p0 = this.eval(t0);
        const p2 = this.eval(t1);
        const dt = t1 - t0;
        const p1x = p0.x + (this.x1 - this.x0 + t0 * (this.x2 - 2 * this.x1 + this.x0)) * dt;
        const p1y = p0.y + (this.y1 - this.y0 + t0 * (this.y2 - 2 * this.y1 + this.y0)) * dt;
        return new QuadBez(p0.x, p0.y, p1x, p1y, p2.x, p2.y);
    }

    // The maximum error between a chord and the quadratic Bézier.
    // Note: this isn't quite right for extreme examples.
    error() {
        const x1 = this.x1 - this.x0;
        const y1 = this.y1 - this.y0;
        const x2 = this.x2 - this.x0;
        const y2 = this.y2 - this.y0;
        const t = (x1 * x2 + y1 * y2) / (x2 * x2 + y2 * y2);
        const u = Math.min(Math.max(t, 0), 1);
        const p = new Point(this.x0, this.y0).lerp(new Point(this.x2, this.y2), u);
        return 0.5 * p.dist(new Point(this.x1, this.y1));
    }

    // Determine the x values and scaling to map to y=x^2
    map_to_basic() {
        const ddx = 2 * this.x1 - this.x0 - this.x2;
        const ddy = 2 * this.y1 - this.y0 - this.y2;
        const u0 = (this.x1 - this.x0) * ddx + (this.y1 - this.y0) * ddy;
        const u2 = (this.x2 - this.x1) * ddx + (this.y2 - this.y1) * ddy;
        const cross = (this.x2 - this.x0) * ddy - (this.y2 - this.y0) * ddx;
        const x0 = u0 / cross;
        const x2 = u2 / cross;
        // There's probably a more elegant formulation of this...
        const scale = Math.abs(cross) / (Math.hypot(ddx, ddy) * Math.abs(x2 - x0));
        return {x0: x0, x2: x2, scale: scale, cross: cross};
    }

    recurse_subdiv_inner(tol, t0, t1, result) {
        const q = this.subsegment(t0, t1);
        if (q.error() <= tol) {
            result.push(t1);
        } else {
            const tm = (t0 + t1) * 0.5;
            this.recurse_subdiv_inner(tol, t0, tm, result);
            this.recurse_subdiv_inner(tol, tm, t1, result);
        }
    }

    recurse_subdiv(tol) {
        const result = [0]
        this.recurse_subdiv_inner(tol, 0, 1, result);
        return result;
    }

    // Subdivide using fancy algorithm.
    my_subdiv(tol) {
        const params = this.map_to_basic();
        const a0 = approx_myint(params.x0);
        const a2 = approx_myint(params.x2);
        const count =  0.5 * Math.abs(a2 - a0) * Math.sqrt(params.scale / tol);
        const n = Math.ceil(count);
        const u0 = approx_inv_myint(a0);
        const u2 = approx_inv_myint(a2);
        let result = [0];
        for (let i = 1; i < n; i++) {
            const u = approx_inv_myint(a0 + ((a2 - a0) * i) / n);
            const t = (u - u0) / (u2 - u0);
            result.push(t);
        }
        result.push(1);
        return result;
    }
}

//for (let i = 0; i < sub.length - 1; i++) {
//    console.log(qb.subsegment(sub[i], sub[i + 1]).error());
//}

class QuadUi {
    constructor(id) {
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

        this.pts = [new Point(200, 450), new Point(400, 450), new Point(600, 50)];
        this.quad = this.make_stroke();
        this.quad.classList.add("quad");
        this.polyline = this.make_stroke();
        this.polyline.classList.add("polyline");
        this.hull = this.make_stroke();
        this.hull.classList.add("hull");
        this.handles = [];
        for (let pt of this.pts) {
            this.handles.push(this.make_handle(pt));
        }
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

    onPointerDown(e) {
        const x = e.offsetX;
        const y = e.offsetY;
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
        for (let i = 0; i < this.pts.length; i++) {
            if (Math.hypot(x - this.pts[i].x, y - this.pts[i].y) < 10) {
                this.current_obj = i;
            }
        }
    }

    onPointerMove(e) {
        if (this.current_obj !== null) {
            const i = this.current_obj;
            const x = e.offsetX;
            const y = e.offsetY;
            this.pts[i] = new Point(x, y);
            this.handles[i].setAttribute("cx", x);
            this.handles[i].setAttribute("cy", y);
            this.update();
        }
    }

    onPointerUp(e) {
        this.current_obj = null;
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

    make_stroke(qb) {
        const path = document.createElementNS(svgNS, "path");
        path.setAttribute("fill", "none");
        path.setAttribute("stroke", "blue");
        this.root.appendChild(path);
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
        const p0 = this.pts[0];
        const p1 = this.pts[1];
        const p2 = this.pts[2];
        const qb = new QuadBez(p0.x, p0.y, p1.x, p1.y, p2.x, p2.y);
        this.quad.setAttribute("d", qb.to_svg_path());

        const h = `M${p0.x} ${p0.y}L${p1.x} ${p1.y}L${p2.x} ${p2.y}`;
        this.hull.setAttribute("d", h);

        const tol = this.thresh;
        let sub;
        if (this.method == "analytic") {
            sub = qb.my_subdiv(tol);
        } else {
            sub = qb.recurse_subdiv(tol);
        }
        const n = sub.length - 1;
        let p = "";
        for (let t of sub) {
            const xy = qb.eval(t);
            this.pips.push(this.plot(xy.x, xy.y));
            if (p == "") {
                p = `M${xy.x} ${xy.y}`;
            } else {
                p += `L${xy.x} ${xy.y}` 
            }
        }
        this.polyline.setAttribute("d", p);
        this.n_label.textContent = `n = ${n}`;
        this.type_label.textContent = this.method;
        this.thresh_label.textContent = `threshold: ${this.thresh}`;
    }
}

new QuadUi("s");
</script>


A few other discussions of the recursive subdivision idea are on [antigrain], [caffeineowl], and [Stack Overflow]. The latter thread also links to some other approaches. A common sentiment is "maybe we shouldn't subdivide exactly in half, but be smarter exactly where to subdivide." This blog post is basically about how to be smarter.

Working with cubic Bézier curves is tricky, but quadratic Bézier curves are pleasantly simple; they are something of a halfway station between cubics and straight lines. In this blog post, we present an analytical approach to flattening quadratic Bézier curves into polylines. It is so good that it's probably best to flatten other curve types (including cubic Béziers) by converting them to quadratics first, then applying the technique of this blog post.

## How many segments?

The core insight of this blog post is a closed-form analytic expression for the number of line segments needed in the continuous limit.

For a small curve segment, the maximum distance between curve and chord can be approximated as $ \Delta y \approx \frac{1}{8}\kappa \Delta s^2 $. Here, $\kappa$ represents curvature, and we use $s$ to represent an infinitesimal distance, not to be confused with $t$ as commonly used to represent the parameter for the Bézier equation.

![Diagram of error](/assets/chord_error.svg)

To simplify the presentation of the math here, we'll solve the basic parabola $y = x^2$, rather than more general quadratic Béziers. However, all quadratic Béziers are equivalent to a segment of this parabola, modulo rotation, translation, and scaling. The first two factors don't affect flattening, and the last can be taken into account by scaling the tolerance threshold. (For the curious, this transformation is `map_to_basic` in the [code] for this post.) Note that $x$ in this transformed version is a linear transform of $t$ in the source Bézier: it can be written $x = x_0 + t(x_1 - x_0)$.

The basic parabola has nice, simple expressions of curvature and infinitesimal arclength in terms of the parameter $x$:


$$
\kappa = \frac{2}{(1 + 4x^2)^\frac{3}{2}}
$$

$$
\Delta s = \sqrt{1 + 4x^2} \Delta x
$$

Plugging these into the above formula, we get an expression for the error:

$$
\Delta y = \frac{(1 + 4x^2) \Delta x^2}{4(1 + 4x^2)^\frac{3}{2}}
$$

Holding the error fixed, we can now simplify and solve this for the step size of the parameter $x$ per segment.

$$
\Delta x = 2  \sqrt{\Delta y} \sqrt[4]{1 + 4x^2}
$$

The rate at which segments occur is the reciprocal of the step size (here I'm also sneaking in a change to the continuous realm).

$$
\frac{\mathrm{d}\; \mbox{segments}}{\mathrm{d}x} = \frac{1}{2\sqrt{\Delta y}\sqrt[4]{1 + 4x^2}}
$$

Taking this to the continuous limit, we can finally write a closed form expression for the number of segments required for the parabola segment from $x_0$ to $x_1$:

$$
\mbox{segments} =  \frac{1}{2\sqrt{\Delta y}} \int_{x_0}^{x_1} \frac{1}{\sqrt[4]{1 + 4x^2}} dx
$$

As it turns out, this integral has a closed form solution, thanks to [hypergeometric functions][Hypergeometric function], though we won't be making too much use of this fact; we'll be doing numerical approximations instead. (I've left out the constant and am making the natural assumption that $f(0) = 0$, as it's an odd function).

$$
f(x) = \int \frac{1}{\sqrt[4]{1 + 4x^2}} dx = x\; {}_2F_1(\tfrac{1}{4}, \tfrac{1}{2}; \tfrac{3}{2}; -4x^2)
$$

But no matter how we evaluate this integral, here we have an expression that tells us how many segments we need. To make sure the solution meets or exceeds the error tolerance, we round up to the nearest integer.

### Actually subdividing

Now we need to come up with $t$ values to know *where* to subdivide. Fortunately, given the mechanisms we've developed, this is fairly straightforward. I'll actually show the code, as it's probably clearer than trying to describe it in prose:

```javascript
    my_subdiv(tol) {
        const params = this.map_to_basic();
        const a0 = approx_myint(params.x0);
        const a2 = approx_myint(params.x2);
        const count = 0.5 * Math.abs(a2 - a0) * Math.sqrt(params.scale / tol);
        const n = Math.ceil(count);
        const u0 = approx_inv_myint(a0);
        const u2 = approx_inv_myint(a2);
        let result = [0];
        for (let i = 1; i < n; i++) {
            const u = approx_inv_myint(a0 + ((a2 - a0) * i) / n);
            const t = (u - u0) / (u2 - u0);
            result.push(t);
        }
        result.push(1);
        return result;
    }
```
Essentially we're subdividing the interval in "count space" into $n$ equal parts, then evaluating the *inverse function* of the interval at each of those points. This loop could easily be evaluated in parallel, and, as we'll see below, the actual formula for the approximate inverse integral is quite simple.

## Numerical techniques

A great way to evaluate this integral is Legendre-Gauss quadrature, as described in my previous [arclength] blog. But we don't actually need to compute this very precisely; we're rounding up to an integer. I played around and found a nice efficient function with roughly the same shape (including asymptotic behavior). Blue is the true curve, orange is our approximation.

$$
f(x) \approx \frac{x}{0.33 + \sqrt[4]{0.67^4 + \frac{1}{4}x^2}}
$$

<!-- Todo: I'd like better images, these are screenshots from colab -->
![Approximation of the integral](/assets/flatten_approx.png)

Reading this graph is fairly intuitive; the slope is the rate at which subdivisions are needed. That's higher in the center of the graph where the curvature is highest.

I actually had even better luck with the inverse function of the integral, which is in some ways more important; this is the one that's evaluated (potentially in parallel) for each intermediate point.

$$
f^{-1}(x) \approx x (0.61 + \sqrt{0.39^2 + \tfrac{1}{4}x^2})
$$

![Approximation of the inverse of the integral](/assets/flatten_inverse_approx.png)

Essentially, the first approximation gives a direct and fairly accurate solution to determining the number of segments needed, and the second gives a direct and even more accurate solution to determining the $t$ parameter values for the subdivision, so that the error is evenly divided among all the segments.

An exercise for the reader is to come up with a better approximation for the first function.

## Comparison to related work

Probably the closest existing literature is the [Precise Flattening of Cubic Bézier Segments] work, which is also the basis of the flattening algorithm in the [lyon] library. This approach also uses parabolas as an approximation, and greedily generates segments of the requested error tolerance.

My technique is both more efficient to evaluate, and also more accurate. Because we round the segment count up to the nearest integer, the error is always within the tolerance. Because "fractional segments" wouldn't make sense, the error of at least one segment is in general considerably less than the threshold. Using a greedy approach, that lower error belongs to the final lucky segment. But our approach distributes the error evenly.

Though the theory is perhaps a bit math-intensive, the code is refreshingly simple. It can probably be used as a drop-in replacement in many implementations that use recursive subdivision for flattening.

And of course, the fact that it can be evaluated in parallel, as well as predict the number of generated segments in advance, means that it's especially well suited for GPU. I'll very likely use this technique as I reboot [piet-metal], though I'll likely be exploring analytical approaches to rendering quadratic Béziers.

[Random Access Vector Graphics]: http://hhoppe.com/proj/ravg/
[caffeineowl]: http://www.caffeineowl.com/graphics/2d/vectorial/bezierintro.html
[Piecewise Linear Approximation]: http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.86.162&rep=rep1&type=pdf
[antigrain]: https://web.archive.org/web/20190329074058/http://antigrain.com:80/research/adaptive_bezier/index.html
[Stack Overflow]: https://stackoverflow.com/questions/9247564/convert-bezier-curve-to-polygonal-chain
[Fast, Precise Flattening of Cubic Bézier Segment Offset Curves]: https://web.archive.org/web/20170911131001/http://cis.usouthal.edu/~hain/general/Publications/Bezier/Bezier%20Offset%20Curves.pdf
[lyon]: https://docs.rs/lyon/0.4.1/lyon/Bézier/index.html#flattening
[Precise Flattening of Cubic Bézier Segments]: https://pdfs.semanticscholar.org/8963/c06a92d6ca8868348b0930bbb800ff6e7920.pdf
[code]: https://github.com/raphlinus/raphlinus.github.io/tree/master/_posts/2019-12-23-flatten-quadbez.md
[Hypergeometric function]: https://en.wikipedia.org/wiki/Hypergeometric_function
[arclength]: https://raphlinus.github.io/curves/2018/12/28/bezier-arclength.html
[piet-metal]: https://github.com/linebender/piet-metal
