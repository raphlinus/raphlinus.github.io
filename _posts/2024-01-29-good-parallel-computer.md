---
layout: post
title:  "I want a good parallel computer"
date:   2024-01-29 10:30:42 -0800
categories: [gpu]
---
Much of my research over the past few years has been 2D vector graphics rendering on GPUs. That work goes well, but I am running into the limitations of GPU hardware and programming interfaces, and am starting to see hints that a much better parallel computer may be possible. At the same time, I see some challenges regarding actually getting there. This essay will explore both in depth.

I should qualify, the workload I care about is unusual in a number of respects. Most game workloads involve rasterization of a huge number of triangles, and most AI workloads involve multiplication of large matrices, both very conceptually simple operations. By contrast, 2D rendering has a lot of intricate, conditional logic, and is very compute intensive compared with the raw memory bandwidth needed. Compute shaders on modern GPUs can handle the conditional logic quite well, but lack *agility,* which to me means the ability to make fine-grained scheduling decisions. I believe agility 

## The changing workload

Even within games, compute is becoming a much larger fraction of the total workload (for AI, it's everything). Recent analysis of [Starfield] by Chips and Cheese shows that about half the time is in compute. The [Nanite] renderer also uses compute even for rasterization of small triangles, as hardware is only more efficient for triangles above a certain size. As games do more image filtering, global illumination, and primitives such as Gaussian splatting, the trend will almost certainly continue.

In 2009, Tim Sweeney gave a thought-provoking talk entitled [The end of the GPU roadmap], in which he proposed that the concept of GPU would go away entirely, replaced by a highly parallel general purpose computer. That has not come to pass, though there has been some movement in that direction: the Larrabee project (described in more detail below), the groundbreaking [cudaraster] paper from 2011 implemented the traditional 3D rasterization pipeline entirely in compute, and found (simplifying quite a bit) that it was about 2x slower than using fixed function hardware, and more recent academic GPU designs based on grids of RISC-V cores.

## Larrabee

Probably the most ambitious effort to unify the CPU and GPU worlds was the Larrabee project. It was a failure, and I think that failure set back the field substantially. Bits of it survive, as AVX-512 (soon to evolve into AVX-10), but overall it was not competitive. I think the main thing that sunk it (and also held back AVX-512 adoption) was a very high power budget.

In an alternate universe where Larrabee had succeeded, *initially* it would be running traditional GPU workloads (DirectX), but the hardware would have been capable of exceptional degrees of agility. Computation would be represented as hundreds of physical cores, each fully general purpose CPUs, and each with 16 lanes of predicated SIMD. It's not hard to imagine that a programming interface would have emerged (possibly just C++ with intrinsics for the SIMD) to allow full access to both the throughput and agility of the device.

## Agility

* Agility: ability dispatch work on a fine-grained basis. Not a standard term, maybe come up with something better.

## Complexity

Taking a step back, one of the main features of the GPU ecosystem is a dizzying level of complexity. There's the core parallel computer, then lots of special function hardware (and the scope of this is increasing, especially with newer features such as ray tracing), then big clunky mechanisms to get work scheduled and run. Those start with the basic compute shader dispatch mechanism (a 3D grid with x, y, z dimensions, 16 bits each), but then augment that with various [indirect command encoding] extensions.

[Work graphs] also fit into the category of complexifying the execution model to work around the limitations of the primitive 3D grid. I was initially excited about their prospect, but when I took a closer look, I found they were inadequate for expressing any of the producer/consumer relationships in Vello, for three reasons: lack of joins, no ability to maintain ordering constraints, and fixed size inputs only.

There's a lot of accidental complexity as well. There are multiple competing APIs, each with subtly different semantics, which makes it especially hard to write code once and just have it work.

The complexity of the GPU ecosystem has many downstream effects. Drivers and shader compilers are buggy and [insecure], and there is probably no path to really fixing that. Core APIs tend to be very limited in functionality and performance, so there's a dazzling array of extensions that need to be detected at runtime, and the most appropriate permuation selected. This in turn makes it far more likely to run into bugs that appear only with specific combinations of features, or on particular hardware.

All this is in fairly stark contrast to the CPU world. A modern CPU is also dazzlingly complex, with billions of transistors, but it is rooted in a much simpler computational model. From a programmer perspective, coding for an Apple M3 isn't that different than, say, a Cortex M0, which can be made with about 48,000 transistors. Similarly, a low performance RISC-V implementation is a reasonable student project. Obviously the M3 is doing a lot more with branch prediction, superscalar issue, memory hierarchies, op fusion, and other performance tricks, but it's recognizably doing the same thing as a vastly similar chip.

In the past, there were economic pressures towards replacing special-purpose circuitry with general purpose compute performance, but those incentives are shifting. Basically, if you're optimizing for number of transistors, then somewhat less efficient general purpose compute can be kept busy almost all the time, while special purpose hardware is only justified if there is high enough utilization in the workload. However, as Dennard scaling has ended and we're more constrained by power than transistor count, special purpose hardware starts winning more; it can simply be powered down if it isn't used by the workload. The days of a purely RISC computational model are probably over. What I'd *like* to see replacing it is an agile core (likely RISC-V) serving as the control function for a bunch of special-purpose accelerator extensions. That certainly is the model of the [Vortex] project among others.

## Big grid of RISC-V

There are many, many AI accellerators in the pipeline – see the [New Silicon for Supercomputers] talk for a great survey. One approach (definitely the one taken by the original [Google TPU]) is to sacrifice agility and make hardware that's specialized just for doing big matrix multiplications and essentially nothing else. Another approach, suitable for the low end, is a fairly vanilla VLIW microprocessor with big vector units, an architecture actually quite similar to existing DSPs. That is the approach taken by the [Qualcomm Hexagon]. Neither of these is suitable for running a workload like Vello.

Far more interesting is the "big grid of RISC-V cores" approach. The idea is to achieve throughput simply by having many cores in parallel, a strategy generally in alignment with GPUs. However, all of the special purpose graphics hardware can be stripped out, leaving a focus on pure compute, so potentially a much simpler design. I'm aware of two, as both have provided substantial public detail, but I expect there are quite a few others.

### Tenstorrent

  + [TT-Metal] (Metalium)

### Esperanto

Another approach is [Esperanto], which is about 1000 efficiency RISC-V cores on a chip. The company way founded by Dave Ditzel, previously of Transmeta. The linked paper goes into a fair amount of detail and quantitative measurement. Not surprisingly, it focuses on the AI acceleration use case, but it also appears suitable for HPC workloads. Because it is at heard many CPUs, each running a program independently, it promises great agility. Unfortunately, there's no indication their software stack is open source, so it's hard for me to find out more.

## A research program

I'm tempted to port Vello to the Tenstorrent chip in particular, due to the open source availabilty of the tools, but it's a nontrivial amount of work, and it would be of academic interest, as it's extremely unlikely anyone would want to deploy that hardware just for 2D rendering. Even so, I believe it could answer some intriguing questions about the best way to do parallel computation in the future.

A related project would be to run a 3D renderer, related to [cudaraster] and the software stack of [Vortex]. This project would have considerable overlap with a Vello port, as many aspects of the pipeline (element processing, binning, coarse then fine rasterization) are in common, the main difference being the primitive (triangle vs Bézier path). If the outcome is more pixels per joule than a state-of-the-art GPU, then that would be a massive validation of the grid-of-RV approach, suggesting that the GPU market could be massively disrupted both for graphics and AI workloads. If it is less efficient, that points to a world in which computing hardware continues to become more specialized to the workload it runs.


[Starfield]: https://chipsandcheese.com/2023/10/15/starfield-on-the-rx-6900-xt-rx-7600-and-rtx-2060-mobile/
[Nanite]: https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf
[cudaraster]: https://research.nvidia.com/publication/2011-08_high-performance-software-rasterization-gpus
[The end of the GPU roadmap]: https://web.archive.org/web/20090823200347/http://graphics.cs.williams.edu/archive/SweeneyHPG2009/TimHPG2009.pdf
[indirect command encoding]: https://developer.nvidia.com/blog/new-vulkan-device-generated-commands/
[Vortex]: https://vortex.cc.gatech.edu/
[insecure]: https://chromium.googlesource.com/chromium/src/+/main/docs/security/research/graphics/webgpu_technical_report.md
[Esperanto]: https://www.esperanto.ai/wp-content/uploads/2022/05/Dave-IEEE-Micro.pdf
[New Silicon for Supercomputers]: https://www.youtube.com/watch?v=w3xNLj6nRgs
[TT-Metal]: https://github.com/tenstorrent-metal/tt-metal/
[Google TPU]: https://arxiv.org/abs/1704.04760
[Hexagon NPU]: https://chipsandcheese.com/2023/10/04/qualcomms-hexagon-dsp-and-now-npu/
[Jim Keller AI hardware summit talk]: https://www.youtube.com/watch?v=lPX1H3jW8ZQ