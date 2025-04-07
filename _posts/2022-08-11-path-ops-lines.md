---
layout: post
title:  "Robust path operations, part 1: lines"
date:   2022-08-11 08:41:42 -0700
categories: [2d]
---
One of the fundamental 2D computational geometry problems is path operations: computing the union or intersection of two shapes represented as vector paths. There is a classic, efficient solution to this problem: the [Bentley-Ottmann sweep line algorithm][Bentley–Ottmann algorithm], but it suffers from numerical stability issues. In particular, it relies on accurate *orientation* results (essentially, which side of a line a point is on), and floating point precision issues can make these results inconsistent. Then the algorithm falls apart, and 

There are a number of approaches to this. In the academic literature, a common approach is to increase the precision of arithmetic until reliable results are obtained. If the input is expressed in rational coordinates (of which floating point numbers are a subset), then all line-line intersections are also rational, so it is possible to get exact results for everything. Generally this is the approach of Jonathan Shewchuk's [robust predicates]  work - compute everything in floating point, and if results are close, redo with higher precision.

This blog presents a different approach to the problem: we use ordinary floating point arithmetic and accept its imprecision, but detect when orientations are within epsilon, and use different rules in those cases. The goal is a reasonably simple algorithm, one which should be possible to prove correct. Though this post will only address the polyline case, an additional motivation is curved segments such as cubic Béziers - the guarantees from rational coordinates of line endpoints don't extend there.

TODO: cite [Skia path ops].

## Statement of the problem

What does it mean to provide numerically robust path intersection? Using floating point arithmetic means an exact solution is not possible (computing the intersection between two lines will introduce roundoff error), so what does it mean to have an approximate solution? The exact formulation of the problem has a profound effect on the solution space.

The first cut looks like this: line segments in the input can be *perturbed* by up to epsilon, then the output is the exact answer of that perturbed input. No two lines are allowed to cross in the output.

What does it mean to perturb a line? We won't move endpoints (preserving the topology), but we can insert any number of additional points, as long as those are within epsilon of the original line. As a preview of what's to follow, there will be three reasons for inserting these points: intersections as in the classical Bentley-Ottman algorithm, splitting of a line when a point comes too close, and introduction of short horizontal segments to resolve tricky intersection cases.

![perturbed line](/assets/pathops_perturb_line.svg)

The algorithm internally works by maintaining an *active list* with an associated invariant. Maintaining that invariant lets us state an even stronger form of the problem, which implies the previous one: for any horizontal slice bounded between y0 and y1 such that no line segments have endpoints y0 < y < y1, all line segments in the output intersecting that interval are *ordered* in nondecreasing order, meaning each successive line segment can be equal to the previous one, or is strictly to the right of it (note to self: I'm wondering if strictly horizontal segments need to be treated separately). We'll need to define that ordering predicate quite carefully; that is a major contribution of this blog post.

## Ordering of line segments

We'll define the ordering predicate in two stages. First is *orientation* of a point with respect to a line, then the ordering between two lines is built in terms of orientation.

With precise math, the orientation of a point with respect to a line is three valued: the point can be to the left of the line, to the right of the line, or exactly on top of it. However, we don't have the luxury of precise math, and there will be cases where the point is so close to the line that we can't reliably determine its orientation. For those cases, we admit a fourth value: ambiguous.

There are two versions of the point-line orientation: a simpler one, used for analysis, and a more sophisticated one.

The simpler one is defined as follows. The point is on top of the line if it is equal to either endpoint. Otherwise, if it is within epsilon of the line, the result is ambiguous. Finally, if it is more than epsilon away, it is right or left of the line consistent with the exact answer.

![graphic showing point-line orientation](/assets/pathops_capsule_orient.svg)

If we were able to completely avoid ambiguous orientations in the output, such a predicate could be the basis for a correct algorithm. However, doing so is hard. Thus, we introduce a more sophisticated "scanline" version of the orientation predicate: when the point has a y coordinate equal to either endpoint, orientation is determined based on comparison of the x coordinate with that endpoint.

![graphic showing point-line orientation, sweep line version](/assets/pathops_sweep_orient.svg)

As we will see, this version of the predicate gives us a convenient way to avoid ambiguous orientations: when a point is ambiguously oriented to a line, split the line there (possibly introducing additional error from the "solve for x given y" floating point calculation, but within epsilon).

### Orientation computational details

This section can be skipped if the goal is an intuitive understanding of the algorithm, but this level of detail would be required for a formal proof of correctness.

* We can't exactly compute these orientations, as comparison with epsilon is also imprecise
* We *can* compute a predicate with lower and upper bounds for epsilon. If < e0, always return ambiguous. If > e1, always return precise answer. Otherwise, either is allowed.
* Small rant: often floating point equality tests are discouraged, there's even a [float_cmp Clippy lint] against it. However, in this case it is precisely what we need. If we took Clippy's helpful advice, it would wreck the algorithm.

Also note: there are a whole bunch of possible variations on the orientation predicate. A more efficient version might be based on bounding boxes rather than Euclidean distance calculations; this would give a square shape rather than round near the endpoints. These may be more efficient to compute but are probably more complicated to analyze.

### Line segment ordering

Now that we have point-line orientation sorted, we can look at the relative ordering of two lines with respect to a horizontal scanline. We'll split that into top and bottom orientation. As a precondition, we assume that the vertical overlap is nonzero. We can also assume that neither line is purely horizontal.

If the two top endpoints have the same y value, the ordering is fully determined by the x coordinates. Otherwise, we solve both lines for x given the maximum y value, and compute the point-line orientations. If either is ambiguous, the ordering is ambiguous, otherwise the ordering is determined from the orientations (note that the orientations can't be inconsistent with each other, as that would break mathematics).

![TODO](/assets/line_seg_order.svg)

Similarly for the bottom ordering. Note that all combinations of top and bottom orderings are possible; if one line segment is top-ordered to the right of the other, but bottom-ordered to the left of it, they intersect somewhere in the middle.

We say a line is strictly ordered to the right of the other when either it is both top-ordered and bottom-ordered to the right, or ordered to the right on either top or bottom, and the other endpoint is equal.

## Sweep line algorithm

The algorithm follows the same structure as Bentley-Ottman. There is an *active list* which is a sorted list of segments that intersect the sweep line. As an outer loop, the sweep line advances from the top of the input to the bottom (generally a [priority queue] is the best data structure to maintain y values for the sweep line). An input segment is inserted into the active list when the sweep line reaches its top endpoint, and likewise deleted at the bottom. During processing, additional y values can be inserted into the priority queue based on newly discovered intersection points.

The invariant is that the active list is in nondecreasing order. This is the same as the original Bentley-Ottmann algorithm, but of course we use our carefully crafted ordering predicate. Note particularly that ambiguous orderings are forbidden. That gives us confidence the algorithm will be correct, but is also an obligation - any time an ambiguous ordering might be introduced, it must be resolved.

Just inserting a line segment into the active list may well violate the invariant. In the classical Bentley-Ottmann algorithm, it might introduce an intersection (the bottom ordering is the opposite direction as the top), but we now have an additional way to violate the invariant, namely introduction of ambiguous orderings. As a general strategy, any change to the active list can potentially violate the invariant, so we have a step for detecting and resolving invariant violations of adjacent segments, and we iterate that until all modified segments satisfy the invariant with respect to their left and right neighbors. (Note: this iteration is a new feature. In the classic Bentley-Ottmann algorithm, when two lines cross, the invariant is restored after computing a single intersection point)

### Restoring the invariant

Thus, the core of the algorithm reduces to detecting violations of the invariant and applying rules to restore it. Since there are four values for top-ordering (left, right, equal, ambiguous) and also for bottom, there are a grand total of 16 cases to consider.

IDEA: show 4x4 grid of all cases.

The easiest cases are when the invariant is valid: (equal, equal), (equal, right), (right, equal), and (right, right). No further work is required.

Next lets take a look at all the cases where at least one ordering is ambiguous. Assume the top-ordering is ambiguous. Then we add an additional split point on the line with the higher top endpoint, solving at y = the top endpoint of the other line. Same for bottom-ordering. If we're lucky, the result is properly ordered, but it might also send us to one of the cases below.

If the second line is top-ordered to the left of the first, then it is in the wrong order in the active list, and the two segments need to be swapped. The same is true in the (equal, left) case.

That leaves us with (right, left), meaning there is an intersection. In classic Bentley-Ottmann, we would always determine the intersection point by solving the relevant line equations, but this may be ill-conditioned, so we apply different strategies. We still do that if all relevant endpoints are > epsilon from the line, otherwise we choose one of the two endpoints and insert that into the *other* line. If we choose the line segment with the more vertical slope, that guarantees it is within epsilon of the other line.

![intersection of two lines, X case](/assets/pathops_intersect_x.svg)

![intersection of two lines, T case](/assets/pathops_intersect_t.svg)

### Horizontal segments

Another detail is special handling of strictly horizontal segments. These don't need to be stored in the active list, but can be in a separate data structure. A sorted list of (x, Δwinding) pairs is good when the winding numbers of the segments will be summed up - a line can be added by inserting (x0, winding) and (x1, -winding). Also note that the within-epsilon intersection case above will tend to insert horizontal segments.

## Generating output

* With equal line segments, sum winding numbers
* Merge active list with horizontal segment line
* Run winding numbers through function (union: >= 1, intersection: >=2, symmetric diff: odd), generate out based on mapped winding

[Bentley–Ottmann algorithm]: https://en.wikipedia.org/wiki/Bentley%E2%80%93Ottmann_algorithm
[Triangle]: https://www.cs.cmu.edu/~quake/triangle.html
[robust predicates]: https://www.cs.cmu.edu/~quake/robust.html
[float_cmp Clippy lint]: https://rust-lang.github.io/rust-clippy/master/#float_cmp
[Priority queue]: https://en.wikipedia.org/wiki/Priority_queue
[Skia path ops]: https://skia.org/docs/dev/present/pathops/
