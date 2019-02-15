---
layout: post
title:  "IIR filters can be evaluated in parallel"
date:   2019-02-14 14:50:42 -0700
categories: [audio]
---
(Author's note: I got slightly stuck writing this, so am publishing a somewhat unfinished draft, largely so I can get to the many other items in my blogging queue (there's a [thread] on the xi Zulip for those more interested). I can come back to this and do a more polished version if there's interest.)

At the risk of oversimplification, there are basically two types of digital filter: finite impulse response and infinite impulse response. The former is basically taking the dot product of the filter response with a slice of input samples, for each input sample. Analysis of the latter is trickier, and involves internal *state*, which in general decays over time but never goes exactly to zero.

It is trivial to see how to evaluate FIR filters in parallel; each dot product is independent of the others, so it's basically [embarrassingly parallel]. There's more to to the story, especially as the filter kernel becomes larger. Then the simple O(nm) approach yields to O(n log n) techniques based on FFT, and these techniques are why [convolutional reverb] is practical. But we know how do FFT with high parallelism.

By contrast, IIR filters at first glance look like they *must* be evaluated in series. But the first glance is misleading. Their linear nature means they can be evaluated quite efficiently in parallel, but it's not obvious. I'm not stating a new fact here, it's in the literature, but I haven't found a particularly clear statement of it, nor a clear discussion of whether it only applies to linear time-invariant filters or whether the filter parameters can be modulated (spoiler alert: they can).

Take the simplest IIR filter, the so-called "RC lowpass filter", better known as a one-pole filter:

```
y[i] = x[i] * c + y[i - 1] * (1 - c)
```

From this formulation, it's not possible to start calculation of `y[i]` until `y[i-1]` is known. This is basically a serial chain of data dependencies. But there's more we can do, because of the linear nature of the filter. Let's see if we can unroll it to do two at a time:

```
y[i] = x[i] * c + (x[i - 1] * c + y[i - 2] * (1 - c)) * (1 - c)
```

This calculates the same value (subject to floating point roundoff), but the data dependency is twice as long. On the other hand, it seems to require more multiplications, so it's not obvious it's a win.

This idea generalizes to more sophisticated filters. Rather than writing it it in [direct form], which emphasizes the serial evaluation strategy, it's better to use matrices. Basically, `y` becomes a state vector, and `a` becomes a matrix. This is known as the state space approach to filters and has many advantages. I'd go so far to say that direct form is essentially obsolete now, optimizing for the number of multiplies at the expense of parallelism, numerical stability, and good modulation properties.

When I implemented the filter in [music-synthesizer-for-android], I was looking for techniques to speed up the code using SIMD. I came across some papers, notably [Implementation of recursive digital filters into vector SIMD DSP architectures], that gave working recipes for more general filtering, optimized for SIMD. These techniques work, and I wrote about them a bit in a notebook entitled [Second order sections in matrix form]. In that notebook, I also argue for the advantages for numerical stability and modulation, and I won't go into that more here.

Reviewing the literature again, a *much* more detailed reference is [Compiling High Performance Recursive Filters], which emphasizes code generation techniques but does cover the underlying math, including good references. Likely the earliest reference showing that IIR filters can be evaluated in parallel is the Sung and Mitra 1986 reference, "Efficient multi-processor implementation of recursive digital filters." (no direct link available, but of course sci-hub works). Indeed, Sung and Mitra state quite clearly that time-varying filters work, as long as they're linear.

## Monoid homomorphism time

One of my favorite mathematical frameworks, monoid homomorphism, is powerful enough to accommodate parallel evaluation of IIR filters as well. The basic insight is that the target monoid is a function from input state to output state, and represents any integral number of samples. The monoid binary operator is function composition, which is by nature associative and has an identity (the identity function).

This is the same fundamental trick as lifting a regular expression (or, equivalently, finite state machine) into a monoid. In *general,* the amount of state required to represent such a function, as opposed to a single state value, is intractable, but in these two cases it works. In the case of a regular expression, it works because the domain of the regular expression is finite. In the case of an IIR, it works because the function is linear.

Let's dig into more detail. The target monoid is a function of this form:

```
y_out = a * y_in + b
```

This can obviously be represented as two floats, we can write the representation as simply `(a, b)`. The homomorphism then binds a single sample of input into a function. Given the filter above, an input of `x` maps to `(c, x * (1 - c))`.

Similarly, we can write out the effect of function composition in the two-floats representation space. Given `(a1, b1)` and `(a2, b2)`, their composition is `(a1 * a2, a2 * b1 + b2)`. Not especially complicated or difficult to compute.

If we just evaluate this homomorphism, then we get the filter state at the end, which is nice, doesn't really count as evaluating the filter. What we want is the filter state at the end of each prefix of the input. But fortunately, that's possible too. It's most generally called "scan," and is often thought of as a generalization of [prefix sum]. Fortunately, there is a great literature on parallel evaluation of this primitive - one of the more sophisticated approaches has a depth of 2 * log<sub>2</sub> n), and a work factor of 2, meaning twice the number of primitive evaluations as the serial approach. A number of other intermediates are possible, especially including the SIMD-friendly variants we saw earlier. A good read on scans is the paper that (I believe) introduced them is [Scans as Primitive Parallel Operations].

## Time-varying parameters

Note that there's nothing about the homomorphism above that requires the filter to be time-invariant. We can take both the input signal and the filter parameters as input to the map, and the math works out the same. Note that this is in stark contrast to convolutional reverb techniques, which do require that the filter kernel be linear time invariant.

## Possible extension to nonlinearity

This technique is for linear filters, but nonlinear filters are also interesting in audio (and other) applications. For example, many subtractive synthesizers use a "virtual analog" technique, which is often based on a linear core but has nonlinearities, more faithfully capturing the actual performance of electronic components used in active filters. (For a comprehensive treatment of this topic, see [The art of VA filter design] (PDF).)

I think it's an interesting question whether the parallelization techniques described here can be extended to nonlinear filters as well. My guess is, probably not realistically for faithful simulation of analog circuits, but it might be possible to design a nonlinear response that can be represented with a relatively small number of parameters, and also composes (and respects associativity, the main requirement of monoid homomorphisms). I haven't come up with one yet, but would be interested to explore the space more.

## Other references

There's a great [historical article comparing FIR and IIR] that talks about how the relative advantages and disadvantages of each has evolved over time, and cites exploitation of parallelism as one of the reasons for FIR's success.

For an extremely in-depth presentation of digital filters, with solid mathematical foundations, see Julius Smith's online book [Introduction to Digital Filters]. In particular, it's a great exposition of the state space approach.

[embarrassingly parallel]: https://en.wikipedia.org/wiki/Embarrassingly_parallel
[convolutional reverb]: https://en.wikipedia.org/wiki/Convolution_reverb
[Compiling High Performance Recursive Filters]: https://hal.inria.fr/hal-01167185
[Implementation of recursive digital filters into vector SIMD DSP architectures]: http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.1.3729&rep=rep1&type=pdf
[historical article comparing FIR and IIR]: http://www.rci.rutgers.edu/~shunsun/resource/IIR_History.pdf
[Introduction to Digital Filters]: https://ccrma.stanford.edu/~jos/filters/
[music-synthesizer-for-android]: https://github.com/google/music-synthesizer-for-android
[direct form]: https://ccrma.stanford.edu/~jos/fp/Direct_Form_I.html
[Second order sections in matrix form]: https://github.com/google/music-synthesizer-for-android/blob/master/lab/Second%20order%20sections%20in%20matrix%20form.ipynb
[Scans as Primitive Parallel Operations]: https://people.eecs.berkeley.edu/~driscoll/cs267/papers/scan_primitive.pdf
[sigfpe post]: http://blog.sigfpe.com/2009/01/beyond-regular-expressions-more.html
[prefix sum]: https://en.wikipedia.org/wiki/Prefix_sum
[thread]: https://xi.zulipchat.com/#narrow/stream/181284-blogging/topic/Raph's.20backlog
[The art of VA filter design]: (https://www.native-instruments.com/fileadmin/ni_media/downloads/pdf/VAFilterDesign_1.1.1.pdf)