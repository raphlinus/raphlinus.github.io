---
layout: post
title:  "Requiem for piet-gpu-hal"
date:   2023-01-07 09:12:42 -0800
categories: [rust, gpu]
---
Recently we switched [Vello] to using [wgpu] for its GPU infrastructure and deleted piet-gpu-hal entirely. This was the right decision, but there was something special about piet-gpu-hal and I hope its vision is realized some day. This post talks about the reasons for the change and the tradeoffs involved. Oh, and as part of the same cycle of changes we renamed piet-gpu to Vello, as there was very little of the original piet-gpu implementation left.

In the process of doing piet-gpu-hal, we learned a *lot* about portable compute infrastructure. We will apply that knowledge to improving WebGPU implementations to better suit our needs.

## Goals

The goals of piet-gpu-hal were admirable, and I believe there is still a niche for something that implements them. Essentially, it was to provide a lightweight yet powerful runtime for running compute shaders on GPU. Those goals are fulfilled to a large extent by WebGPU, it's just that it's not quite as lightweight and not quite as powerful, but on the other hand the developer experience is much better.

The most important goal, especially compared with WebGPU implementations, is to reduce the cost of shader compilation by precompiling them to the intermediate representation (IR) expected by the GPU, rather than compiling them at runtime. This avoids anywhere from about 1MB to about 20MB of binary size for the shader compilation infrastructure, and somewhere around 10ms to 100ms of startup time to compile the shaders.

An additional goal was to unlock the powerful capabilities present on many (but not all) GPUs by runtime query. For our needs, the most important ones are subgroups, descriptor indexing, and detection of unified memory (avoiding the need for separate staging buffers). However, for the coming months we are prioritizing getting things working well for the common denominator, which is well represented by WebGPU, as it's generally possible to work around the lack of such features by doing a bit more shuffling around in memory.

## Implementation choices

In this section, I'll talk a bit about implementation choices made in piet-gpu-hal and their consequences.

### Shader language

After considering the alternatives, we landed on GLSL as the authoritative source for writing shaders. HLSL was in the running, and it seems to be the most popular choice (primarily in the game world) for writing portable shaders, but ultimately GLSL won because it is capable of expressing *all* of what Vulkan can do. In particular, I wanted to experiment with the Vulkan memory model.

Another choice considered seriously was [rust-gpu]. That looks promising, and it has many desirable properties, not least being able to run the same code on CPU and GPU, but just isn't mature enough. Hopefully that will change. I think porting Vello to it would be an interesting exercise, and would shake out many of the issues needing to be solved to use it in production.

Another appealing choice would be [Circle], a C++ dialect that targets compute shaders among other things.

### Ahead of time compilation

From the GLSL source, we also had a pipeline to compile this to shader IR. For all targets, the first step was [glslangValidator] to compile the GLSL to SPIR-V. And for Vulkan, that was sufficient.

For DX12, the next step was [spirv-cross] to convert the SPIR-V to HLSL, followed by [DXC] to convert this to DXIL. We really wanted to use DXC, especially over relying on the system provided shader compiler (some arbitrary version of FXC), both to access advanced features in Shader Model 6 such as wave operations, and also because FXC is somewhat buggy and we have evidence it will cause problems. In fact, I see the current reliance on FXC to be one of the biggest risks for WebGPU working well on Windows. (In the longer term, it is likely that both Tint and naga will compile WGSL directly to DXIL and perhaps DXBC, which will solve these problems, but will take a while)

For Metal, we used spirv-cross to convert SPIR-V to Metal Shading Language. We intended to go one step further, to AIR (also known as metallib), using the [command-line tools][Metal Command-Line Tools], but we didn't actually do that, as it would have made setting up the CI more difficult.

All this was controlled through a simple hand-rolled ninja file. At first, we ran this by hand and committed the results, but it was tricky to make sure all the tools were in place (which would have been considerably worse had we required both DXC with the [signing DLL] in place and the Metal tools), and it also resulted in messy commits, prone to merge conflicts. Our solution to this was to run [shader compilation in CI], which was better but still added considerably to the friction, as it was easy to be in the wrong branch for committing PRs.

### GPU abstraction layer

The core of piet-gpu-hal was an abstraction layer over GPU APIs. Following the tradition of gfx-hal, we called this a HAL (hardware abstraction layer), but that's not quite accurate. Vulkan, Metal, and DX12 are all hardware abstraction layers, responsible among other things for compiling shader IR into the actual ISA run by the hardware, and dispatching compute and other work.

The design was based loosely on gfx-hal, but more tuned to our needs. To summarize, gfx-hal was a layer semantically very close to Vulkan, so that the Vulkan back-end was a very thin layer, and the Metal back-end resembled MoltenVK, with the DX12 back-end also simulating Vulkan semantics in terms of the underlying API. This ultimately wasn't a very satisfying approach, and for wgpu was [deprecated in favor of wgpu-hal].

We only implemented the subset needed for compute, which turned out to be a significant limitation because we found we also needed to run the rasterization pipeline for some tasks such as image scaling. The need to add new functionality to the HAL was also a major friction point. We *were* able to add features like runtime query for advanced capabilities such as subgroup size control, and a polished timer query implementation on all platforms (the latter is still not quite there for wgpu). Overall it worked pretty well.

I make one general observation: all these APIs and HALs are very object oriented. There are objects representing the adapter, device, command lists, and resources such as buffers and images. Managing these objects is nontrivial, especially because the lifetimes are complex due to the asynchronous nature of GPU work submission; you can't destroy a resource until all command buffers referencing it have completed. These patterns are fairly cumbersome, and especially don't translate easily to Rust.

For others seeking to build cross-platform GPU infrastructure, I suggest exploring a more declarative, data-oriented approach. In that approach, build a declarative render graph using simple value types as much as possible, then write specialized engines for each back-end API. This should lead to more straightforward code with less dynamic dispatching, and also resolve the need to find common-denominator abstractions. We are moving in this direction in Vello, and may explore it further as we encounter the need for native back-ends.

## WebGPU pain points

Overall the task of porting piet-gpu to WGSL went well, but we ran into some issues. We expect these to be fixed and improved, but at the moment the experience is a bit rough. In particular, the demo is not running on DX12 at all, either in Chrome Canary for the WebGPU version or through wgpu. The main sticking point is uniformity analysis, which only very recently has a good solution in WGSL ([workgroupUniformLoad]) and the implementation hasn't fully landed.

## Collaboration with community

A major motivation for switching to WGSL and WebGPU is to interoperate better with other projects in that ecosystem. We already have a demo of [Bevy interop], which is particularly exciting.

One such project is [wgsl-analyzer], which gives us interactive language server features as we develop WGSL shader code: errors and warnings, inline type hints, go to definition, and more. Thanks especially to Daniel McNab, they've been super responsive to our needs and we maintain a working configuration. I strongly recommend using such tools; the old way sometimes felt like submitting shader code on a stack of punchcards to the shader compiler.

Obviously another major advantage is deploying to the web using WebGPU, which we have working in at least prototype form. With an [intent to ship] from Chrome and active involvement from Mozilla and Apple, the prospect of WebGPU shipping in usable state on real browsers seems close.

## Precompiled shaders on roadmap?

The goals of a lightweight runtime for GPU compute remain compelling, and in our roadmap we plan to return to precompiled shaders so it is possible to use Vello without a need to carry along runtime shader compilation infrastructure, and also to more aggressively exploit advanced GPU features. There are two avenues we are exploring.

One is to add support for precompiled shaders to either wgpu or wgpu-hal. We have an [issue][wgpu precompiled shaders] with some thoughts. The advantage of this approach is that it potentially benefits many applications developed on WebGPU, for example native binary distributions of games. If it is possible to identify all [permutations] of shaders which will actually be used, and bundle the IR.

The other is to add additional native renderer back-ends. The new architecture is much more declarative and less object oriented, so the module that runs the render graph can be written directly in terms of the target API rather than going through an abstraction layer. If there is a desire to ship Vello for a targeted application (for example, animation playback on Android), this will be the best way to go.

## Other GPU compute clients

One thing we were watching for was whether there was any interest in using piet-gpu-hal for other applications. Matt Keeter did some [experimentation][fidget prototype] with it, but otherwise there was not much interest. Sometimes the "portable GPU compute" space feels a bit lonely.

An intriguing potential application space is machine learning. It would be an ambitious but doable project to get, say, Stable Diffusion running on portable compute using either piet-gpu-hal or something like it, so that very little runtime (probably less than a megabyte of code) would be required. Related projects include [Kompute.cc], which runs machine learning workloads but is Vulkan only, and also [MediaPipe].

One downside to trying to implement machine learning workloads in terms of portable compute shaders is that it doesn't get access to neural accelerators such the [Apple Neural Engine]. When running in native Vulkan, you *may* get access to [cooperative matrix] features, which on Nvidia are branded "tensor cores," but for the most part these are proprietary vendor extensions and it is not clear if and when they might be exposed through WebGPU. Even so, at least on Nvidia hardware it seems likely that using these features can unlock very high performance.

Going forward, one approach I find particularly promising for running machine learning is [wonnx], which implements the ONNX spec on top of WebGPU. No doubt in the first release, performance will lag highly tuned native implementations considerably, but once such a thing exists as a viable open source project, I think it will be improved rapidly. And WebGPU is not standing still...

## Beyond WebGPU 1.0

WebGPU 1.0 is basically a "least common denominator" of current GPUs. This has advantages and disadvantages. It means that there are fewer choices (and fewer permutations), and that code written in WGSL can run on all modern GPUs. The downside is that there are a number of features that most (but not all) current GPUs have that can speed things further, and those features are not available.

The likely path through this forest is to define extensions to WebGPU. These can start as privately implemented extensions, running in native, then, hopefully based on that experience, proposed for standardization on the web. They would be optional, meaning that more shader permutations will need to be written to make use of them when available, or fall back when not. One such extension, fp16 arithmetic, has already been standardized, though we don't yet exploit it in Vello.

In Vello, we have identified three promising candidates for further extension.

First, [descriptor indexing], which is the ability to create an array of textures, then have a shader dynamically access textures from that array. Without it, a shader can only have a fixed number of textures bound. To work around that limitation, we plan to have an image atlas, copy the source images into that atlas using rasterization stages, then access regions (defined by uv quads) from the single binding. We don't expect performance to be that bad, as such copies are fairly cheap, but it does require extra memory and is not ideal. For fully native, the GPU world is moving to [bindless], which is popular in DX12, and the recent [descriptor buffer] extension makes Vulkan work the same way. It is likely that WebGPU will standardize on something more like descriptor indexing than full bindless, because the latter is basically raw pointers and thus unsafe by design. In any case, see the [image resources] issue for more discussion.

Second, device-scoped barriers, which unlock single-pass (decoupled look-back) prefix sum techniques. They are not present in Metal, but otherwise should be straightforward to add. I wrote about this in my [Prefix sum on portable compute shaders] blog post. In the meantime, we are using multiple dispatches, which is much more portable, but not quite as performant.

Third, subgroups, also known as SIMD groups, wave operations, and warp operations. Within a workgroup, especially for stages resembling prefix sum, Vello uses a lot of workgroup shared memory and barriers. With subgroups, it's possible to reduce that traffic, in some cases dramatically. That should make the biggest difference on Intel GPUs, which have relatively slow shared memory.

Unfortunately, there is a tricky aspect to subgroups, which is that in most cases there is no control in advance over the subgroup size, the shader compiler picks it on the basis of a heuristic. The [subgroup size control] extension, which is mandatory in Vulkan 1.3, fixes this, and allows writing code specialized to a particular subgroup size. Otherwise, it will be necessary to write a lot of conditional code and hope that the compiler constant-folds the control flow based on subgroup size. Another challenge is that it is more difficult to test code for a particular subgroup size, in contrast to workgroup shared memory which is quite portable. More experimentation is needed to determine whether subgroups *without* the size control extension work well across a wide range of hardware.

And on the topic of that experimentation, it's difficult to do so without adequate GPU infrastructure. I may find myself reaching for the archived version of piet-gpu-hal.

## Conclusion

Choosing the right GPU infrastructure depends on the goals, as sadly there is not yet a good consensus choice for. For the goals of researching the cutting edge of performance, hand-rolled infrastructure was the right choice, and piet-gpu-hal served that well. For the goal of lowering the friction for developing our engine, and also interoperating with other projects, WebGPU and wgpu are a better choice. Our experience with the port suggests that the performance and features are good enough, and that it is a good experience all-around.

We hope to make Vello useful enough to use in production within the next few months. For many applications, WebGPU will be an appropriate infrastructure. For others, where the overhead of runtime shader compilation is not acceptable, we have a path forward but will need to consider alternatives. Either ahead-of-time shader compilation can be retrofitted to wpgu, or we will explore a more native approach.

In any case, we look forward to productive development and collaboration with the broader community.

[Vello]: https://github.com/linebender/vello
[wgpu]: https://wgpu.rs
[wgpu precompiled shaders]: https://github.com/gfx-rs/wgpu/issues/3103
[indirect command encoding]: https://developer.apple.com/documentation/metal/indirect_command_encoding
[VK_NV_device_generated_commands]: https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_NV_device_generated_commands.html
[ExecuteIndirect]: https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12graphicscommandlist-executeindirect
[Kompute.cc]: https://kompute.cc/
[MediaPipe]: https://google.github.io/mediapipe/
[wonnx]: https://github.com/webonnx/wonnx
[Bevy interop]: https://github.com/linebender/vello/tree/main/examples/with_bevy
[permutations]: https://therealmjp.github.io/posts/shader-permutations-part1/
[workgroupUniformLoad]: https://github.com/gpuweb/gpuweb/pull/3586
[descriptor indexing]: https://chunkstories.xyz/blog/a-note-on-descriptor-indexing/
[bindless]: https://alextardif.com/Bindless.html
[descriptor buffer]: https://www.khronos.org/blog/vk-ext-descriptor-buffer
[image resources]: https://github.com/linebender/vello/issues/176
[Prefix sum on portable compute shaders]: https://raphlinus.github.io/gpu/2021/11/17/prefix-sum-portable.html
[decoupled look-back]: https://research.nvidia.com/publication/2016-03_single-pass-parallel-prefix-scan-decoupled-look-back
[subgroup size control]: https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_EXT_subgroup_size_control.html
[rust-gpu]: https://github.com/EmbarkStudios/rust-gpu
[Circle]: https://www.circle-lang.org/
[DXC]: https://github.com/microsoft/DirectXShaderCompiler
[spirv-cross]: https://github.com/KhronosGroup/SPIRV-Cross
[Metal Command-Line Tools]: https://developer.apple.com/documentation/metal/shader_libraries/building_a_library_with_metal_s_command-line_tools
[glslangValidator]: https://github.com/KhronosGroup/glslang
[signing DLL]: https://www.wihlidal.com/blog/pipeline/2018-09-16-dxil-signing-post-compile/
[shader compilation in CI]: https://github.com/linebender/vello/blob/480e5a5e2fb1ed5c38da083bfa00c1ae6b9b2486/doc/shader_compilation.md
[MoltenVK]: https://github.com/KhronosGroup/MoltenVK
[deprecated in favor of wgpu-hal]: https://gfx-rs.github.io/2021/08/18/release-0.10.html#pure-rust-graphics
[wgsl-analyzer]: https://github.com/wgsl-analyzer/wgsl-analyzer
[intent to ship]: https://groups.google.com/a/chromium.org/g/blink-dev/c/VomzPhvJCxI/m/SUhU9Z0vAgAJ
[fidget prototype]: https://github.com/mkeeter/fidget/blob/1b41b6b8e4bdb017e2ca28c151391a4a080b581a/jitfive/src/metal.rs
[cooperative matrix]: https://www.khronos.org/assets/uploads/developers/presentations/Cooperative_Matrix_May22.pdf
[Apple Neural Engine]: https://github.com/hollance/neural-engine
