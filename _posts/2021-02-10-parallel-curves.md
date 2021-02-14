---
layout: post
title:  "Cleaner parallel curves with Euler spirals"
date:   2021-02-10 12:31:42 -0700
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

Determining [parallel curves][Parallel curve] is one of the basic 2D geometry operations. It has obvious applications in graphics, being the basis of creating a stroke outline from a path, but also in computer aided manufacturing (determining the path of a milling tool with finite radius) and path planning for robotics. There's a good literature by now, but in this post I propose a cleaner solution.

A good paper comparing alternative approaches is [Comparing Offset Curve Approximation Methods]. The main difference between these approaches is the choice of curve representation. An example of a curve representation highly specialized for deriving parallel curves is the [Pythagorean Hodograph]. This parallel curve of a Pythagorean Hodograph is an exact parametric polynomial curve, but approximation techniques are still needed in practice, both to convert the source curve into the representation, and because the resulting curves are higher order rational polynomials, which require further approximation to convert into, say, cubic Beziers.

Specifically, this blog proposes piecewise Euler spirals as a curve representation particularly well suited to the parallel curve problem.

There's an implementation of many of these ideas (currently still in PR stage) in [kurbo](https://github.com/linebender/kurbo/pull/169). I also used a colab notebook to explore a bunch of the math, and I've made a [copy of that available](/assets/Euler_spiral_scratchpad.ipynb) as well. [TODO: figure out best way to publish that]

## The cusp

One of the things that makes parallel curves special is that cusps often appear. In particular, a cusp appears whenever the radius of curvature of the source curve matches the offset. This is classified as an [ordinary cusp] and is a feature of many curve families - we'll quantify that a bit more below.

A common feature of algorithms for computing parallel curves is identifying the location of the cusp, and subdividing there. That basically means solving for the specific value of curvature (the reciprocal of the offset distance). If the source curve is a cubic Bezier, there can be up to four such cusps, and finding them requires some nontrivial numerical solving.

## Curvature as a function of arclength

A theme of my approach to parallel curves (and much of my curve work in general, including my [thesis]), is to consider the relationship of curvature to arclength. A concrete intuition is that it is the position of the steering wheel as a car drives along the curve at constant speed. For some curves, curvature can be represented as a closed-form analytical formula as a function of arclength (the [Cesàro equation]), but in general determining the relation requires numerical techniques. For example, in the [Euler explorer], there's a plot of curvature as a function of arclength below the interactive cubic Bezier. Experimenting with that is an excellent way to develop intuition.

One curve that *does* have an especially simple Cesàro equation is the Euler spiral. An Euler spiral segment has this formula:

$$
\kappa(s) = \kappa_0 + \kappa_1 s
$$

(A note for those trying to follow along with the detailed math and code: most of the math and numerical code uses $-0.5 \leq s \leq 0.5$ because it helps exploit even/odd symmetries, but the convention for parametrized curves, including the [ParamCurve] trait in kurbo, is $0 \leq s \leq 1$. Thus, you'll frequently see offsets of 0.5. Similarly, you'll see various scaling to the actual arc length, while the parametrized curve convention assumes an arc length of 1. In this blog, we'll skim over such details, as the goal is to provide intuition without too much clutter from details.)

## The parallel curve of an Euler spiral

In general, most curves do not have a simple formula for their parallel curve. The obvious exception is a circular arc, for which the parallel curve is another circular arc. [The only other "classical" curve I'm aware of is a [circle involute][involute], which has the same self-parallel property, and about more which later. -- TODO clunky; it might be better to call out PH here]

Thanks to its exceptionally simple formulation as a Cesàro equation, the Euler spiral is one of the rare curves with a simple closed-form equation for its parallel curve. That equation was first published in a 1906 paper by Heinrich Wieleitner, [Die Parallelkurve der Klothoide]. For those who don't read German, [Rahix] has kindly provided a translation into English: [PDF], [TeX source]. [TODO: put in assets]

Going over this math, I see Wieleitner missed an opportunity for further simplification. The style at the time was to write the Cesàro equation in terms of the *radius* of curvature (the reciprocal of curvature), but especially for the Euler spiral and its parallel curve, using curvature directly yields a much simpler equation. Putting the cusp at $s = 0$, the equation is gratifyingly simple:

$$
\kappa(s) = \frac{c}{\sqrt{s}} + \frac{1}{l}
$$

Here $c$ is a coefficient dependent on the parameters of the spiral. To connect it to the notation in the Wieleitner paper, $c = a / \sqrt{2 l^3}$, and the offset to $s$ to place the cusp at zero is $-a^2/{2l}$. I've also made a [Desmos calculator graph](https://www.desmos.com/calculator/imvqywsb8o) that interactively demostrates the equivalence of this equation and the more involved one from the Wieleitner paper.

There are a number of other curves that have a cusp similar to the above, with characteristic inverse-square root curvature. The clearest connection is the [circle involute][involute], which is the same but without the $1/l$ term, or in other words the Euler spiral parallel curve approaches the circle involute as the offset goes to infinity. This provides intuition for the fact that a circle involute is its own parallel curve. The circle involute is perhaps most famous as the optimized profile for meshing [gear] teeth, transferring force smoothly with no slop or friction.

Other curves with a similar cusp include the [cycloid] (as well as its many variants including epicycloid, hypocycloid, astroid, deltoid, cardioid, and nephroid), as well as the [semicubical parabola]. The latter is of particular interest because it can be exactly represented as a case of a cubic Bezier (it is when the control arms form a symmetrical X).

![semicubical parabola](/assets/semicubical_parabola.svg)

The parallel curve of the Euler spiral is perfectly cromulent, and, following the tradition of Pythagorean Hodograph curves and their higher-order rational polynomials, we could simply require everything downstream to simply deal with them. But to make that downstream processing easier, we will convert back to piecewise Euler spirals, a more tractable representation.

## Geometric Hermite interpolation

[Hermite interpolation] is a well known technique. In its simplest form, it is used to generate a piecewise polynomial approximation to some function, where the parameters for each polynomial segment are determined from the values and derivatives of the endpoints. For example, in cubic Hermite interpolation, a cubic polynomial is determined from the values and first derivatives at the endpoints - four values, corresponding to four coefficients for the polynomial. The result is C1 continuous as the derivatives exactly match (and are equal to the source curve).

In 2D, there is a distinction between C1 and G1 (geometric) continuity. In C1 continuity, the full derivatives must match, both direction and magnitude. For applications such as animating motion curves, the magnitude is important (it represents speed of motion), but for curves, it is not. G1 continuity requires that the tangents match, but does not specify the magnitude of the derivatives.

In these applications, geometric Hermite interpolation is more efficient, as all parameters of the curve are available to make the shape fit. The Euler spiral is especially well suited to geometric Hermite interpolation, and there is literature on this topic. For reasonable assumptions of smoothness (excluding fractal curves but including simple cusps), the accuracy scales as $O(n^4)$ - a doubling of the number of subdivisions reduces the error by a factor of 16. This scaling is the same as cubic Hermite interpolation of a 1D function, not surprising as an Euler spiral segment approximates a cubic polynomial when $y$ values are small.

### A simple, accurate error metric

The most common approach to approximation given a target error bound is adaptive subdivision: approximate the error, and if it exceeds the target, subdivide. Evaluating the error is not always easy; most generally, it's based on numerical techniques such as evaluating the curve at several points along its length and testing how near those points lie to the source curve.

Fortunately, for approximating an Euler spiral parallel curve using an Euler spiral, there is an extremely simple formula for the error. In fact, it's possible to avoid the adaptive subdivision altogether, and precisely predict how many subdivisions are needed to meet an error bound, as well as analytically place the subdivisions so each segment has the same error.

Normalized to a chord length of 1, where the arc length of the Euler spiral segment is $a$, the error for approximating an Euler spiral segment with central curvature $\kappa_0$ and curvature variation $\kappa_1$ offset by distance $l$ is:

$$
E \approx 0.005a\left|\frac{1}{\kappa_0 a^{-1} + l^{-1}}\right|\kappa_1 ^ 2
$$

The $\kappa_0 a^{-1} + l^{-1}$ term represents a distance from the cusp; the error scales in inverse proportion to this distance. Also note that $\kappa_1$ scales as the square of the number of subdivisions, so the entire formula scales as the fourth power, as expected.


## Euler spiral or parabola

At heart, the algorithm is similar to the subdivision into parabolas.

## Euler spirals to cubic Beziers

A previous blog post, [Secrets of smooth Béziers revealed], addressed the question of fitting a cubic Bezier to approximate an Euler spiral segment. There is more to be said on the topic, but here I will show a simple and appealing solution.

Graphic designers using cubic Beziers are commonly taught that smooth curves result when the distance from the control point to the endpoint is approximately 1/3 the distance between the endpoints. A more precise refinement of this concept is to draw a parabola around each endpoint, with the vertex 1/3 way along the chord, and a distance of 2/3 in the orthogonal direction. The Euler spiral approximation is simply the point along that parabola in the desired tangent direction.

![Fitting an Euler spiral to a cubic bezier](/assets/euler_fit.svg)

[TODO: It's tempting to make this into a full-fledged interactive, possibly including an error metric]

In the symmetrical case, this solution is equivalent to the standard solution for approximating a [circular arc using a cubic bezier], as can be seen with a bit of trigonometry. What's less obvious is that it remains very good even in the non-symmetrical case, in particular the arclength of the bezier matches the true curve pretty well. The error scaling is as the fifth power, which is better than fourth power scaling of using standard Hermite interpolation (it consistently undershoots arclength), but not as good as the sixth power scaling that is theoretically possible, as shown in [High Accuracy Geometric Hermite Interpolation].

This simple fitting with $n^5$ scaling, is appealing because it is very fast to evaluate, and in most cases will produce cubic beziers with a comfortable but not excessive safety margin for accuracy, especially since the approximation.

[thesis]: https://www.levien.com/phd/phd.html
[Cesàro equation]: https://en.wikipedia.org/wiki/Ces%C3%A0ro_equation
[Parallel curve]: https://en.wikipedia.org/wiki/Parallel_curve
[Comparing Offset Curve Approximation Methods]: https://www.semanticscholar.org/paper/Comparing-Offset-Curve-Approximation-Methods-Elber-Lee/9ac1978746ec54bdd555b906e2ea1eb922cd6ffd
[Pythagorean Hodograph]: https://www.semanticscholar.org/paper/Pythagorean-hodographs-Farouki-Sakkalis/e20aeb60de908061797b6eaf3af79fdc7e5acdd7
[ordinary cusp]: https://en.wikipedia.org/wiki/Cusp_(singularity)
[Euler explorer]: https://levien.com/euler_explorer/
[ParamCurve]: https://docs.rs/kurbo/0.8.0/kurbo/trait.ParamCurve.html
[Die Parallelkurve der Klothoide]: https://books.google.com/books?id=UvpZAAAAYAAJ&pg=PA373&lpg=PA373&dq=%22Die+Parallelkurve+der+Klothoide%22&source=bl&ots=fuY39VdPpd&sig=K0AbL03rXAm_g4J9KsheQbbxyaA&hl=en&sa=X&ved=2ahUKEwiUrcD1poTfAhVvFjQIHVthBPoQ6AEwAnoECAMQAQ#v=onepage&q=%22Die%20Parallelkurve%20der%20Klothoide%22&f=false
[Rahix]: https://github.com/Rahix
[gear]: https://ciechanow.ski/gears/
[involute]: https://en.wikipedia.org/wiki/Involute
[cycloid]: https://en.wikipedia.org/wiki/Cycloid
[semicubical parabola]: https://en.wikipedia.org/wiki/Semicubical_parabola
[Hermite interpolation]: https://en.wikipedia.org/wiki/Hermite_interpolation
[Secrets of smooth Béziers revealed]: https://raphlinus.github.io/curves/2018/12/08/euler-spiral.html
[circular arc using a cubic bezier]: https://pomax.github.io/bezierinfo/#circles_cubic
[High Accuracy Geometric Hermite Interpolation]: https://minds.wisconsin.edu/bitstream/1793/58822/1/TR692.pdf