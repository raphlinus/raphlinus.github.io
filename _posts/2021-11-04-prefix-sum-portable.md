---
layout: post
title:  "Prefix sum on portable compute shaders"
date:   2021-11-17 07:04:42 -0800
categories: [gpu]
---
This is a followup to my previous post, [prefix sum on Vulkan]. Last year, I got a fancy algorithm for this problem running well on one device. This time, I will dive into the question of how to make it work well across a wide range of devices. The short answer is, it can be made to work pretty portably on Vulkan and DX12, but Metal remains out of reach, at least for now, as is WebGPU. Even on Vulkan, there are some sharp edges to watch out for.

When writing code that runs on CPU, most developers don't spend a lot of time agonizing over the infrastructure. You write your code in some source language, let's say Rust, type `cargo build`, and you get a nice self-contained executable that you can run on your system or distribute to others. Then that just runs. There might be some interesting things going on under the hood, such as runtime detection of SIMD capabilities, but that's mostly hidden. Things work.

That is sadly not the case for GPU compute code. The most common scenario is dependency on a large, vendor-dependent toolkit such as CUDA (that installer is a 2.4GB download). If you have the right hardware, and the runtime installed properly, *then* your code can run. [Update: I need to find a better way to say this. CUDA is a heavyweight dependency for sure, but it is possible to build an executable that runs, at least on Nvidia hardware.]

What's standing in the way of a clean simple executable that can run GPU code? This is something I've been exploring for at least the last year, as I'd like to ship piet-gpu as a 2D rendering engine that just works, rather than something that requires excessive fiddling with GPU infrastructure. To that end, I've built an abstraction layer that pretty much works, at least for my needs - you write a compute shader once, in GLSL, and it runs it on Vulkan, Metal, or DX12, dynamically detecting which is supported at runtime. A binary that runs a simple shader is around 400k.

I've been using prefix sum as a test case, for a variety of reasons. It's a good motivating example for advanced atomic features, as the decoupled look-back algorithm can run considerably faster (about 1.5x) than the more basic tree reduction approach. It exercises advanced features of GPU compute infrastructure. And finally, it's useful; it's the basis for the [element processing pipeline] in piet-gpu, so I do care about making it run reliably and fast.

A promising alternative to rolling my own abstraction layer is the emerging WebGPU standard, with at least two active open source implementations, [wgpu] in Rust, and [Dawn] in C++. This will be a huge upgrade to the web's GPU capabilities from the current WebGL 2. In addition, as argued in the essay [Point of WebGPU on native], it also promises to be a good solution for native binaries.

Given these choices of compute runners, now can we run the fancy decoupled look-back algorithm? The answer is complex. It is possible to get it running well in some cases, but it is still quite far from "just works."

## WebGPU

I'll start with WebGPU. Though I was initially able to get a version running on the Vulkan backend, it did not run on either DX12 or Metal, for different reasons. And as I explored more deeply, I found that, sadly, WebGPU in its present form cannot run the decoupled look-back algorithm at all.

Metal is the biggest problem. It took a while to figure this out, but Metal is simply missing an atomic barrier type that is necessary to run the algorithm. Decoupled look-back is an advanced usage of GPUs in that each workgroup doesn't work independently, but rather coordinates with other workgroups to send and receive their partial results. At the heart of that coordination is an atomic acquire/release pair (the message-passing idiom) that has scope that encompasses both those workgroups. This can be implemented either with acquire/release atomic semantics (still not present in Metal) or barriers. Unfortunately, the `threadgroup_barrier(mem_flags::mem_device)` call turns out to actually have threadgroup (workgroup) scope, which is not enough - it needs device scope. As far as I can tell, there is simply no way to run decoupled look-back on Metal (unless the payload can be packed into a 32 bit word, which is not the case for the types I want to scan in piet-gpu). And if it can't be done in Metal, it can't be part of the WebGPU spec, otherwise it would be unimplementable.

This limitation of Metal wasn't clear at the outset. Partly as a result of my investigation, the [WebGPU spec has changed][WebGPU barrier change] to reflect the weaker barrier capabilities.

Even if this barrier were to be added, I found [other problems][Atomic concerns]. One such problem is uniformity analysis, which originally showed up as a failure to compile the shader on DX12, as the [FXC] compiler's uniformity analysis failed to recognize that my shader had uniform control flow, and thus rejected it. WebGPU will also need a strict uniformity analysis when doing operations such as control barriers that are unsafe unless control flow is uniform, and it appears that the [current proposal][uniformity analysis proposal] also wouldn't accept my code. It is possible that could be improved, though.

The fact that FXC is on the critical path points to a deeper difference in approach between WebGPU and piet-gpu. By necessity, shaders pulled from the Web must be compiled at runtime, and FXC is considerably more convenient for that than the more advanced [DXC], as FXC is included in Windows and can be called by any application. The DLL for DXC is approximately 20MB.

In piet-gpu, by contrast, shaders are compiled to IR ahead of time as part of the build pipeline, so there is no need for runtime translation. That helps a lot with binary size and startup time, as well as a lot more flexibility in tools that can be run as part of the build.

The Windows story with Dawn (Chrome's WebGPU implementation) continues to evolve. Currently they run with their own version of FXC, which brings compatibility to the widest range of devices (there's still a large fraction that can't run code from DXC) and lets them roll fixes if the version that ships with the OS is stale. I'm concerned that even so, there will be shaders that fail because of FXC limitations. There are a number of ways forward, including direct translation from WGSL into DXBC (and DXIL as well, for access to [Shader Model 6] capabilities), but at best this work will take a while.

There is a [WebGPU implementation of decoupled look-back] by Reese Levine, but at this point it's not expected to pass. Hopefully it will be useful to shake out bugs (it would pass if the barrier scope were upgraded), and also to point the way for extensions to WebGPU that would allow this algorithm to run.

### Shader translation

Another concern I have about WebGPU is that it is difficult to maintain shaders as single authoritative source files and translate them automatically into WGSL. For all other shader languages, we've arrived at a reasonable solution involving GLSL source and translation using spirv-tools to HLSL and Metal (enabling translation to intermediate representation as well). That won't work for WGSL, and, not surprisingly, the problem is atomics.

The fundamental issue is that [atomics][Atomics proposal] in WGSL are strictly typed, following the precedent of Metal, which in turn follows C++. By contrast, in HLSL and GLSL, atomics are operations that can work on ordinary memory locations. Arguably, that's not ideal, but it is how shaders are written today. That doesn't create a portability problem in practice, because Metal also supports unsafe C++ casting operations, and this pattern is widely used (by spirv-cross and perhaps ironically also [Tint]) to compile the standard atomics of shader languages into Metal.

WGSL can't expose unsafe casting, so that's not a viable solution. Unfortunately, this means that translation into WGSL is not a straightforward problem, but in some cases might require rearchitecting the shader and the pipeline that contains it. A very common pattern (used in piet-gpu) is to treat memory as a big array of 32 bit integers (RWByteAddressBuffer), and store all the types in there. It's primitive, but low level shader languages aren't really expressive enough to represent richer types properly, especially if you want to pack them efficiently or have dynamic layout. That doesn't work when some of the types have atomic fields; they have to be in a separate space recognized by the compiler.

Maintaining source in WGSL and translating to other shader languages would be viable, but that would also limit the code to using only least common denominator features. Perhaps WGSL will grow a rich set of optional extensions, in which case it might make sense. In any case, at least for my needs, it is a fairly major point of friction.

It's worth noting that the Vulkan memory model faced a similar choice when they were defining the semantics of atomics for Vulkan, and came to a different decision than WGSL. As is clear from [their blog post,][Atomic Operations vs Atomic Objects] they recognize the importance of working with existing shaders and shader languages, even if it isn't as aesthetically pleasing as a strictly typed approach.

While I've been disappointed to run into these problems, I remain optimistic about the prospects for WebGPU to run real compute loads. I just think it'll take a while to get there. In the meantime, I will continue to track progress and help where I can with things like test cases and spec clarifications.

## GPU bugs

I've encountered a lot of GPU bugs in my work on piet-gpu. That's largely because it's off the beaten path, trying to do advanced things with compute shaders that haven't been done before. I've probably spent about half my time in the last three weeks tracking down exciting crashes and hangs, trying to figure out whether it was a problem with my code, a spec, my understanding of the spec, or the GPU driver.

My approach to bugs is a little different than most people who work with GPUs. While I have been doing quite a bit of experimentation, I'm also trying to work with specs in a principled way, so there is a solid argument my code is correct with respect to the spec. Then, when there is a failure, once we figure out it isn't in my code, I try to create a reduced test case to clearly demonstrate GPU behavior that is not compliant with the spec. And then I would like to get some form of that test case into the official test suite, so it hopefully gets fixed and stays fixed. I believe this will help the entire ecosystem.

It's expected that decoupled look-back should run well on Nvidia hardware, as the algorithm was invented there, but less so on other hardware. While it is mainstream to structure compute pipelines so each dispatch can depend on data written in previous stages, there is something slightly taboo about coordination between workgroups in the same dispatch. Many GPU experts I've talked with express skepticism that this can work at all. Even so, interest in this type of pattern is picking up, in part because of advanced rendering engines like [Nanite][A Deep Dive into Nanite], which also uses atomics to coordinate work between workgroups in a live-running dispatch. Future GPUs will be expected to run this type of workload reliably; right now there's a flavor of hacking around until it works on a specific GPU, then hoping a driver update doesn't break things.

One device I tested, the AMD 5700 XT running under Windows with Vulkan, had a very interesting pattern of failures. The "compatibility mode" variant of prefix sum (which uses barriers and coherent buffers, but no explicit atomic loads or stores) worked just fine, and with impressive performance to boot (around 48 G elements/s, basically the same as memcpy). But the other two variants, based on atomics, failed badly. I spent some time tracking this down, and was ultimately able to reduce the failure to a version of the classical message-passing atomic litmus test. This was surprising to me, as there's also a version of that test in the [Vulkan Conformance Test Suite][Vulkan CTS]. Why didn't that catch it?

That's still a bit of a mystery, but there are two properties of my version that might explain it. For one, it's much more parallel, stressing the system and giving more opportunities to observe failure. And second, it also permutes the addresses of reads and writes in a way that may be better at triggering atomic coherence issues. I've proposed this [permuted parallel message passing test] and hope it gets added in some way to the CTS.

I'm also starting to populate the tests/ subdirectory of the [piet-gpu repo] with these tests as I come up with them, as well as tests of pipeline stages in the renderer. It's my hope that this will become a useful resource for people who want to test GPUs in ways that might not be well covered by existing tests.

## Progress on progress

One controversial aspect of the original decoupled look-back algorithm is that it depends on a forward progress guarantee from the GPU. If a workgroup is running, and an adversarial scheduler unfairly schedules threads that are waiting on the flag in favor of threads that would set it if they were run, then the dispatch as a whole will hang (this is similar to a deadlock in a classical setting, but a bit more subtle). My code from last year also depended on this guarantee, and while it ran well on the hardware I tested on, it might not everywhere. The Vulkan specification itself is careful to make no forward progress guarantees.

There are a number of interesting workloads that depend on or benefit from these kind of properties, likely including the Nanite renderer. To help those applications, Tyler Sorensen's group has been characterizing existing GPUs, with the aim of defining a GPU property that might be queried at runtime. Their latest paper is [Specifying and Testing GPU Workgroup Progress Models]. Among its findings, Apple and ARM exhibit failures of forward progress, so it cannot be specified in the Vulkan core, but only as an optional property.

Separately, Elias Naur coded up a modification to my code that makes a bit of scalar progress instead of spinning uselessly. Thus, even in the face of completely adversarial scheduling, the code is guaranteed to complete, it might just be slow. Hopefully, those types of events are rare, and statistically, performance is good. It seems OK so far, but we haven't done extremely careful testing. The piet-gpu test suite might be useful for identifying whether there are any GPUs where this is a real problem.

## A note on DX11

I explored DX11 compatibility this time around, but did not actually build the port. I believe it is possible, but would take some work and involve some compromises. One challenge is the relatively simplistic uniformity analysis in FXC mentioned above; it doesn't seem to accept the relatively common pattern (in advanced compute) of broadcasting a value from one thread to other threads in a workgroup through a shared variable, at least in the presence of "interesting" control flow. I could hand-craft a version that does all the decoupled look-back logic on single thread, and uses pure memory fences rather than barriers involving control flow synchronization, so it gets by FXC. In fact I have written a [draft of that][FXC workaround], but didn't feel it was worth the work, at least at this time.

Based on experience of other projects, I also think it's likely that DX11, and FXC in particular, would be more likely to trigger shader compilation bugs than actively maintained toolchains. At some point, I hope to port piet-gpu to DX11 as well, but I expect to stick to shaders optimized for compatibility, rather than using advanced features, even if DX11 can theoretically support them.

There will always be devices where the GPU is not capable enough to run the workload, or is on a blocklist because of bugs that preclude correct execution. In those cases, I think [SwiftShader] is the best way forward; it compiles the shader to run on the CPU, but using SIMD and multithreading extensively for potentially big speedups compared with scalar code. Elias Naur has also done successful experiments with running SwiftShader ahead of time, and that might also be a promising avenue to reduce the runtime costs of shader translation.

## Other projects

I'm not the only person trying to run interesting workloads on compute shaders. In this section, I want to point to other projects that people should follow.

### IREE

Arguably the most ambitious project currently targeting portable compute shaders is [IREE], which has a strong machine learning focus - it's basically a way to get TensorFlow deployed to more devices. Currently their main focus is Vulkan, but ports to CUDA and Metal are also on their roadmap. It compiles models written in a much higher language to GPU code (for example, SPIR-V on Vulkan), then runs those on the target device.

### rust-gpu

Writing GLSL is painfully low-level, and WGSL isn't much of an improvement. I'd love to write compute shaders in a real language, and one very promising approach is [rust-gpu]. The choice of higher level shader language is more or less orthogonal to the runner infrastructure; Vulkan is defined in terms of the SPIR-V intermediate representation, so presumably can run anything that compiles to that. It's still early days for the project, but I'm following it with great interest and looking forward to when it can support interesting compute workloads; they're more focused on 3D rendering at the moment.

### Kompute.cc

The [Kompute] project addresses another side of the problem: it provides a clean way to run compute shaders, but is dependent on Vulkan, and generally the shaders are written in GLSL. Like IREE, they focus on machine learning workloads.

## Summary and conclusion

There are a number of new findings in this blog post, so it might be useful to summarize them.

* Decoupled look-back cannot run on WebGPU because of a [missing synchronization primitive][WebGPU barrier change]. This was surprising to me because of confusion from an earlier version of the spec, which is now clarified.

* Decoupled look-back cannot run on Metal either, which is the underlying reason for the above. Also surprising, because I had imagined otherwise from reading the [Metal Shading Language Specification]. I believe that language will also be clarified.

* I now have a version of my code that is portable and performant on Vulkan and DX12.

* All this is running on a hand-rolled portability layer that works on Vulkan, Metal, and DX12. The more-compatible but slower tree reduction version works fine with low risk of triggering GPU bugs. This layer is a potentially a good starting point for others who want to run compute workloads portably.

* That portability layer also comes with a small, focused test suite designed to uncover these types of bug. Those tests catch some real behavior that slips through existing test suites.

* WebGPU continues to look promising but has a number of serious issues which need to be resolved before it's suitable for heavy compute.

These findings affect decisions in piet-gpu. It's now capable of running compute shaders on a wide range of devices, as long as they're not too aggressive in use of advanced capabilities such as the full set of Vulkan atomics. Thus, in rewriting the element pipeline, I'm going to focus on tree reduction, as it should pose no compatibility problems, and the performance is likely to be "good enough" (the redesign should have other optimizations which I hope will make up for this regression). We can always come back to decoupled look-back later, as the story becomes clearer exactly which devices will support it reliably.

This blog benefitted from discussions with many people, though the (no doubt numerous) mistakes are my own. I'd like to thank in particular Elias Naur for tracking down many Metal and shader issues, Jeff Bolz for spotting a particularly subtle write-after-read hazard and insight into atomic issues, Reese Levine for collaboration on the WebGPU tests, and a number of people working at GPU vendors, who tend not to be used to working in the open, but whose generosity with time and insight I appreciate.

Discuss on [Hacker News](https://news.ycombinator.com/item?id=29254668).

[prefix sum on Vulkan]: https://raphlinus.github.io/gpu/2020/04/30/prefix-sum.html
[Point of WebGPU on native]: https://kvark.github.io/web/gpu/native/2020/05/03/point-of-webgpu-native.html
[IREE]: https://google.github.io/iree/
[naga]: https://github.com/gfx-rs/naga
[Nanite]: https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf
[Kompute]: https://kompute.cc/
[D3DCompiler]: https://docs.microsoft.com/en-us/windows/win32/api/d3dcompiler/nf-d3dcompiler-d3dcompile
[FXC]: https://docs.microsoft.com/en-us/windows/win32/direct3dtools/fxc
[DXC]: https://github.com/microsoft/DirectXShaderCompiler
[element processing pipeline]: https://github.com/linebender/piet-gpu/issues/119
[Dawn]: https://dawn.googlesource.com/dawn
[Tint]: https://dawn.googlesource.com/tint/
[wgpu]: https://wgpu.rs/
[WebGPU barrier change]: https://github.com/gpuweb/gpuweb/pull/2297
[Atomic concerns]: https://github.com/gpuweb/gpuweb/issues/2229
[uniformity analysis proposal]: https://github.com/gpuweb/gpuweb/pull/1571
[permuted parallel message passing test]: https://github.com/KhronosGroup/VK-GL-CTS/issues/295
[WebGPU implementation of decoupled look-back]: https://gpuharbor.ucsc.edu/prefix-sum/
[Specifying and Testing GPU Workgroup Progress Models]: https://users.soe.ucsc.edu/~tsorensen/publication/oopsla2021b/
[Vulkan CTS]: https://github.com/KhronosGroup/VK-GL-CTS
[piet-gpu repo]: https://github.com/linebender/piet-gpu
[Atomics proposal]: https://github.com/gpuweb/gpuweb/issues/1360
[FXC workaround]: https://github.com/linebender/piet-gpu/pull/121
[Atomic Operations vs Atomic Objects]: https://www.khronos.org/blog/comparing-the-vulkan-spir-v-memory-model-to-cs#_atomic_operations_vs_atomic_objects
[Metal Shading Language Specification]: https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
[rust-gpu]: https://github.com/EmbarkStudios/rust-gpu
[A Deep Dive into Nanite]: https://www.youtube.com/watch?v=eviSykqSUUw
[SwiftShader]: https://swiftshader.googlesource.com/SwiftShader
[Shader Model 6]: https://docs.microsoft.com/en-us/windows/win32/direct3dhlsl/hlsl-shader-model-6-0-features-for-direct3d-12
