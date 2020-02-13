---
layout: post
title:  "GPU resources"
date:   2020-02-12 10:20:42 -0800
categories: [gpu]
---
This post is basically a dump of resources I've encountered while doing a deep dive into GPU programming. I welcome pull requests against the [repo](https://github.com/raphlinus/raphlinus.github.io) for other useful resources. Also feel free to ask questions in issues, particularly if the answer might be in the form of a patch to this post.

## Understanding the hardware

### Intel

Intel is one of the best GPU hardware platforms to understand because it's documented and a lot of the work is open source.

* [Wikichip gen 9](https://en.wikichip.org/wiki/intel/microarchitectures/gen9), [gen 9.5](https://en.wikichip.org/wiki/intel/microarchitectures/gen9.5), [gen 11](https://en.wikichip.org/wiki/intel/microarchitectures/gen11)

* [Intel white paper on Gen9 compute](https://software.intel.com/sites/default/files/managed/c5/9a/The-Compute-Architecture-of-Intel-Processor-Graphics-Gen9-v1d0.pdf)

* [Programmer's Reference Manual](https://01.org/sites/default/files/documentation/intel-gfx-prm-osrc-kbl-vol07-3d_media_gpgpu.pdf) for Kaby Lake (Gen 9.5)

There's also some academic literature:

* [Performance Characterisation and Simulation of Intel’s Integrated GPU Architecture](http://comparch.gatech.edu/hparch/papers/gera_ispass18.pdf)

One of the funky things about Intel is the varying subgroup width; it can be SIMD8, SIMD16, or SIMD32, mostly determined by [compiler heuristic](https://software.intel.com/en-us/forums/opencl/topic/564990), but there is a new [VK_EXT_subgroup_size_control](https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/chap44.html#VK_EXT_subgroup_size_control) extension.

### NVidia

There's a lot of interest and activity around NVidia, but much of it is reverse engineering.

* [Dissecting the NVIDIA Volta GPU Architecture via Microbenchmarking](https://arxiv.org/pdf/1804.06826.pdf)

* [Dissecting the NVidia Turing T4 GPU via Microbenchmarking](https://arxiv.org/pdf/1903.07486.pdf)

## Understanding API capabilities

* [vulkan.gpuinfo.org](https://vulkan.gpuinfo.org/) - a detailed database of what extensions are available on what hardware/driver/platform combinations.

* [Metal Feature Set Tables](https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf) has similar info for Metal.

## Subgroups

Subgroup/warp/SIMD/shuffle operations are very fast, but less compatible (nonuniform shuffle is missing from HLSL/SM6), and you (mostly) don't get to control the subgroup size, so portability is a lot harder.

* [Vulkan Subgroup Tutorial](https://www.khronos.org/blog/vulkan-subgroup-tutorial)

* [Vulkan Subgroup Explained](https://www.khronos.org/assets/uploads/developers/library/2018-vulkan-devday/06-subgroups.pdf)

* [Reading Between The Threads: Shader Intrinsics](https://developer.nvidia.com/reading-between-threads-shader-intrinsics)

## Languages

### GLSL

* [https://github.com/KhronosGroup/glslang](https://github.com/KhronosGroup/glslang) - reference implementation of GLSL, compilation to SPIR-V

* [shaderc](https://github.com/google/shaderc) - Google-maintained tools

### HLSL

* [DirectX Shader Compiler](https://github.com/microsoft/DirectXShaderCompiler) (DXC) - produces both SPIR-V and DXIL.

* [Programming guide for HLSL](https://docs.microsoft.com/en-us/windows/win32/direct3dhlsl/dx-graphics-hlsl-pguide)

* [Shader Model 6](https://docs.microsoft.com/en-us/windows/win32/direct3dhlsl/hlsl-shader-model-6-0-features-for-direct3d-12)

### Metal Shading Language

* [Metal Shading Language Specification](https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf)

### OpenCL

* [clspv](https://github.com/google/clspv) - compile OpenCL C (subset) to run on Vulkan compute shaders.

  * To me, this is evidence that Vulkan will simply eat OpenCL's lunch. This is still [controversial](https://github.com/KhronosGroup/Vulkan-Ecosystem/issues/42), but Khronos people are insisting there's an "OpenCL Next" roadmap.

### TensorFlow

* [MLIR](https://blog.tensorflow.org/2019/04/mlir-new-intermediate-representation.html)

### Exotic languages

* [Halide](https://halide-lang.org/)

* [Futhark](https://futhark-lang.org/)

* [Co-dfns](https://github.com/Co-dfns/Co-dfns)

* [Julia on GPU](https://juliacomputing.com/domains/gpus.html) - layered on CUDA

## SPIR-V

* [SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross) - transpile SPIR-V into GLSL, HLSL, and Metal Shading Language

  * This is an integral part of portability layers including [MoltenVK](https://github.com/KhronosGroup/MoltenVK) and [gfx-rs](https://github.com/gfx-rs/gfx).

## WebGPU

* [Building WebGPU with Rust](https://fosdem.org/2020/schedule/event/rust_webgpu/) - FOSDEM talk

* [wgpu](https://github.com/gfx-rs/wgpu)

* [dawn](https://dawn.googlesource.com/dawn) - Google's WebGPU implementation

* [Get started with GPU Compute on the Web](https://developers.google.com/web/updates/2019/08/get-started-with-gpu-compute-on-the-web) - Google (Chromium/Dawn)

### WebGPU shader language

The discussion of shader language had been very [contentious](https://news.ycombinator.com/item?id=22020511). As of very recently there is a proposal for a textual language that is semantically equivalent to SPIR-V, and there seems to be agreement that this is the path forward.

* [Tint - WebGPU F2F - Feb 12, 2020](https://docs.google.com/presentation/d/1qHhFq0GJtY_59rNjpiHU--JW4bW4Ji3zWei-gM6cabs/edit)

* [Minutes for GPU Web meeting 2020-02-12 Redmond F2F](https://docs.google.com/document/d/1vQPA1JSOvfCHjBrkAEDLA1qCqQXe72vGen_1quoHZV8/edit#)

The previous proposals were some profile of SPIR-V, a binary format, and Apple's [Web High Level Shading Language](https://webkit.org/blog/8482/web-high-level-shading-language/) proposal, which evolved into [Web Shading Language](https://github.com/gpuweb/WSL). Both of these had disadvantages that made them unacceptable to various people. It's not possible to use SPIR-V directly, largely because it has undefined behavior and other unsafe stuff. The Google and Mozilla implementations addressed this by doing a rewrite pass. Conversely, Apple's proposal met with considerable resistance because it didn't deal with the diversity of GPU hardware in the field. There's a lot of ecosystem work centered around Vulkan and SPIR-V, and leveraging that will help WebGPU considerably.
