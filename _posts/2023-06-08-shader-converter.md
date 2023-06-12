---
layout: post
title:  "A note on Metal shader converter"
date:   2023-06-09 07:03:42 -0700
categories: [gpu]
---
At WWDC, Apple introduced [Metal shader converter], a tool for converting shaders from DXIL (the main compilation target of HLSL in DirectX12) to Metal. While it is no doubt useful for reducing the cost of porting games from DirectX to Metal, I feel it does not move us any closer to a world of robust GPU infrastructure, and in many ways just adds more underspecified layers of complexity.

The specific feature I'm salty about is atomic barriers that allow for some sharing of work between threadgroups. These barriers are present in HLSL, and in fact have been since 2009, when [Direct3D 11] and Shader Model 5 were first introduced.

I've discussed the value of this barrier in my blog post [Prefix sum on portable compute shaders], but I'll briefly recap. Among other things, it enables a single-pass implementation of prefix sum, using a technique such as decoupled look-back or the [SAM prefix sum] algorithm. A single-pass implementation can achieve the same throughput as memcpy, while a more traditional tree-reduction approach can at best achieve 2/3 that throughput, as it has to read the entire input in two separate dispatches. Further, tree reduction can actually be more complex to implement in practice, as the number of dispatches varies with the input size (it is typically `2 * ceil(log(n) / log(threadgroup size))`). Prefix sum, in turn is an important primitive for advanced compute workloads. There are a number of instances of it in the [Vello] pipeline, and it's also commonly used in stream compaction, decoding of variable length data streams, and compression.

I believe there are other important techniques that are similarly unlocked by the availability of these primitives. For example, Nanite's advanced compute pipelines schedule work through job queues, and in general it is not possible to reliably coordinate work between different threadgroups (even within the same dispatch) without such a barrier.

## Complexity and reasoning

The GPU ecosystem exists at the knife edge of being strangled by complexity. A big part of the problem is that features tend to inhabit a quantum superposition of existing and not existing. Typically there is an anemic core, surrounded by a cloud of optional features. The Vulkan ecosystem is notorious for this: the [extension list at vulkan.gpuinfo.org] currently lists 146 extensions.

The widespread use of shader translation makes the situation even worse. When writing HLSL that will be translated into other shader languages, it's no longer sufficient to consider [Shader Model 5] to be a baseline, but rather the developer needs to keep in mind all the features that don't translate to other languages. In some cases, the semantics change subtly (the rules for the various flavors "count leading zeros" when the input is 0 vary), and in other cases, like these device scoped barriers.

A separate category is things technically forbidden by the spec, but expected to work in practice. A good example here is the mixing of atomic and non-atomic memory operations (see gpuweb#2229). The spirv-cross shader translation tool casts non-atomic pointers to atomic pointers to support this common pattern, which is technically undefined behavior in C++, but in practice lots of people would be unhappy if the Metal shader compiler did anything other than the reasonable thing. Since Metal's semantics are based on C++, I'd personally love to see this resolved by adopting std::atomic_ref from C++20 (Metal is still based on C++14). I'll also not that the official Metal shader compiler tool generates [reasonable IR] for this pattern. It's concerning that using open source tools such as spirv-cross triggers technical undefined behavior, but it's probably not a big problem in practice.

## Onward

The Metal announcements from WWDC move us no closer to a world of robust GPU infrastructure. But there is much we can still do.

For one, there *is* a GPU infrastructure stack that is based on careful specification and conformance testing, and has two high quality, open source implementations enabling deployment to almost all reasonably current GPU hardware. I speak of course of WebGPU. It's lacking the shiny features – raytracing, bindless, and cooperative matrix operations (marketed as "tensor cores" and quite important for maximum performance in AI workloads) – but what is there should work.

For two, we can cheer on the work of Asahi Linux. They have recently announced [OpenGL 3.1 support] on Apple Silicon, and an intent to implement Vulkan. That work may be highly challenging, as obviously that implies implementing barriers which the Apple GPU engineers haven't been able to manage. But they have done consistently impressive work so far, and I certainly hope they succeed. If nothing else, their work will result in much better public documentation of the hardware's capabilities and limitations.

I have a recommendations for Apple as well. I hope that they document which HLSL features are expected to work and which are not. Currently in their documentation (which is admittedly beta), it just says "Some features not supported," which I personally find not very useful. I would also like to give them credit for clarifying the [Metal Shading Language Specification] with respect to the scope of the `mem_device` flag to `threadgroup_barrier`. It now says, "The flag ensures the GPU correctly orders the memory operations to device memory for threads in the threadgroup," which to a very careful reader does indicate threadgroup scope and no guarantee at device scope. Previously it [said][gpuweb#2297] "Ensure correct ordering of memory operations to device memory," which could easily be misinterpreted as providing a device scope guarantee.

[Metal shader converter]: https://developer.apple.com/metal/shader-converter/
[Prefix sum on portable compute shaders]: https://raphlinus.github.io/gpu/2021/11/17/prefix-sum-portable.html
[Direct3D 11]: https://en.wikipedia.org/wiki/Direct3D#Direct3D_11
[CDSChecker]: http://plrg.eecs.uci.edu/software_page/42-2/
[loom]: https://github.com/tokio-rs/loom
[OpenGL 3.1 support]: https://asahilinux.org/2023/06/opengl-3-1-on-asahi-linux/
[gpuweb#2297]: https://github.com/gpuweb/gpuweb/pull/2297
[Metal Shading Language Specification]: https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
[SAM prefix sum]: https://dl.acm.org/doi/10.1145/2980983.2908089
[Vello]: https://github.com/linebender/vello
[extension list at vulkan.gpuinfo.org]: https://vulkan.gpuinfo.org/listfeaturesextensions.php
[Shader Model 5]: https://learn.microsoft.com/en-us/windows/win32/direct3dhlsl/d3d11-graphics-reference-sm5
[ghpuweb#2229]: https://github.com/gpuweb/gpuweb/issues/2229
[std::atomic_ref]: https://en.cppreference.com/w/cpp/atomic/atomic_ref
[reasonable IR]: https://gist.github.com/raphlinus/a8e0a3a3683127149b746eb37822bdc8
