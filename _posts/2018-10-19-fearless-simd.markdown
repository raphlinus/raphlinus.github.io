---
layout: post
title:  "Towards fearless SIMD"
date:   2018-10-19 10:03:42 -0700
categories: [rust, simd]
---
[SIMD] is a powerful performance technique, and is especially valuable in signal and image processing applications. I will be using it very extensively in my [synthesizer], and also it's increasingly used in [xi-editor] to optimize string comparisons and similar primitives.

Traditionally, programming SIMD has been very difficult, for a variety of reasons. Until recently, the most practical approach was writing assembly code, which is very arcane. Today, probably most SIMD code is written in C using processor-specific intrinsics. The future is portable, high level code, but tools aren't quite there yet. Rust has the potential to be one of the leading langauges for SIMD, but the current state is fairly rough. In this post, I'll set out the challenges, results of some of my explorations, and suggestions for things to improve.

I call my vision for what I'd like Rust to accomplish "fearless SIMD," in analogy with "[fearless concurrency]". In this vision, the programmer writes the computation using high-level, safe, composable primitives, which then compile down to nearly perfect code for each SIMD capability level of the target architecture, with automatic runtime selection. Simple operations (like doing a map of a scalar function evaluation across a vector) can be written simply. More exotic SIMD operations are exposed, portably if possible, but also with an escape hatch of using processor-specific intrinsics when they're truly needed.

I'm very excited by the fact that SIMD is now part of stable Rust, but the current state is very far from "fearless". I've published a crate called [fearless_simd], but that name is more of an aspiration than a promise fulfilled. Even so, I think it points one possible way. I plan to use it for music synthesis and visualization in my synthesizer, and invite the commnunity to adapt the ideas, or come up with something better!

## Why is SIMD hard?

While traditional scalar operations are pretty much standardized and available on all reasonable CPUs (with interesting exceptions such as [popcnt]), SIMD capabilities vary widely from chip to chip. For modern CPUs, ARM (including both 32 and 64 bit variants) has 128 bit wide SIMD types, the bulk of x86_64 CPUs have 256 bit types, and the very newest of those now have 512 bit. Code written for too high a SIMD capability will generally crash (it's [undefined behavior]), while code written for too low a SIMD capability will fall short of the performance available.

Writing SIMD code for a specific chip is much, much easier than writing code that will run well across diverse chips. The challenges are on two levels. The easier (but still challenging) problem is compiling code to run on a single target chip, selecting at compile time the best of several alternates based on SIMD capaibilities. The next-level challenge is compiling multiple alternates into the same binary, and selecting the best at runtime.

The Rust compiler provides a solution for the first level of challenge, at least for the brave programmer. It has a `target-cpu` flag, which directs it to produce code for a specific chip. Then, as part of its [SIMD support in stable], there is a `#[cfg(target_feature = ...)]` mechanism that conditionally compiles code based on the features available for the selected target CPU. Code protected by such a guard can then use [SIMD intrinsics][x86_64 intrinsics] for that level, but these are all marked as `unsafe` because the compiler doesn't statically check that the capability is actually present.

For runtime selection, the Rust compiler provides some raw tools, but it's entirely up to the programmer to put them together correctly. There's an annotation to selectively "turn on" SIMD capabilities for a single function, and an [`is_x86_feature_detected!`] macro for detecting at runtime. The typical idiom looks something like this:

```rust
#[target_feature(enable = "avx")]
unsafe fn foo_avx(...) {
    let ... = _mm256_add_ps(..., ...);
}

fn foo(...) {
    if is_x86_feature_detected!("avx") {
        unsafe { foo_avx(...); }
    } else {
        foo_fallback(...);
    }
}
```

I've left out the additional `#[cfg]` attributes to make sure these variants only compile under x86\_64 (or x86) target_architecture for clarity, but it's obvious this is cumbersome enough. Also note that the `#[target_feature(enable)]` attribute _requires_ the function to be unsafe, as calling it without a runtime check is undefined behavior.

This works pretty well when, as is traditional, the SIMD-specific logic is written as a block inside a single function. Things start falling apart when trying to compose the logic from multiple pieces. First, there are [bugs](https://github.com/rust-lang/rust/issues/50154) that happen when inlining across different calling conventions. More profoundly, the `#[cfg(target_feature = ...)]` attribute that works so well for compile-time detection no longer works for runtime detection, because it's resolved too early, and cannot give the answer to the question, "what capabilities have been asserted in the function that's inlining me"? There's some [discussion](https://internals.rust-lang.org/t/packed-simd-cfg-target-feature-does-not-play-well-with-target-feature/8115) with a pointer to a potential answer, but my personal take is that this is a fundamental problem that ultimately will need deep language changes to address.

## Existing higher-level SIMD approaches

There are two crates which aim to provide a higher level SIMD experience: [packed_simd], which provides non processor specific types such as `f32x4` and operations on them, and [faster], which is an even higher level approach offering iterators and transformers.

Both of these show significant promise in allowing higher level code, but neither address the problem of runtime selection of SIMD capability level.

It's also worth taking a deeper look at what the C/C++ world is doing, as that's currently by far the most mature ecosystem for SIMD development. Though SIMD is not part of the standard language, both GCC and Clang have vector extensions. There's also support for [Function Multi Versioning](https://lwn.net/Articles/691932/), which is particularly well supported on Linux (not sure about other platforms, but I wouldn't be surprised). I think one of the larger discussions going forward is to what extent to adapt these language and runtime level features into the Rust language.

## Approaching fearless SIMD

In the meantime, I wanted to explore how far it's possible to go using Rust as it exists today. Indeed, [fearless_simd] works on stable Rust. It's certainly not a general purpose solution to the problem, but based on my experiments so far, I think it might be usable for some things. I hope to write synthesis and visualization algorithms for my synthesizer using it, and so far it seems to be working - I have samples for both [waveform generation] and [IIR filtering]

The main theme of this crate is to provide _traits_ at two levels. On the lowest level are traits representing some SIMD vector, either of a particular width (`F32x4`) or the native width of the underlying vector (`SimdF32`). The latter is particularly useful for a simple map operation of a scalar function. Then there are implementations (simple newtypes over arch-specific types such as `__m256`) that implement these traits. The usual arithmetic operations are provided (using std::ops traits, so you can write `a + b` rather than having to do `a.add_(b)`), They also provide more specialized SIMD operations such as approximate reciprocal square root (see my [sigmoid] post for an application of these; also note that at that time I was only using SSE so not seeing a dramatic improvement, but with AVX it's almost 2x).

These traits are mostly safe methods, wrapping the underlying unsafety, though of course unsafe methods are also provided if needed as an escape hatch. In particular, _creating_ a value of a particular SIMD implementation is an unsafe operation, as it depends on runtime detection of SIMD capability.

The higher level of trait represents some user-specified computation, generic over all concrete implementations of the SIMD trait. Then, architecture specific runners detect the SIMD level at runtime, and calls into that trait with the appropriate concrete implementation. The runner is safe, using runtime detection to guard its internally unsafe call to the specialized SIMD version. Also, Rust's monomorphization takes care of automatically generating multiple versions of the code for the multiple SIMD levels, including a scalar fallback that works on any architecture.

Right now, I have two higher level traits. One represents simple scalar transforms from f32 to f32, and the other is a _thunk_ which can be pretty much any computation, ie it's not limited to iterators, and can do random access into slices, etc. I can imagine more, such as `(f32, f32) -> f32` functions, but I'll implement these as I need them.

Using these traits requires a somewhat contrived style, but it's _much_ easier than programming SIMD intrinsics directly. The code generation quality is excellent too. Here are unscientific benchmarks for the sinewave generation example:

The particular benchmark is generation of a sinewave with less than -100dB disortion, and times are given in ns to generate 64 samples.

| --------- + --------------- + ----- |
| CPU       | simd level      | time  |
| --------- | --------------- | ----: |
| i7 7700HQ | AVX             |   30  |
| "         | SSE 4.2         |   49  |
| "         | scalar fallback |  344  |
| "         | sin() scalar    |  506  |
| i5 430M   | SSE4.2          |  303  |
| "         | scalar fallback |  717  |
| "         | sin() scalar    | 1690  |

Note that this is a performance of approximately 470 picoseconds per sample. Modern computers are fast when running optimized code.

## Limitations and caveats

I ran into a number of limitations of current Rust while writing this. I think it's likely some of these will improve. Partly why I'm publishing this crate is to shine a light on where more work might be useful.

Using this crate is very sensitive to inlining, an getting it wrong will trigger [rust-lang/rust#50154]. That said, the `GeneratorF32` trait is designed so that iterator creation happens inside a target_feature wrapper, which should both reduce the chance of triggering that bug, and improve code quality. In the [waveform generation] example, the `sin9_shaper` requires `#[inline(always)]`, or else terrible code results; normally it's rare to require this attribute, and just `#[inline]` is usually recommended (see [stdsimd#340](https://github.com/rust-lang-nursery/stdsimd/issues/340) for more discussion).

That bug is not the only inlining misfeature; the `#[cfg(target_feature)]` macro is resolved too early and does not report whether the feature is enabled if the function is inlined. This is discussed a bit in a [rust-internals thread](https://internals.rust-lang.org/t/packed-simd-cfg-target-feature-does-not-play-well-with-target-feature/8115). It's not clear to me that the [proposed approach forward](https://internals.rust-lang.org/t/using-run-time-feature-detection-in-core/8419) really fixes the issue, because runtime feature doesn't always match `[target_feature(enabled)]`. For example, runtime feature detection may show that AVX-512 is available, but the user may choose to use only AVX2 for [performance reasons](https://lemire.me/blog/2018/09/07/avx-512-when-and-how-to-use-these-new-instructions/).

I wanted to make the `GeneratorF32` trait processor-independent and fully generic. In other words, I'd like to be able to write this:

```rust
pub trait GeneratorF32: Sized {
    type Iter<S: SimdF32>: Iterator<Item=S>;
    fn gen<S>(self, cap: S) -> Self::Iter<S>;
}
```
This feature is in the works: generic associated types ([rust-lang/rust#44265]).

If `x` has a `SimdF32` value, it is possible to write, say, `x + 1.0`, but at the moment `1.0 + x` does not work. The relevant trait bounds do work if added to the `SimdF32` trait, but it would force a lot of boilerplate into client implementations, due to [rust-lang/rust#23856]. That looks like it might get improved when Chalk lands.

I use the `SimdFnF32` trait to represent a function is generic in the actual SIMD type. Even better would be something like this:

```rust
pub trait GeneratorF32: Sized {
    fn map<F>(self, f: F) where F: for<S: SimdF32> Fn(S) -> S;
}
```

Currently the `for<>` syntax works for higher-ranked lifetimes but not higher-ranked generics in general. I'm not sure this will ever happen, but it shows a potential real-world example for why these exotic higher-ranked types might be useful.

Also while evaluating the performance through benchmarks, I found that the [`round` operation in Rust is very slow](https://github.com/rust-lang/rust/issues/55107). I'm actually digging into that and trying to fix it, and it's a surprisingly deep issue in and of itself, possibly the subject of a future blog post.

## Prospects

I'm not proposing [fearless_simd] for general usage yet. On the other hand, I do plan to use it as much as possible to develop signal processing and visualization algorithms in my synthesizer, adding capabilities to the library. If other people find it useful, even better. I'll certainly accept pull requests for use cases broadly aligned with the current direction of the crate.

I certainly encourage people to experiment with and explore different ways of writing SIMD code. It's exciting that writing high quality SIMD code in Rust is now possible. The beauty and power of Rust is composing low-level components into higher level systems, using zero-cost abstractions. What are the best traits to represent generic computations that can be implemented efficiently in SIMD? How far can we go using currently stable Rust, and to what extent will extensions to the language enable an even better SIMD experience, perhaps someday fulfilling the promise of fearless SIMD? I'd like to think my exploration contributes to this discussion, and am really looking forward to seeing where it goes.

[SIMD]: https://en.wikipedia.org/wiki/SIMD
[xi-editor]: https://github.com/xi-editor/xi-editor
[popcnt]: https://en.wikipedia.org/wiki/Hamming_weight
[fearless concurrency]: https://blog.rust-lang.org/2015/04/10/Fearless-Concurrency.html
[fearless_simd]: https://github.com/raphlinus/fearless_simd
[AVX-512]: https://lemire.me/blog/2018/09/07/avx-512-when-and-how-to-use-these-new-instructions/
[SIMD support in stable]: https://github.com/rust-lang/rust/issues/48556
[x86_64 intrinsics]: https://doc.rust-lang.org/beta/core/arch/x86_64/index.html
[`is_x86_feature_detected!`]: https://doc.rust-lang.org/std/macro.is_x86_feature_detected.html
[undefined behavior]: https://raphlinus.github.io/programming/rust/2018/08/17/undefined-behavior.html
[waveform generation]: https://github.com/raphlinus/fearless_simd/blob/master/examples/sinewave.rs
[IIR filtering]: https://github.com/raphlinus/fearless_simd/blob/master/examples/iir.rs
[synthesizer]: https://github.com/raphlinus/synthesizer-io
[rust-lang/rust#50154]: https://github.com/rust-lang/rust/issues/50154
[rust-lang/rust#44265]: https://github.com/rust-lang/rust/issues/44265
[rust-lang/rust#23856]: https://github.com/rust-lang/rust/issues/23856
[packed_simd]: https://github.com/rust-lang-nursery/packed_simd
[faster]: https://github.com/AdamNiederer/faster
[sigmoids]: https://raphlinus.github.io/audio/2018/09/05/sigmoid.html
