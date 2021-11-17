---
layout: post
title:  "Prefix sum on Vulkan"
date:   2020-04-30 09:16:42 -0700
categories: [gpu]
---
**Update 2020-05-22:** A new section on [forward progress](#forward-progress) has been added, and the discussion of synchronized shuffles has been improved.

**Update 2021-11-17:** See the follow-up post [Prefix sum on portable compute shaders.](https://raphlinus.github.io/gpu/2021/11/17/prefix-sum-portable.html)

Today, there are two main ways to run compute workloads on GPU. One is CUDA, which has a fantastic ecosystem including highly tuned libraries, but is (in practice) tied to Nvidia hardware. The other is graphics APIs used primarily for gaming, which run on a wide variety of hardware, but historically offer much less power than CUDA. Also, the tooling for compute in that space is terrible. Historically, a lot of compute has also been done with OpenCL, but its future is cloudy as it's been officially deprecated by Apple, and GPU vendors are not consistently keeping their OpenCL implementations up to date.

Vulkan has been catching up fast in its raw capabilities, with recent extensions supporting more advanced GPU compute features such as subgroups, pointers, and a memory model. Is it getting to the point where it can run serious compute workloads?

In this blog post are some initial explorations into implementing [prefix sum] on recent Vulkan. I have a rough first draft implementation which suggests that Vulkan might be a viable platform, for a sufficiently persistent implementor.

## Why prefix sum?

As Hacker News user fluffything points out in [this HN thread](https://news.ycombinator.com/item?id=22902274) on my [Taste of GPU compute] talk, prefix sum is an excellent benchmark for evaluating GPU compute languages and runtimes.

For one, it is useful in and of itself. I use it in [font-rs] to integrate fragments of exact-area computations to arrive at the total area coverage for font rendering. It is also used as a primitive in many more operations, including GPU-side dynamic allocation and [compaction].

For two, it is simple. The sequential version can be expressed in just a handful of lines of code:

```python
def prefix_sum(a):
    s = 0
    result = []
    for x in a:
        s += x
        result.append(s)
    return result
```

For three, it is challenging but possible to implement efficiently on GPU. The above code has a strictly sequential dependency, but because addition is associative, it is possible to exploit a great deal of parallelism, and there is literature on that going back decades. Even so, efficiently exploiting that parallelism on GPU requires communication between invocations ("threads" in more common GPU lingo) and careful attention to the memory hierarchy.

The generalization of prefix sum is called "scan," and works with any associative operation, not just addition. It doesn't even have to be commutative; examples of that include [regular expressions] and [IIR filtering]. More precisely, a scan can be done with any [monoid], a structure with an identity element as well as the associative operation; the identity element is required for the "exclusive" variant of scan, as it is the first element of the output.

## Implementation on GPU

The state of the art is [decoupled look-back]. I'm not going to try to summarize the algorithm here, but recommend reading the paper. The results are impressive — for large data sets, they report reaching memcpy speeds, meaning that no further speedup is possible.

That work is a refinement of [Parallel Prefix Sum (Scan) with CUDA](https://developer.nvidia.com/gpugems/gpugems3/part-vi-gpu-computing/chapter-39-parallel-prefix-sum-scan-cuda) from Nvidia's GPU Gems 3 book. A production-quality, open source implementation is [CUB]. Another implementation, designed to be more accessible but not as optimized, is [ModernGPU scan].

My own [implementation] is very much a research-quality proof of concept. It exists as the [prefix] branch of the [piet-gpu] repository. Basically, I wanted to determine whether it was possible to come within a stone's throw of memcpy performance using Vulkan compute kernels. It's a fairly straightforward implementation of the decoupled look-back paper, and doesn't implement all the tricks. For example, the look-back is entirely sequential; I didn't parallelize the look-back as suggested in section 4.4 of the paper. This is probably the easiest performance win to be gotten. But it's not too horrible, as the partition size is quite big; each workgroup processes 16ki elements. Rough measurements indicate that look-back is on the order of 10-15% of the total time.

The implementation is enough of a rough prototype I don't yet want to do careful performance evaluation, but initial results are encouraging: it takes 2.05ms of GPU time to compute the prefix sum of 64Mi 32-bit unsigned integers on a GTX 1080, a rate of 31.2 billion elements/second. Since each element involves reading and writing 4 bytes, that corresponds to a raw memory bandwidth of around 262GiB/s. The theoretical memory bandwidth is listed as 320GB/s, so clearly the code is able consume a large fraction of available memory bandwidth.

### Do we need a memory model?

One of the achievements of "modern C++" is the C++11 memory model. In the olden days, the mechanism for lock-free programming patterns in C and C++ was the `volatile` qualifier and various nonstandard barrier intrinsics. People reasoned about these operationally — the primary function of `volatile` was to disable certain optimizations, and the barrier intrinsics compile to a memory fence instruction, which generally cause hardware to flush caches.

Today, most lock-free aficionados consider those times to be barbaric. The semantics of `volatile` were never clearly defined for the purpose of multithreading (though people used it anyway, because it appeared to be useful), and the barrier instructions had the disturbing property of being hardware specific. Because x86 has "total store order," barrier instructions are generally not needed for [publication safety]. However, the same code on, say, ARM, which has more weakly ordered memory semantics, would fail, often in subtle ways.

With the C++11 memory model, the programmer specifies the needed ordering constraints precisely. The compiler can then optimize the program very aggressively, as long as it meets those constraints. For example, acquire and release semantics (the basis of publication safety) will compile to explicit memory fence instructions on ARM, but to nothing on x86. A good writeup is the blog post [C++ atomics and memory ordering].

The new [Vulkan memory model] brings the same idea to GPU compute. I used it in my code, in large part because I wanted to experiment with it. I've done a fair amount of lock-free code using the C++ memory model. And lock-free code, while fairly rare on the CPU (my main motivation is to avoid priority inversion for real time audio), is more or less required on the GPU, because mutex is not available in kernel code. Even if it were, it would create a lot of problems, as it would block the entire subgroup, not just a single thread (one of the features of the Vulkan memory model is a much weaker forward progress guarantee than threads running on CPU).

Is a memory model absolutely required to run this code? If you replace the atomic loads and stores with simple array accesses, it deadlocks. However, at least on my hardware, correct operation can be recovered by adding the `volatile` qualifier to the `WorkBuf` array. As with older style C++, there are two risks. Though it seems to work reliably and efficiently on my hardware, it's possible the `volatile` qualifier and explicit fences cause more cache flushing than is needed, or suppress other optimizations that might be possible with a more precise expression of the memory semantics. Alternatively, other hardware or drivers might optimize even more aggressively and break the code.

We're already seeing variation in hardware that requires different levels of vigilance for memory semantics. On most GPU hardware, the invocations (threads) within a subgroup (warp) execute in lock-step, and thus don't require any synchronization. However, as of Nvidia Volta, the hardware is capable of [independent thread scheduling](https://docs.nvidia.com/cuda/volta-tuning-guide/index.html#sm-independent-thread-scheduling). Correct code will add explicit memory semantics even within a subgroup, which will, as in total store order on x86, compile to nothing on hardware that runs invocations in lock-step, while code which just assumes lock-step execution of subgroups will start failing as Vulkan implementations on newer GPUs start scheduling invocations on a more fine grained basis, just as code that assumed total store order failed on CPUs with more relaxed memory consistency, such as ARM.

Note that Vulkan with independent thread scheduling is still work in progress. Shuffles (and related subgroup operations) must synchronize so that all active threads can participate. CUDA 9 solves this problem by introducing new intrinsics such as `__shfl_sync`, which take an additional argument identifying which threads are active. The Vulkan subgroup operations aren't defined this way, and instead implicitly operate on active threads (invocations). Supporting this functionality correctly stresses current compiler technology, including preventing illegal code motion of shuffle intrinsics, and there are threads on the LLVM mailing list discussing this in some detail.

In my research for this blog post, I did not come across any evidence of people actually using the Vulkan memory model, i.e. no search hits for the relevant identifiers other than work associated with the spec. Thus, one contribution of this blog post is to show a concrete example of code that uses it.

### Forward progress

The prototype code has one important flaw, though it appears to run fine on my hardware: it depends on other workgroups making forward progress while it's waiting for the aggregate to be published. The Vulkan spec is careful to make only a [limited forward progress guarantee], and this is not strong enough to reliably run the prefix sum algorithm as written. Thus, there's a risk the program will hang while one workgroup is waiting on an aggregate, and the workgroup that is responsible for computing it is never scheduled because the forward progress guarantee is not strong enough.

Forward progress is a complex problem, and still in flux. A very good summary of the issue is the paper [GPU schedulers: how fair is fair enough?], which describes the needed forward progress guarantee as "occupancy-bound." An earlier paper from the same group, [Forward Progress on GPU Concurrency](https://johnwickerson.github.io/papers/forwardprogress_concur2017.pdf), might be a more accessible presentation of the core ideas. In their experiments, all GPUs they tested meet this guarantee, so it sounds like a good property to standardize. Apple mobile GPUs, however, do not provide this guarantee, though it might take a great deal of testing to uncover a counterexample. As that paper describes, many (but probably not all) other GPUs likely meet the "occupancy-bound" guarantee, so my prefix sum code will run correctly, but I'm not aware of any Vulkan implementations that actually document such a guarantee.

Meanwhile, other devices provide even stronger guarantees. CUDA 9 on Volta and above provides the much stronger [parallel forward progress] guarantee as standardized in C++, and they are able to do this because of independent thread scheduling (see the relevant section of [Inside Volta] for more discussion). This allows even individual threads in a "warp" (subgroup) to hold a mutex and block on other threads without fear of starvation. Another great resource on how Volta improved forward progress is the CppCon 2017 talk [Designing (New) C++ Hardware](https://youtu.be/86seb-iZCnI?t=2043), which I've timestamped for the forward progress discussion. Unfortunately, currently this guarantee is only valid for CUDA, not (yet) Vulkan. In the meantime, from what I understand, Nvidia hardware meets the occupancy-bound guarantee in both CUDA and Vulkan, which is good enough to run prefix sum.

I think it's likely that over time, a consensus will emerge on formalizing the occupancy-bound guarantee, because it's so useful, and at the least you'll be able to query the GPU to determine the level of forward progress guarantee it provides.

In the meantime, it's best to be conservative. Fortunately, for prefix sum, there is a fix (not yet implemented) that restores correct operation even on devices with the weakest forward progress properties: instead of simply spinning waiting from the aggregate from another partition, do a small bit of work towards recomputing the aggregate yourself. After a finite number of cycles, the aggregate for the partition will be done, then you can give up spinning and go to the next partition. This will guarantee getting the result eventually, and hopefully such performance-sapping events are rare.

### Dynamic allocation on GPU

On GPU, it's easiest to run workloads that use static allocation, for example a fixed size buffer per workgroup, and workgroups arranged in a 2D grid ("dispatch" operations support 1D and 3D as well). But dynamic allocation is possible, with care.

The two major approaches to dynamic allocation are prefix sum and atomic bump allocation. The main reason for one over the other is whether you care about the ordering. Let's take a simple problem of computing some function on an array of input values, where the output is variable sized.

Using a prefix-sum approach, you run a first pass of computing the size of the output. The prefix sum of that result yields an offset into an output buffer. The second pass (after the prefix sum) computes the function and writes it into the output buffer, using the offset provided by the prefix sum. [Also note that if we're getting really fancy, it might be possible to fuse either or both of these passes with the prefix sum itself, decreasing the amount of global memory traffic but increasing register pressure and otherwise constraining efficient use of the memory hierarchy, so the extent to which this helps depends greatly on the exact problem].

An atomic bump allocation approach simply does [`atomicAdd`] on each output, using a bump allocation index (effectively a pointer) as the first argument and the size of the allocation as the second. This yields results broadly similar to the prefix sum approach, but with the outputs in arbitrary order. Perhaps the order is not important, or, alternatively, a sort pass can be applied afterwards (sorting on GPU is another topic with a rich literature).

The two can be combined. For example, it makes sense to do a prefix sum of the sizes of items within a workgroup, and a single atomic bump allocation for the per-workgroup total.

One problem that might benefit from prefix sum for dynamic allocation is [flattening] Bézier curves to polylines. Each Bézier segment can be computed in parallel, but you generally want to preserve the order of segments within the full path. The flattening algorithm I presented in that blog post (and its [generalization to cubics](https://github.com/linebender/kurbo/pull/105)) fits nicely into this framework — it's already in two passes, where the first computes the number of segments required, and the second can compute the coordinates of each point in the output independently, thus in parallel.

### Subgroups and subgroup size

High performance prefix sum requires coordination between threads — it's possible to extract some parallelism by running O(log n) tree reduction passes, each of which pulls only from the previous pass, but this would be considerably slower than state of the art. Coordination must be at all levels of the hierarchy. GPU compute has always made threadgroup shared memory available for such coordination. An even faster but newer capability is [subgroups][Vulkan Subgroup Tutorial], not yet universally supported.

My prototype code uses subgroups extensively. One serious limitation is that it assumes a subgroup size of 32, which is true for some hardware. However, other hardware has different size subgroups, and then Intel is special.

By default, when compiling a compute kernel, the Intel drivers use a [heuristic] to determine the subgroup size, which can then be 8, 16, or 32. It actually makes sense they use a heuristic, as there's a complex tradeoff. A bigger subgroup means bigger chunks of work, which means less per-chunk overhead, but also fewer registers available per thread, and potentially more wasted work due to divergence. Again, that depends on workloads. For low-probability, expensive conditional work, generally not a good fit for GPU but sometimes unavoidable, wasted work tends to scale with subgroup size.

It might be *possible* to write a kernel that adapts to subgroup size, but there are a number of considerations that make this tricky. One is whether the number of items processed by a workgroup adapts to subgroup size. If so, then the size of the dispatch must be adapted as well. There is an [extension](https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/vkspec.html#features-pipelineExecutableInfo) for the CPU side to query subgroup size of a pipeline, but, sadly, it doesn't seem to be implemented on Intel drivers on Windows, where it would be most useful. (It is, thankfully, in the latest Linux Intel drivers, so hopefully will be coming soon.)

Another problem is querying the subgroup size from inside the kernel, which has a surprising gotcha. By default, the `gl_SubgroupSize` variable is defined to have the value from [VkPhysicalDeviceSubgroupProperties](https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkPhysicalDeviceSubgroupProperties.html), which in my experiment is always 32 on Intel no matter the actual subgroup size.

Newer (Vulkan 1.2) Intel drivers offer the ability to both accurately query and control over the subgroup size, with the [VK_EXT_subgroup_size_control] extension. With that extension, setting the `VK_PIPELINE_SHADER_STAGE_CREATE_ALLOW_VARYING_SUBGROUP_SIZE_BIT_EXT` at pipeline creation time makes `gl_subgroupSize` behave as expected. Also, I can set the subgroup size to 32, and the kernel works fine. Note though that in general, setting a too-large subgroup size can actually make performance worse, as it increases the chance of register spilling.

On RDNA-based AMD cards, the subgroup size extension lets you get subgroups of 32 on RDNA-based AMD cards, though the default is 64.

In practice, the programmer will write multiple versions of the kernel, each tuned for a different subgroup size, then on CPU side the code will query the hardware for supported subgroup sizes and choose the best one that can run on the hardware. Note that, in general, querying the range of supported subgroup sizes requires the subgroup size extension to be reliable, though you do string-matching on the device name to come up with a good guess. Note that the [specialization constants] mechanism is also a good way to tune constant factors like workgroup or buffer sizes without having to recompile kernel source. In any case, the cost and difficulty of this kind of performance tuning is one reason Nvidia has such a strong first-mover advantage.

Brian Merchant has done more exploration into the tradeoff between subgroups and threadgroup shared memory, for a different primitive operation, transpose of 32x32 boolean matrices. That [transpose timing writeup] contains measurements on a variety of hardware, and is recommended to the interested reader.

### What does subgroupInclusiveAdd compile to?

The `subgroupInclusiveAdd` function seems like it's doing a lot — it's performing a prefix sum operation on an entire subgroup's worth of data. Does hardware contain an assembly instruction that directly implements it? What if you want to do an operation other than addition, where there isn't an intrinsic available?

Obviously different hardware will be different, but looking at the Radeon GPU Analyzer output on [Shader Playground](http://shader-playground.timjones.io/a9e2db94ab3bf88e790694ac869f7879) tells us a lot. It generates a tree reduction (the Hillis-Steele algorithm as presented in the [prefix sum] Wikipedia page) with lg(n) stages of subgroup shuffle + add. Since subgroup shuffle is available in Vulkan (but see below), if you were to write out such a reduction you'd be able to get similar results.

On AMD hardware there is one additional twist: [AMD](https://gpuopen.com/amd-gcn-assembly-cross-lane-operations/) has an additional level of hierarchy between subgroup (64 invocations, 1 wavefront) and invocation (thread). Internally, the hardware is organized around a *row* of 16 elements. Access to elements within a row uses a different instruction modifier (`row_shr`) than across the entire wave (`row_bcast` or `wave_ror`, for two examples), and is likely lower latency in the chip. The Vulkan subgroup extensions provide a powerful and portable set of operations, but don't expose all of the lowest-level operations available on the GPU hardware. To squeeze the last few percent of performance, assembly is still useful.

### Portability considerations: DX12

It is tempting to use a portability layer such as gfx-hal to run compute workloads on a variety of graphics APIs. (Other such portability layers include MoltenVK for running Vulkan on top of Metal, and similar work for running [OpenCL on DX12](https://devblogs.microsoft.com/directx/in-the-works-opencl-and-opengl-mapping-layers-to-directx/)). But such an approach is limited to the lowest common denominator — it can't provide capabilities that are missing in the underlying layer.

Here are some of the pain points for DX12:

* No subgroup size control.

* No subgroup shuffle operations — use threadgroup shared memory instead.

* No memory model — use `volatile` and explicit barriers instead.

* No pointers (not particularly useful for this workload, but important for others).

Also note that gfx-hal currently doesn't give access to Shader Model 6 intrinsics (subgroup operations), but there's an [issue](https://github.com/gfx-rs/gfx/issues/3238) and hopefully that will be fixed.

### Portability considerations: Metal

Metal is closer to Vulkan in capabilities (especially newer versions), but still lacks subgroup size control and a memory model.

## A challenge for GPU compute infrastructure

I covered a fair number of GPU compute infrastructure projects in my talk and the associated [GPU resources] list. Since then I've learned of quite a few more:

* [vuda](https://github.com/jgbit/vuda), which runs (SPIR-V) compute workloads using an API similar to the CUDA host API.

* [clvk](https://github.com/kpet/clvk) and [clspv](https://github.com/google/clspv), which run OpenCL workloads on Vulkan.

* [OpenCL 3.0] is announced, with a number of strategies to rescue OpenCL from a fate of irrelevance.

* [oneAPI](https://software.intel.com/en-us/oneapi), which offers a CUDA migration path but aspires to being a portable standard.

I am also optimistic about [WebGPU] becoming a viable platform for compute workloads, both delivered over the web and in native implementations such as [wgpu].

Echoing fluffything's comment, I propose adopting prefix sum as something of a "hello world" benchmark of GPU compute. It's simple enough it should be practical to implement without too much effort (and if not, that's also an important data point), it exercises "advanced" features such as subgroup shuffles, and it's reasonably easy to quantify. When looking at these potential infrastructure projects, ask these questions:

* How close can it get to the performance offered by the hardware?

* How portable is the high-performance result?

* Are there ways to smoothly downgrade on less capable platforms?

The result of my explorations on Vulkan suggest (but do not yet prove) good answers to these questions, but at the expense of doing a lot of the low-level legwork yourself, and programming the kernel in a very low-level style (in GLSL). I think there's a huge opportunity for more sophisticated tools.

Also, I think it's a great benchmark for the emerging field of GPU-friendly languages. Is it possible to express the algorithm in a reasonably high-level manner? If so, does it compile to code with competitive performance? Can we write a high-performance abstraction as a library that can be consumed easily? Can that abstraction offer portability across hardware but hide the complexity from its users? Can you provide your own monoid?

## Conclusion

I've showed that Vulkan can do prefix sum with near state of the art performance. However, I've also outlined some of the challenges involved in writing Vulkan compute kernels that run portably and with high performance. The lower levels of the stack are becoming solid, enabling a determined programmer to ship high performance compute across a wide range of the hardware, but there is also an opportunity for much better tooling at the higher levels. I see a bright future ahead for this approach, as the performance of GPU compute is potentially massive compared with CPU-bound approaches.

Thanks to Brian Merchant, Matt Keeter, and msiglreith for discussions on these topics, and Jason Ekstrand for setting me straight on subgroup size concerns on Intel. I've also enjoyed the benefit of talking with a number of other people working on GPU drivers, which informs the section on forward progress in particular, though of course any mistakes remain my own.

There is some interesting [HN discussion](https://news.ycombinator.com/item?id=23035194) of this post.

[prefix sum]: https://en.wikipedia.org/wiki/Prefix_sum
[Taste of GPU compute]: https://news.ycombinator.com/item?id=22880502
[font-rs]: https://github.com/raphlinus/font-rs
[compaction]: http://www.davidespataro.it/cuda-stream-compaction-efficient-implementation/
[regular expressions]: http://blog.sigfpe.com/2009/01/fast-incremental-regular-expression.html
[IIR filtering]: https://raphlinus.github.io/audio/2019/02/14/parallel-iir.html
[decoupled look-back]: https://research.nvidia.com/publication/single-pass-parallel-prefix-scan-decoupled-look-back
[Vulkan memory model]: https://www.khronos.org/blog/comparing-the-vulkan-spir-v-memory-model-to-cs
[ModernGPU scan]: https://moderngpu.github.io/scan.html
[CUB]: https://nvlabs.github.io/cub/
[C++ atomics and memory ordering]: https://bartoszmilewski.com/2008/12/01/c-atomics-and-memory-ordering/
[publication safety]: https://bartoszmilewski.com/2008/08/04/multicores-and-publication-safety/
[`atomicAdd`]: https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/atomicAdd.xhtml
[transpose timing writeup]: https://github.com/bzm3r/transpose-timing-tests/blob/master/POST.md
[VK_EXT_subgroup_size_control]: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VK_EXT_subgroup_size_control.html
[GPU resources]: https://raphlinus.github.io/gpu/2020/02/12/gpu-resources.html
[flattening]: https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
[heuristic]: https://software.intel.com/en-us/forums/opencl/topic/564990
[Vulkan Subgroup Tutorial]: https://www.khronos.org/blog/vulkan-subgroup-tutorial
[wgpu]: https://github.com/gfx-rs/wgpu
[WebGPU]: https://www.w3.org/community/gpu/
[OpenCL 3.0]: https://www.khronos.org/news/press/khronos-group-releases-opencl-3.0
[monoid]: https://en.wikipedia.org/wiki/Monoid
[implementation]: https://github.com/linebender/piet-gpu/blob/prefix/piet-gpu-hal/examples/shader/prefix.comp
[prefix]: https://github.com/linebender/piet-gpu/tree/prefix
[piet-gpu]: https://github.com/linebender/piet-gpu
[specialization constants]: https://blogs.igalia.com/itoral/2018/03/20/improving-shader-performance-with-vulkans-specialization-constants/
[limited forward progress guarantee]: https://www.khronos.org/blog/comparing-the-vulkan-spir-v-memory-model-to-cs#_limited_forward_progress_guarantees
[GPU schedulers: how fair is fair enough?]: https://www.cs.princeton.edu/~ts20/files/concur2018.pdf
[parallel forward progress]: https://en.cppreference.com/w/cpp/language/memory_model#Parallel_forward_progress
[Inside Volta]: https://devblogs.nvidia.com/inside-volta/
