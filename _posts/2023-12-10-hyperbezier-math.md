---
layout: post
title:  "The mathematical beauty of hyperbezier curves"
date:   2024-11-18 11:29:42 -0800
categories: [curves]
---
I have been on a decades-long quest to find a curve family better suited for interactive design than the trusty cubic Bézier. My [PhD thesis] proposed Spiro, which is better in some ways (smoother curves) but is also less versatile and expressive. In particular, it does a poor job representing curves with high tension. I now propose a curve family which I believe is simply better than Béziers. I call it "hyperbezier," both because it is based on Béziers, and also because it is related to hypergeometric functions and can reasonably well approximate hyperbolas.

The curve family is given as a [Cesàro equation], curvature as a function of arc length:

$$\kappa(s) = \frac{p_1s+p_0}{(q_2s^2+q_1s+q_0)^{1.5}}$$

The curve family contains within its parameter space two beautiful curves: Euler spiral and circle involute. It is also a versatile mimic

## Related curves

By considering various ranges of the parameter space, the hyperbezier either contains or closely approximates quite a large variety of familiar curves, including Euler spiral, circle involute, cubic Béziers, hyperbolas.

### Euler spiral

Setting $$q_1$$ and $$q_2$$ to zero, the hyperbezier reduces to a simple Euler spiral.

### Circle involute

Setting $$p_1$$ and $$q_1$$ to one, the others to zero, the hyperbezier reduces to a circle involute, given by the Cesàro equation $$\kappa(s)=\frac{1}{\sqrt{s}}$$. This curve is probably best known as the ideal profile for [gear teeth][Gears] without slipping, but has other fascinating properties. In particular, it is the only curve I know other than circular arc that is its own parallel curve.

The circle involute has a cusp of order 3/2, and in this way is related to the semi-cubical parabola, which has the same cusp.

## Whewell equation

In general, when a curve is given as a Cesàro equation, a good way to evaluate it is to convert it into a [Whewell equation] (tangent angle as a function of arc length) by integrating the curvature, then doing numerical integration of the sine and cosine of that angle to give Cartesian coordinates.

Somewhat remarkably, the Whewell form of the hyperbezier is extremely similar to its Cesàro form:

$$\theta(s) = \frac{p_1's+p_0'}{\sqrt{q_2s^2+q_1s+q_0}}$$

The quadratic polynomial in the denominator is the same, but the linear equation in the numerator is a straightforward transform of the Cesàro parameters (this is assuming that the coefficients have been normalized so that $$q_0 = 1$$):

$$p_0' = \frac{2p_0q_1 - p_1q_1^2/q_2}{4q_2 - q_1^2} - \frac{p_1}{q_2}$$
$$p_1' = \frac{4p_0q_2 - 2p_1q_1}{4q_2 - q_1^2}$$

This transform follows fairly straightforwardly from the following integral, which has a pleasing symmetry:

$$\int \frac{p_1s+p_0}{(as^2 + 1)^{1.5}} ds = \frac{p_0s-p_1/a}{\sqrt{as^2 + 1}} + C$$

## The exponent

The exponent in the denominator is 1.5, which may seem like an odd choice. I experimented also with both one and two. The former would be appealing; it would essentially write the curvature as a simple rational function, evoking [Padé approximation]. However, it is not numerically robust. To represent high-tension curves, the parameters 

A choice of two is practical. The integrals don't come out quite as nicely, and we don't get the circle involute (instead, a logarithmic spiral).

[PhD thesis]: TODO
[Cesàro equation]: https://en.wikipedia.org/wiki/Ces%C3%A0ro_equation
[Whewell equation]: https://en.wikipedia.org/wiki/Whewell_equation
[Gears]: https://ciechanow.ski/gears/
[Padé approximation]: https://en.wikipedia.org/wiki/Pad%C3%A9_approximant
