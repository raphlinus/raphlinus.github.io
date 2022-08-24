---
layout: post
title:  "Followups"
date:   2019-01-04 09:54:42 -0700
categories: [curves, graphics]
---
<script type="text/x-mathjax-config">
        MathJax.Hub.Config({
                tex2jax: {
                        inlineMath: [['$', '$']]
                }
        });
</script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/mathjax/2.7.0/MathJax.js?config=TeX-AMS-MML_HTMLorMML" type="text/javascript"></script>

Here are some followups on previous blog postings.

## [Arclength](/curves/2018/12/28/bezier-arclength.html)

I believe my [arclength post](/curves/2018/12/28/bezier-arclength.html) gave a very good solution to the cubic Bézier arclength problem, probably better than any implementation out there. But ultimately I was unsatisifed. Is it the best possible?

Clearly not, as I wasn't able to let it go, and kept working on it a bit more. Here's what I've come up with so far.

One path of exploration is a closed-form analytical solution. Commenter [xyzzyz](https://news.ycombinator.com/item?id=18786831) on Hacker News asked whether it was possible, and a bit of exploration in Mathematica indeed resulted in a formula. But it involves the calculation of something like 9 elliptic functions, and has a number of divisions by roots of a quartic equation, which suggests that numerical stability is going to be a big challenge. So, verdict: it is technically possible, but it's almost certain that numerical techniques will win.

I think the basic approach of quadrature with an error bound is sound, but I didn't get it very tight. After considerably more experimentation, I came up with this formula for the core of an error bound:

$$
\int_0^1 \frac{|\mathbf{x}''(t)|^2}{|\mathbf{x}'(t)|^2} dt
$$

Once you've got that, you can take it to pretty much any power you like, and that accurately predicts the error of a corresponding order of quadrature. What I've got now in my [research branch](https://github.com/linebender/kurbo/pull/225) uses 8th order Legendre-Gauss quadrature to compute this integral. Luckily, I've found that you don't need to compute the error bound any more accurately than that.

This lets me use considerably higher orders of quadrature. What I've got now uses 8, 16, and 24, and then subdivides if 24 is not good enough. A good way to visualize its performance is to raise quadratic Béziers to cubic, run the algorithm, and visualize the error (this methodology of course has the potential to miss any contribution from the third derivative, if any, but it's consistent with evaluation of random cubics). Here's what I get:

<img src="/assets/latest_arclen_1e-6.png" width="652" height="602">

This is dramatically better than what I had before. The central three stars (counting from the origin in the lower left corner) are a single application of order 8, 16, and 24 quadrature. Beyond that are subdivisions. The smallest star is about 100ns of computation (50ns each for 8th order quadrature of error estimation and the arclength computation itself), and each edge crossing is about another 50ns. One way to read this graph is that any color other than orange outside the central stars represents wasteful computation. The over-precision in the central stars represents a very cheap computation, there's no benefit to picking a finer-grained order of quadrature.

So, given the constraints of scalar computation, I suspect the only way to do significantly better than this is to come up with an even cheaper way to compute an accurate error bound. I experimented with a bunch of stuff, including expansion of the Taylor series of $\sqrt{x+1}$, and also finding the point of minimum first derivative norm, but didn't make good progress. I would not be at *all* surprised if a better solution were possible, though. The error bound is a pretty smooth function.

### SIMD

The other thing that caught my attention is how well-suited quadrature is for SIMD – it's basically just map, zip, multiply, reduce to sum. Just to get a sense how much speedup is possible, I hand-wrote some AVX intrinsics (also in the [`more_cubic_arclen`][research branch] branch), and the results were stunning. Instead of 50ns for 8th order quadrature, the microbenchmark shows 10ns for 16th. I caution that this is an apples-to-oranges comparison, the full comptuation including branches vs just a microbenchmark of quadrature core, but it shows potential for dramatic speedup. In fact, I wouldn't be surprised if the numeric solution became competitive against the analytical solution _for quadratics,_ which of course should put an end to speculation about the cubic case.

I think the ability to do these kinds of comptuations would be a good thing to add to [fearless_simd]. I've already got a map combinator, so adding zip and sum should be fairly straightforward.

In any case, I haven't committed the improvements back to the main branch of [kurbo] yet. I guess I'm waiting to see if more improvements come down the research pipeline. Frankly, I'm surprised that nobody has really taken up the implicit challenge; I was under the impression that a lot of people from an engineering and computational physics background know how to numerically integrate simple functions and would consider this an easy exercise.

### Better worst case: cusps

The worst-case performance is increasingly fine subdivision when the curve has a cusp. In the worst case, it's about $O(\log n)$ in accuracy, because it's subdividing at the halfway point, so in effect doing a bisection search on the location of the cusp. Each bisection halves the length of the curve so halves the worst-case error. (Note: careful reviewers will find a tricky issue here, but we'll skip over it here.)

If we were particularly concerned about worst-case performance, a useful approach would be to find the cusp and use that to guide the subdivision. I wouldn't be surprised if you could prove $O(1)$ subdivisions for all reasonable tolerances, and with a low constant factor.

The logic to find the cusp already exists in kurbo: it's `cubic.deriv().nearest(Vec2::new(0.0, 0.0), 1e-12)`. I think that's a good demonstration of how the library presents high-level concepts in a composable manner. It feels very Rust-like.

I'm not sure whether I'll do this (it's a bit of additional code complexity and stuff to work out), but would be an important component of a truly gold-plated Bézier arclength method. In particular, it would close the gap with the main remaining advantage of any analytical solution.

## [2D Graphics]

Three months ago I wrote a [blog post][2D Graphics] calling for a new Rust library for 2D graphics. There was some discussion, especially an [insightful response](https://nical.github.io/posts/rust-2d-graphics-01.html) by Nical, and some interest, but not the beginnings of a useful library.

So now I've started [piet]. At the moment, it can fill and stroke Bézier paths, using solid (well, semi-transparently solid) RGBA colors, and not much else. But I'm very excited about it. Among other things, my prototype has a web back-end as a first-class citizen. I was quite surprised, the code for that is actually a lot more similar to the Cairo back-end than either is to the Direct2D one.

In desiging the API I'm leveraging Rust traits. The core of the thing is a [`RenderContext`] trait, which abstracts multiple back-ends such as Direct2D [RenderTarget], Cairo [Context], and Web [CanvasRenderingContext2D]. But beyond that I'm excited about a new [`Shape`] trait, so you have a unified interface with just `stroke`, `fill`, and `clip` on `Shape`, rather than a cross-product of the operation and the primitive. Back-ends can pick and choose which simple shapes (rectangle, line, circle, rounded rect, etc.) to special-case. If they don't, then they just get a path iterator. All this is carefully designed so no allocation when calling piet methods.

The toughest part comes next: text and fonts. I'm going to dive into those today, wish me luck!

I'll be writing about this in considerably more detail soon, but wanted to give people a heads-up. When things settle just a bit more, this will be a good opportunity for collaboration. Among other things, it's designed so that it's easy to wire up a new back-end. In general, that shouldn't require changes to the core API crate.

I'm excited about all these things in the new year. I've had lots of ideas recently, so I'd like a theme to be follow-up, driving them towards useful software.

[kurbo]: http://github.com/linebender/kurbo
[piet]: http://github.com/linebender/piet
[fearless_simd]: https://github.com/raphlinus/fearless_simd
[2D graphics]: /rust/graphics/2018/10/11/2d-graphics.html
[RenderTarget]: https://docs.microsoft.com/en-us/windows/desktop/direct2d/render-targets-overview
[Context]: https://cairographics.org/documentation/pycairo/2/reference/context.html
[CanvasRenderingContext2D]: https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D
[`Shape`]: https://github.com/linebender/kurbo/pull/5
[research branch]: https://github.com/linebender/kurbo/pull/225
[`RenderContext`]: https://github.com/linebender/piet/blob/master/piet/src/render_context.rs
