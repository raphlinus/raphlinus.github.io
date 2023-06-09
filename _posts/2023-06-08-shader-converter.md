---
layout: post
title:  "A note on Metal shader converter"
date:   2023-06-09 07:03:42 -0700
categories: [gpu]
---
At WWDC, Apple introduced [Metal shader converter], a tool for converting shaders from DXIL (the main compilation target of HLSL in DirectX12) to Metal. While it is no doubt useful for reducing the cost of porting games from DirectX to Metal, I feel it does not move us any closer to a world of robust GPU infrastructure, and in many ways just adds more underspecified layers of complexity.

The specific feature I'm salty about is atomic barriers that allow for some sharing of work between threadgroups. These barriers are present in HLSL, and in fact have been since 2009, when [Direct3D 11] and Shader Model 5 were first introduced.

## Typed vs untyped atomics

Another challenge for reliable automated translation into Metal is typed vs untyped atomics. In C++, `atomic<int32_t>` and `int32_t` are distinct types, and atomic operations can only be performed on the former. This is a reasonable choice, and I'm generally in favor of relying on the type system to enforce invariants; Rust follows the same tradition.

The problem is that other shader languages, in this case most importantly HLSL, have an *untyped* approach to atomics. A memory location simply has type `uint`, and that can be accessed both through ordinary loads and stores, and with atomic operations (called "interlocked" in HLSL argot). In some cases, atomic and non-atomic accesses can be cleanly separated, in other cases they might be inextricably mixed. The latter happens when a buffer is a [RWByteAddressBuffer](https://learn.microsoft.com/en-us/windows/win32/direct3dhlsl/sm5-object-rwbyteaddressbuffer) which presents as a completely untyped array of 32-bit words, and the actual semantic meaning of types is expressed in program logic above the low-level access to the raw buffer.

Other cases are somewhat in-between. Here's a simple shader that computes the maximum value of each 256 chunk of input:
```hlsl
ByteAddressBuffer input;
RWByteAddressBuffer output;

groupshared uint max_value;

[numthreads(256, 1, 1)]
void main(uint index: SV_GroupIndex) {
    if (index == 0) {
        max_value = 0;
    }
    GroupMemoryBarrierWithGroupSync();
    InterlockedMax(max_value, input.Load(index * 4));
    GroupMemoryBarrierWithGroupSync();
    if (index == 0) {
        output.Store((index / 256) * 4, max_value);
    }
}
```

The initialization and use of `max_value` can be done with non-atomic operations, but of course the max computation needs to be atomic because all the threads are participating in parallel.

Here's the translation of that using DXC and spirv-cross, a combination of open-source tools that accomplishes the same thing as the new Apple tool:

```msl
kernel void main0(const device type_ByteAddressBuffer& _input [[buffer(0)]], device type_RWByteAddressBuffer& _output [[buffer(1)]], uint gl_LocalInvocationIndex [[thread_index_in_threadgroup]])
{
    threadgroup uint max_value;
    bool _26 = gl_LocalInvocationIndex == 0u;
    if (_26)
    {
        max_value = 0u;
    }
    threadgroup_barrier(mem_flags::mem_threadgroup);
    uint _33 = atomic_fetch_max_explicit((threadgroup atomic_uint*)&max_value, _input._m0[(gl_LocalInvocationIndex * 4u) >> 2u], memory_order_relaxed);
    threadgroup_barrier(mem_flags::mem_threadgroup);
    if (_26)
    {
        _output._m0[((gl_LocalInvocationIndex / 256u) * 4u) >> 2u] = max_value;
    }
}
```

The key bit is `(threadgroup atomic_uint*)&max_value`, which is a pointer cast from a non-atomic type to an atomic type. In C++, this is considered undefined behavior. Almost certainly, this should be considered "technical undefined behavior," because if the Metal shader compiler did anything other than the reasonable interpretation, a great many games in the App Store that use spirv-cross to translate shaders from HLSL would be extremely unhappy.

Even so, we're in a position where it's not possible to *reason* about correctness systematically. There's a tradition in lock-free algorithms and data structures where the first publication is almost always flawed, then there's a follow-up that fixes it. It's hard to be confident any of these algorithms are correct until there's been formal verification of some kind. Fortunately, these formal tools exist and are put to good use; there are Alloy formulations of the C++11 memory model, model checking tools such as [CDSChecker] (and its Rust counterpart [loom]), and a small academic industry of proving lock-free algorithms correct. Trying to use these formal techniques to prove correctness of an algorithm translated into Metal would result in an instant report of UB.

## Onward

The Metal announcements from WWDC move us no closer to a world of robust GPU infrastructure. But there is much we can still do.

For one, there *is* a GPU infrastructure stack that is based on careful specification and conformance testing, and has two high quality, open source implementations enabling deployment to almost all reasonably current GPU hardware. I speak of course of WebGPU. It's lacking the shiny features – raytracing, bindless, and cooperative matrix operations (marketed as "tensor cores" and quite important for maximum performance in AI workloads) – but what is there should work.

For two, we can cheer on the work of Asahi Linux. They have recently announced [OpenGL 3.1 support] on Apple Silicon, and an intent to implement Vulkan. That work may be highly challenging, as obviously that implies implementing barriers which the Apple GPU engineers haven't been able to manage. But they have done consistently impressive work so far, and I certainly hope they succeed. If nothing else, their work will result in much better public documentation of the hardware's capabilities and limitations.



[Metal shader converter]: https://developer.apple.com/metal/shader-converter/
[Prefix sum on portable compute shaders]: https://raphlinus.github.io/gpu/2021/11/17/prefix-sum-portable.html
[Direct3D 11]: https://en.wikipedia.org/wiki/Direct3D#Direct3D_11
[CDSChecker]: http://plrg.eecs.uci.edu/software_page/42-2/
[loom]: https://github.com/tokio-rs/loom
[OpenGL 3.1 support]: https://asahilinux.org/2023/06/opengl-3-1-on-asahi-linux/

