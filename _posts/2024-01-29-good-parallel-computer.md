---
layout: post
title:  "I want a good parallel computer"
date:   2025-03-21 10:30:42 -0700
categories: [gpu]
---
The GPU in your computer is about 10 to 100 times more powerful than the CPU, depending on workload. For real-time graphics rendering and machine learning, you are enjoying that power, and doing those workloads on a CPU is not viable. Why aren't we exploiting that power for other workloads? What prevents a GPU from being a more general purpose computer?

I believe there are two main things holding it back. One is an impoverished execution model, which makes certain tasks difficult or impossible to do efficiently; GPUs excel at big blocks of data with predictable shape, such as dense matrix multiplication, but struggle when the workload is dynamic. Second, our languages and tools are inadequate. Programming a parallel computer is just a lot harder.

Modern GPUs are also extremely complex, and getting more so rapidly. New features such as mesh shaders and work graphs are two steps forward one step back; for each new capability there is a basic task that isn't fully supported.

I believe a simpler, more powerful parallel computer is possible, and that there are signs in the historical record. In a slightly alternate universe, we would have those computers now, and be doing the work of designing algorithms and writing programs to run well on them, for a very broad range of tasks.

Last April, I gave a colloquium at the UCSC CSE program with the same title [UCSC Colloquium] (video). This blog is a companion to that.

## Memory efficiency of sophisticated GPU programs

I’ve been working on Vello, an advanced 2D vector graphics renderer, for many years. The CPU uploads a scene description in a simplified binary SVG-like format, then the compute shaders take care of the rest, producing a 2D rendered image at the end. The compute shaders parse tree structures, do advanced computational geometry for [stroke expansion], and sorting-like algorithms for binning. Internally, it's essentially a simple compiler, producing a separate optimized byte-code like program for each 16x16 pixel tile, then interpreting those programs. What it cannot do, a problem I am increasingly frustrated by, is run in bounded memory. Each stage produces intermediate data structures, and the number and size of these structures depends on the input in an unpredictable way. For example, changing a single transform in the encoded scene can result in profoundly different rendering plans.

The problem is that the buffers for the intermediate results need to be allocated (under CPU control) before kicking off the pipeline. There are a number of imperfect solutions. We could estimate memory requirements on the CPU before starting a render, but that's expensive and may not be precise, resulting either in failure or waste. We could try a render, detect failure, and retry if buffers were exceeded, but doing readback from GPU to CPU is a big performance problem, and creates a significant architectural burden on other engines we'd interface with.

The details of the specific problem are interesting but beyond the scope of this blog post. The interested reader is directed to the [Potato] design document, which explores the question of how far you can get doing scheduling on CPU, respecting bounded GPU resources, while using the GPU for actual pixel wrangling. It also touches on several more recent extensions to the standard GPU execution model, all of which are complex and non-portable, and none of which quite seem to solve the problem.

Fundamentally, it shouldn't be necessary to allocate large buffers to store intermediate results. Since they will be consumed by downstream stages, it's far more efficient to put them in queues, sized large enough to keep enough items in flight to exploit available parallelism. Many GPU operations internally work as queues (the standard vertex shader / fragment shader / rasterop pipeline being the classic example), so it's a question of exposing that underlying functionality to applications. The [GRAMPS] paper from 2009 suggests this direction, as did the [Brook] project, a predecessor to CUDA.

There are a lot of potential solutions to running Vello-like algorithms in bounded memory; most have a fatal flaw on hardware today. It's interesting to speculate about changes that would unlock the capability. It's worth emphasizing, I'm not feeling held back by the amount of parallelism I can exploit, as my approach of breaking the problem into variants of prefix sum easily scales to hundreds of thousands of threads. Rather, it's the inability to organize the overall as stages operating in parallel, connected through queues tuned to use only the amount of buffer memory needed to keep everything smoothly, as opposed to the compute shader execution model of large dispatches separated by pipeline barriers.

## Parallel computers of the past

The lack of a good parallel computer today is especially frustrating because there were some promising designs in the past, which failed to catch on for various complex reasons, leaving us with overly complex and limited GPUs, and extremely limited, though efficient, AI accelerators.

### Connection Machine

I'm listing this not because it's a particularly promising design, but because it expressed the dream of a good parallel computer in the clearest way. The first Connection Machine shipped in 1985, and contained up to 64k processors, connected in a hypercube network. The number of individual threads is large even by today's standards, though each individual processor was extremely underpowered.

Perhaps more than anything else, the CM spurred tremendous research into parallel algorithms. The pioneering work by Blelloch on [prefix sum] was largely done on the Connection Machine, and I find early paper on [sorting on CM-2] to be quite fascinating.

![Photo of Connection Machine 1 computer, with lots of flashing red LEDs](/assets/teco_connection_machine.jpg) \
[Connection Machine 1 (1985) at KIT / Informatics / TECO] • by KIT TECO • CC0

### Cell

Another important pioneering parallel computer was Cell, which shipped as part of the PlayStation 3 in 2006. That device shipped in fairly good volume (about 87.4 million units), and had fascinating applications including [high performance computing][PlayStation 3 cluster], but was a dead end; the Playstation 4 switched to a fairly vanilla Radeon GPU.

Probably one of the biggest challenges in the Cell was the programming model. In the version shipped on the PS3, there were 8 parallel cores, each with 256kB of static RAM, and each with 128 bit wide vector SIMD. The programmer had to manually copy data into local SRAM, where a kernel would then do some computation. There was little or no support for high level programming; thus people wanting to target this platform had to painstakingly architect and implement parallel algorithms.

All that said, the Cell basically met my requirements for a "good parallel computer." The individual cores could run effectively arbitrary programs, and there was a global job queue.

The Cell had approximately 200 GFLOPS of total throughput, which was impressive at the time, but pales in comparison to modern GPUs or even a modern CPU (Intel i9-13900K is approximately 850 GFLOPS, with a medium-high end Ryzen 7 is 3379 GFLOPS).

### Larrabee

Perhaps the most poignant road not taken in the history of GPU design is the Larrabee. The [2008 SIGGRAPH paper][Larrabee paper] makes a compelling case, but ultimately the project failed. It's hard to say why exactly, but I think it's possible it was just poor execution on Intel's part, and with more persistence and a couple of iterations to improve the shortcomings in the original version, it might well have succeeded. At heart, Larrabee is a standard x86 computer with wide (512 bit) SIMD units and just a bit of special hardware to optimize graphics tasks. Most graphics functions are implemented in software. If it had succeeded, it would very easily fulfill my wishes; work creation and queuing is done in software and can be entirely dynamic at a fine level of granularity.

Bits of Larrabee live on. The upcoming AVX10 instruction set is an evolution of Larrabee's AVX-512, and supports 32 lanes of f16 operations. In fact, Tom Forsyth, one of its creators, argues that [Larrabee did not indeed fail][Why didn't Larrabee fail?] but that its legacy is a success. Another valuable facet of legacy is ISPC, and Matt Pharr's blog on [The story of ispc] sheds light on the Larrabee project.

Likely one of the problems of Larrabee was power consumption, which has emerged as one of the limiting factors in parallel computer performance. The fully coherent (total store order) memory hierarchy, while making software easier, also added to the cost of the system, and since then we've gained a lot of knowledge in how to write performant software in weaker memory models.

Another aspect that definitely held Larrabee back was the software, which is always challenging, especially for new directions. The drivers didn't expose the special capabilities of the highly programmable hardware, and performance on traditional triangle-based 3D graphics scenes was underwhelming, but even with a standard OpenGL interface it did quite well on CAD workloads involving lots of antialiased lines.

## The changing workload

Even within games, compute is becoming a much larger fraction of the total workload (for AI, it's everything). Recent analysis of [Starfield] by Chips and Cheese shows that about half the time is in compute. The [Nanite] renderer also uses compute even for rasterization of small triangles, as hardware is only more efficient for triangles above a certain size. As games do more image filtering, global illumination, and primitives such as Gaussian splatting, the trend will almost certainly continue.

In 2009, Tim Sweeney gave a thought-provoking talk entitled [The end of the GPU roadmap], in which he proposed that the concept of GPU would go away entirely, replaced by a highly parallel general purpose computer. That has not come to pass, though there has been some movement in that direction: the Larrabee project (as described above), the groundbreaking [cudaraster] paper from 2011 implemented the traditional 3D rasterization pipeline entirely in compute, and found (simplifying quite a bit) that it was about 2x slower than using fixed function hardware, and more recent academic GPU designs based on grids of RISC-V cores. It's worth noting, a more recent [update from Tellusim] suggests that cudaraster-like rendering in compute is closer to parity.

An excellent 2017 presentation, [Future Directions for Compute-for-Graphics] by Andrew Lauritzen, highlighted many of the challenges of incorporating advanced compute techniques into graphics pipelines. There's been some progress since then, but it speaks to many of the same problems I'm raising in this blog post. Also see [comments by Josh Barczak], which also links the [GRAMPS] work discusses issues with language support.

## Paths forward

I can see a few ways to get from the current state to a good parallel computer. Each basically picks a starting point that might have been on the right track but got derailed.

### Big grid of cores: Cell reborn

The original promise of the Cell still has some appeal. A modern high end CPU chip has north of 100 billion transistors, while a reasonably competent RISC CPU can be made with orders of magnitude fewer. Why not place hundreds or even thousands of cores on a chip? For maximum throughput, put a vector (SIMD) unit on each core. Indeed, there are at least two AI accelerator chips based on this idea: [Esperanto] and Tenstorrent. I'm particularly interested in the latter because its [software stack][TT-Metal] is open source. 

That said, there are most definitely challenges. A CPU by itself isn't enough, it also needs high bandwidth local memory and communications with other cores. One reason the Cell was so hard to program is that the local memory was small and needed to be managed explicitly - your program needed to do explicit transfers through the network to get data in and out. The trend in CPU (and GPU) design is to virtualize everything, so that there's an abstraction of a big pool of memory that all the cores share. You'll still want to make your algorithm cache-aware for performance, but if not, the program will still run. It’s *possible* a sufficiently smart compiler can adapt a high-level description of the problem to the actual hardware (and this is the approach taken by Tenstorrent’s [TT-Buda] stack, specialized to AI workloads), but in an analogous approach to exploiting instruction-level parallelism through VLIW, the Itanium is a cautionary tale.

From my read of the Tenstorrent docs, the matrix unit is limited to just matrix multiplication and a few supporting operations such as transpose, so it's not clear it would be a significant speedup for complex algorithms as needed in 2D rendering. But I think it's worth exploring, to see how far it can be pushed, and perhaps whether practical extensions to the matrix unit to support permutations and so on would unlock more algorithms.

Most of the “big grid of cores” designs are targeted toward AI acceleration, and for good reason: it is hungry for raw throughput with low power costs, so alternatives to traditional CPU approaches are appealing. See the [New Silicon for Supercomputers] talk for a great survey of the field.

### Running Vulkan commands from GPU-side

A relatively small delta to existing GPUs would be the ability to dispatch work from a controller mounted on the GPU and sharing address space with the shaders. In its most general form, users would be able to run threads on this controller that could run the full graphics API (for example, Vulkan). The programming model could be similar to now, just that the thread submitting work is running close to the compute units and therefore has dramatically lower latency.

In their earliest form, GPU’s were not distributed systems, they were co-processors, tightly coupled to the host CPU’s instruction stream. These days, work is issued to the GPU by the equivalent of async remote procedure calls, with end-to-end latency often as high as 100µs. This proposal essentially calls for a return to less of a distributed system model, where work can efficiently be implemented on a much finer grain and with much more responsiveness to the data. For dynamic work creation, latency is the most important blocker.

Note that GPU APIs are slowly inventing a more complex, more limited version of this anyway. While it’s not possible to run the Vulkan API directly from a shader, with a recent Vulkan extension ([VK_EXT_device_generated_commands]) it is possible to encode some commands into a command buffer. Metal has this capability as well (see [gpuweb#431] for more details about portability). It’s worth noting that the ability to run indirect commands is one of the missing functions; it seems that the designers did not take Hofstadter to heart.

It is interesting to contemplate actually running Vulkan API directly from a shader. Since the Vulkan API is expressed in terms of C, one of the requirements is the ability to run C. This is being done on an experimental basis (see the [vcc] project), but is not yet practical. Of course, CUDA *can* run C. CUDA 12.4 also has support for [conditional nodes], and as of 12.0 it had support for [device graph launch], which reduces latency considerably.

### Work graphs

[Work graphs] are a recent new extension to the GPU execution model. Briefly, the program is structured as a graph of nodes (kernel programs) and edges (queues) all running in parallel. As a node generates output, filling its output queues, the GPU dispatches kernels (at workgroup granularity) to process those outputs further. To a large extent, this is a modern reinvention of the [GRAMPS] idea.

While exciting, and very likely useful for an increasing range of graphics tasks, work graphs also have serious limitations; I researched whether I could use them for the existing Vello design and found three major problems. First, they cannot easily express joins, where progress of a node is dependent on synchronized input from two different queues. Vello uses joins extensively, for example one kernel to compute a bounding box of a draw object (aggregating multiple path segments), and another to process the geometry within that bounding box. Second, there is no ordering guarantee between the elements pushed into a queue, and 2D graphics ultimately does require ordering (the whiskers of the tiger must be drawn over the tiger’s face). Third, work graphs don’t support variable-size elements.

The lack of an ordering guarantee is particularly frustrating, because the traditional 3D pipeline *does* maintain ordering, among other reasons, to prevent Z-fighting artifacts (for a fascinating discussion of how GPU hardware preserves the blend order guarantee, see [A trip through the Graphics Pipeline part 9]). It is not possible to faithfully emulate the traditional vertex/fragment pipeline using the new capability. Obviously, maintaining ordering guarantees in parallel systems is expensive, but ideally there is a way to opt in when needed, or at least couple work graphs with another mechanism (some form of sorting, which is possible to implement efficiently on GPUs) to re-establish ordering as needed. Thus, I see work graphs as two steps forward, one step back.

### CPU convergent evolution

In theory, when running highly parallel workloads, a traditional multi-core CPU design is doing the same thing as a GPU, and if fully optimized for efficiency, should be competitive. That, arguably, is the design brief for Larrabee, and also motivation for more recent academic work like [Vortex]. Probably the biggest challenge is power efficiency. As a general trend, CPU designs are diverging into those optimizing single-core performance (performance cores) and those optimizing power efficiency (efficiency cores), with cores of both types commonly present on the same chip. As E-cores become more prevalent, algorithms designed to exploit parallelism at scale may start winning, incentivizing provision of even larger numbers of increasingly efficient cores, no matter how underpowered each may be at single-threaded tasks.

An advantage of this approach is that it doesn’t change the execution model, so existing languages and tools can still be used. Unfortunately, most existing languages are poor at expressing and exploiting parallelism at both the SIMD and thread level – shaders have a more limited execution model but at least it’s clear how to execute them in parallel efficiently. And for thread-level parallelism, avoiding performance loss from context switches is challenging. Hopefully, newer languages such as [Mojo] will help, and potentially can be adapted to GPU-like execution models as well.

I’m skeptical this approach will actually become competitive with GPUs and AI accelerators, as there is just a huge gap in throughput per watt compared with GPUs – about an order of magnitude. Also, GPUs (and AI accelerators) won’t be standing still either.

### Maybe the hardware is already there?

It's possible that there is hardware currently shipping that meets my criteria for a good parallel computer, but its potential is held back by software. GPUs generally have a "command processor" onboard, which, in cooperation with the host-side driver, breaks down the rendering and compute commands into chunks to be run by the actual execution units. Invariably, this command processor is hidden and cannot run user code. Opening that up could be quite interesting. A taste of that is in Hans-Kristian Arntzen's notes on implementing work graphs in open source drivers: [Workgraphs in vkd3d-proton].

GPU designs vary in how much is baked into the hardware and how much is done by a command processor. Programmability is a good way to make things more flexible. The main limiting factor is the secrecy around such designs. Even in GPUs with open source drivers, the firmware (which is what runs on the command processor) is very locked down. Of course, a related challenge is security; opening up the command processor to user code increases the vulnerability surface area considerably. But from a research perspective, it should be interesting to explore what's possible aside from security concerns.

Another interesting direction is the rise of "Accelerated Processing Units" which integrate GPUs and powerful CPUs in the same address space. Conceptually, these are similar to integrated graphics chips, but those rarely have enough performance to be interesting. From what I've seen, running existing APIs on this hardware (Vulkan for compute shaders, or one of the modern variants of OpenCL) would also not have good latency for synchronizing work back to the CPU, due to context switching overhead, but it's possible a high priority or dedicated thread might quickly process items placed in a queue by GPU-side tasks. The key idea is queues running at full throughput, rather than async remote procedure calls with potentially huge latency.

## Complexity

Taking a step back, one of the main features of the GPU ecosystem is a dizzying level of complexity. There's the core parallel computer, then lots of special function hardware (and the scope of this is increasing, especially with newer features such as ray tracing), then big clunky mechanisms to get work scheduled and run. Those start with the basic compute shader dispatch mechanism (a 3D grid with x, y, z dimensions, 16 bits each), and then augment that with various [indirect command encoding] extensions.

[Work graphs] also fit into the category of complexifying the execution model to work around the limitations of the primitive 3D grid. I was initially excited about their prospect, but when I took a closer look, I found they were inadequate for expressing any of the producer/consumer relationships in Vello.

There's a lot of accidental complexity as well. There are multiple competing APIs, each with subtly different semantics, which makes it especially hard to write code once and just have it work.

CUDA is adding lots of new features, some of which improve agility as I’ve been wanting, and there is a tendency for graphics APIs to adopt features from CUDA. However, there’s also a lot of divergence between these ecosystems (work graphs can’t be readily adapted to CUDA, and it’s very unlikely graphics shaders will get independent thread scheduling any time soon).

The complexity of the GPU ecosystem has many downstream effects. Drivers and shader compilers are buggy and [insecure], and there is probably no path to really fixing that. Core APIs tend to be very limited in functionality and performance, so there's a dazzling array of extensions that need to be detected at runtime, and the most appropriate permutation selected. This in turn makes it far more likely to run into bugs that appear only with specific combinations of features, or on particular hardware.

All this is in fairly stark contrast to the CPU world. A modern CPU is also dazzlingly complex, with billions of transistors, but it is rooted in a much simpler computational model. From a programmer perspective, writing code for a 25 billion transistor Apple M3 isn't that different from, say, a Cortex M0, which can be made with about 48,000 transistors. Similarly, a low performance RISC-V implementation is a reasonable student project. Obviously the M3 is doing a lot more with branch prediction, superscalar issue, memory hierarchies, op fusion, and other performance tricks, but it's recognizably doing the same thing as a vastly smaller chip.

In the past, there were economic pressures towards replacing special-purpose circuitry with general purpose compute performance, but those incentives are shifting. Basically, if you're optimizing for number of transistors, then somewhat less efficient general purpose compute can be kept busy almost all the time, while special purpose hardware is only justified if there is high enough utilization in the workload. However, as Dennard scaling has ended and we're more constrained by power than transistor count, special purpose hardware starts winning more; it can simply be powered down if it isn't used by the workload. The days of a purely RISC computational model are probably over. What I'd *like* to see replacing it is an agile core (likely RISC-V) serving as the control function for a bunch of special-purpose accelerator extensions. That certainly is the model of the [Vortex] project among others.

## Conclusion

In his talk shortly before retirement, Nvidia GPU architect Erik Lindholm [said][Erik Lindholm talk] (in the context of work creation and queuing systems), "my career has been about making things more flexible, more programmable. It's not finished yet. There's one more step that I feel that needs to be done, and I've been pursuing this at Nvidia Research for many years." I agree, and my own work would benefit greatly. Now that he has retired, it is not clear who will take up the mantle. It may be Nvidia disrupting their previous product line with a new approach as they have in the past. It may be an upstart AI accelerator making a huge grid of low power processors with vector units, that just happens to be programmable. It might be CPU efficiency cores evolving to become so efficient they compete with GPUs.

Or it might not happen at all. On the current trajectory, GPUs will squeeze out incremental improvements on existing graphics workloads at the cost of increasing complexity, and AI accelerators will focus on improving the throughput of slop generation to the exclusion of everything else.

In any case, there is an opportunity for intellectually curious people to explore the alternate universe in which the good parallel computer exists; architectures can be simulated on FPGA like Vortex, and algorithms can be prototyped on multicore wide-SIMD CPUs. We can also start to think about what a proper programming language for such a machine might look like, as frustrating as it is to not have real hardware to run it on.

Progress on a good parallel computer would help my own little sliver of work, trying to make a fully parallel 2D renderer with modest resource requirements. I've got to imagine it would also help AI efforts, potentially unlocking sparse techniques that can't run on existing hardware. I also think there's a golden era of algorithms that *can* be parallel but aren't a win on current GPUs, waiting to be developed.


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
[TT-Buda]: https://tenstorrent.com/en/software/tt-buda
[Google TPU]: https://arxiv.org/abs/1704.04760
[Hexagon NPU]: https://chipsandcheese.com/2023/10/04/qualcomms-hexagon-dsp-and-now-npu/
[Jim Keller AI hardware summit talk]: https://www.youtube.com/watch?v=lPX1H3jW8ZQ
[Transputer]: https://en.wikipedia.org/wiki/Transputer

[stroke expansion]: https://linebender.org/gpu-stroke-expansion-paper/
[GRAMPS]: https://dl.acm.org/doi/10.1145/1477926.1477930
[Brook]: https://graphics.stanford.edu/papers/brookgpu/brookgpu.pdf

[prefix sum]: https://www.cs.cmu.edu/~guyb/papers/Ble93.pdf
[sorting on CM-2]: https://www.cs.umd.edu/class/fall2019/cmsc714/readings/Blelloch-sorting.pdf
[Cell]: https://en.wikipedia.org/wiki/Cell_(processor)
[PlayStation 3 cluster]: https://en.wikipedia.org/wiki/PlayStation_3_cluster
[A trip through the Graphics Pipeline part 9]: https://fgiesen.wordpress.com/2011/07/12/a-trip-through-the-graphics-pipeline-2011-part-9/
[Workgraphs in vkd3d-proton]: https://github.com/HansKristian-Work/vkd3d-proton/blob/workgraphs/docs/workgraphs.md
[The Hardware Lottery]: https://hardwarelottery.github.io/
[Larrabee paper]: https://web.archive.org/web/20210307230536/https://software.intel.com/sites/default/files/m/9/4/9/larrabee_manycore.pdf
[Potato]: https://docs.google.com/document/d/1gEqf7ehTzd89Djf_VpkL0B_Fb15e0w5fuv_UzyacAPU/edit?usp=sharing
[Why didn't Larrabee fail?]: https://tomforsyth1000.github.io/blog.wiki.html#%5B%5BWhy%20didn%27t%20Larrabee%20fail%3F%5D%5D
[The story of ispc]: https://pharr.org/matt/blog/2018/04/18/ispc-origins
[Erik Lindholm talk]: https://ubc.ca.panopto.com/Panopto/Pages/Viewer.aspx?id=880a1d92-30d7-4683-80e7-b1e000f501d3
[UCSC Colloquium]: https://www.youtube.com/watch?v=c52ziyKOArc
[vcc]: https://shady-gang.github.io/vcc/
[VK_EXT_device_generated_commands]: https://registry.khronos.org/vulkan/specs/latest/man/html/VK_EXT_device_generated_commands.html
[gpuweb#431]: https://github.com/gpuweb/gpuweb/issues/431
[conditional nodes]: https://developer.nvidia.com/blog/dynamic-control-flow-in-cuda-graphs-with-conditional-nodes/
[device graph launch]: https://developer.nvidia.com/blog/enabling-dynamic-control-flow-in-cuda-graphs-with-device-graph-launch/
[Mojo]: https://en.wikipedia.org/wiki/Mojo_(programming_language)
[work graphs]: https://devblogs.microsoft.com/directx/d3d12-work-graphs/
[Future Directions for Compute-for-Graphics]: https://openproblems.realtimerendering.com/s2017/index.html
[comments by Josh Barczak]: http://www.joshbarczak.com/blog/?p=1317
[update from Tellusim]: https://tellusim.com/compute-raster/
[Connection Machine 1 (1985) at KIT / Informatics / TECO]: https://www.flickr.com/photos/teco_kit/24095266110/