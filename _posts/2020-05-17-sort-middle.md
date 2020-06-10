---
layout: post
title:  "A sort-middle architecture for 2D graphics"
date:   2020-06-10 08:49:42 -0700
categories: [rust, graphics, gpu]
---
In my recent [piet-gpu update], I wrote that I was not satisfied with performance and teased a new approach. I'm on a quest to systematically figure out how to get top-notch performance, and this is a report of one station I'm passing through.

To recap, piet-gpu is a new high performance 2D rendering engine, currently a research protoype. While most 2D renderers fit the vector primitives into a GPU's rasterization pipeline, the brief for piet-gpu is to fully explore what's possible using the compute capabilities of modern GPU's. In short, it's a software renderer that is written to run efficiently on a highly parallel computer. Software rendering has been gaining more attention even for complex 3D scenes, as the traditional triangle-centric pipeline is less and less of a fit for high-end rendering. As a striking example, the new [Unreal 5] engine relies heavily on compute shaders for software rasterization.

The new architecture for piet-gpu draws heavily from the 2011 paper [High-Performance Software Rasterization on GPUs] by Laine and Karras. That paper describes an all-compute rendering pipeline for the traditional 3D triangle workload. The architecture calls for sorting in the middle of the pipeline, so that in the early stage of the pipeline, triangles can be processed in arbitrary order to maximally exploit parallelism, but the output render still correctly applies the triangles in order. In 3D rendering, you can *almost* get away with unsorted rendering, relying on Z-buffering to decide a winning fragment, but that would result in "Z-fighting" artifacts and also cause problems for semitransparent fragments.

The original piet-metal architecture tried to avoid an explicit sorting step by traversing the scene graph from the root, each time. The simplicity is appealing, but it also required redundant work and limited the parallelism that could be exploited. The new architecture adopts a similar pipeline structure as the Laine and Karras paper, but with 2D graphics "elements" in place of triangles.

Central to the new piet-gpu architecture, the scene is represented as a contiguous sequence of these elements, each of which has a fixed-size representation. The current elements are "concatenate affine transform", "set line width", "line segment for stroke", "stroke previous line segments", "line segment for fill", and "fill previous line segments", with of course many more elements planned as the capability of the renderer grows.

While triangles are more or less independent of each other aside from the order of blending the rasterized fragments, these 2D graphics elements are a different beast: they affect graphics *state,* which is traditionally the enemy of highly parallel approaches. Filled outlines present another challenge: the effects are non-local, as the interior of a filled shape depends on the winding number as influenced by segments of the outline that may be very far away. It is not obvious how a pipeline designed for more or less independent triangles can be adapted to such a stateful model. This post will explain how it's done.

## Scan

In general, a sequence of operations, each of which manipulates state in some way, must be evaluated sequentially. An extreme example is a cryptographic hash such as SHA-256. A parallel approach to evaluating such a function would upend our understanding of computation.

However, in certain cases parallel evaluation is quite practical, in particular when the change to state can be modeled as an associative operation. The simplest nontrivial example is counting; just divide up the input into *partitions,* count each partition, then sum those.

Can we design an associative operation to model the state changes made by the elements of our scene representation? Almost, and as we'll see, it's close enough.

At this stage in the pipeline, there are three components to our state: the stroke width, the current affine transform, and the bounding box. Written as sequential pseudocode, our desired state manipulation looks like this:

```
Input an element.
If the previous element was "fill" or "stroke", reset the bounding box.
If the element is:
    "set line width", set the line width to that value.
    "concatenate transform", set the transform to the current transform times that.
    "line segment for fill," compute bounding box, and accumulate.
    "line segment for stroke," compute bounding box, expand by line width, and accumulate.
    "fill", output accumulated bounding box
    "stroke", output accumulated bounding box
```

Note that most graphics APIs have a "save" operation that pushes a state onto a stack, and "restore" to pop it. Because we desire our state to be fixed-size, we'll avoid those. Instead, to simulate a "restore" operation, if the transform was changed since the previous "save," the CPU encodes the inverse transform (this requires that transforms be non-degenerate, but this seems like a reasonable restriction).

As the result of such processing, each element in the input is annotated with a bounding box, which will be used in later pipeline stages for binning. The bounding box of a line segment is just that segment (expanded by line width in the case of strokes), but for a stroke or fill it is the union of the segments preceding it.

Given the relatively simple nature of the state modifications, we can design an "almost monoid" with an almost associative binary operator. I won't give the whole structure here (it's in the code as [`element.comp`]), but will sketch the highlights.

When modeling such state manipulations, it helps to think of the state changes performed by some contiguous slice of the input, then the combination of the effects of two contiguous slices. For example, either or both slices can set the line width. If the second slice does, then at the end the line width is that value, no matter what the first slice did. If it doesn't, then the overall effect is the same as the first slice.

The effect on the transform is even simpler, it's just multiplication of the affine transforms, already well known to be associative (but not commutative).

Where things get slightly trickier is the accumulation of the bounding boxes. The union of bounding boxes is an associative (and commutative) operator, but we also need a couple of flags to track whether the bounding box is reset. However, in general, affine transformations and bounding boxes don't distribute perfectly; the bounding box resulting from that affine transformation of a bounding box might be larger than the bounding box of transforming the individual elements.

[[Image showing that]]

For our purposes, it's ok for the bounding box to be conservative, as it's used only for binning. If we restricted transforms to axis-aligned, or if we used a convex hull rather than bounding rectangle, then transforms would distribute perfectly and we'd have a true monoid. But close enough.

When I wrote my [prefix sum] blog post, I had some idea it might be useful in 2D, but did not know at that time how central it would be. Happily, that implementation could be adapted to handle the transform and bounding box calculation with only minor changes, and it's lightning fast, as we'll see below in the performance discussion.

Note that the previous version of piet-gpu (and piet-metal before it) required the CPU to compute the bounding box for each "item." Part of the theme of the new work is to offload as much as possible to the GPU, including bounding box

## Binning

While element processing is totally different than triangle processing in the Laine and Karras paper, binning is basically the same. The purpose of binning is fairly straightforward: divide the render target surface area into "bins" (256x256 pixels in this implementation), and for each bin output a list of elements that touch the bin, based on the bounding boxes as determined above.

If you look at the code, you'll see a bunch of concern about `right_edge`, which is in service of the backdrop calculation, which we'll cover in more detail below.

The binning stage is generally quite similar to cudaraster. Only a small number (16) of workgroups are launched, just enough to keep all the threads in the GPU busy. Each workgroup work-steals a partition of elements in the input, and puts its binning output into a per-workgroup, per-bin queue. These queues don't interfere with each other, so no synchronization is needed. Also, since the partitions are taken in order, within a queue the elements are in sorted order.

The binning stage is also quite fast, not contributing significantly to the total render time.

## Coarse rasterization

This pipeline stage was by far the most challenging to implement, both because of the grueling performance requirements and because of how much logic it needed to incorporate.

The core of the coarse rasterizer is very similar to cudaraster. Internally it works in processes, each cycle consuming 256 elements from the bin until all elements in the bin have been processed.

* The first stage merges the bin outputs, restoring the elements to sorted order. This stage repeatedly reads chunks generated in the binning stage until 256 elements are read (or the end of the input is reached).

* Next is the input stage, with each thread reading one element. It also compute the coverage of that element, effectively painting a 16x16 bitmap. There's special handling of backdrops as well, see below.

* The total number of line segments is counted, and space in the output is allocated using an atomic add.

* As in cudaraster, segments are output in a highly parallel scheme, with all output segments evenly divded between threads, so each thread has to do a small stage to find its work item.

* The commands for a tile are then written sequentially, one tile per thread. This lets us keep track of per-tile state, and there are many fewer commands than segments.

### Backdrop

A special feature of coarse rasterization for 2D vector graphics is the filling of the interior of shapes. The general approach is similar to [RAVG][Random-Access Rendering of General Vector Graphics]; when an edge crosses the top edge of a tile, a "backdrop" is propagated to all tiles to the right, up to the right edge of the fill's bounding box.

While conceptually fairly straightforwrd, the code to implement this efficiently covers a number of stages in the pipeline. For one, the right edge of fills must be propagated back to segments within the fill, even in early stages such as binning.

* The first right edge of each partition is recorded in the aggregate for each partition in element processing.

* The right edge is computed for each segment in binning, and recorded in the binning output. This logic also adds segments to the bin, when they cross the top edge of a tile.

* In the input stage of coarse rasterization, segments that cross the top edge of a tile "draw" 1's into the bitmap for all tiles to the right, again up to the right edge of the fill. The sign of the crossing is also noted in a separate bitmap, but that doesn't need to be both per-element and per-tile, as it is consistent for all tiles.

* In the output stage of coarse rasterization, bit counting operations are used to sum up the backdrop. Then, if there are any path segments in the tile, the backdrop is recorded in the path command. Otherwise, if the backdrop is nonzero, a solid color command is output.

## Fine rasterization

The fine rasterization stage was almost untouched from the previous piet-gpu iteration. I was very happy with the performance of that; the problem we're trying to solve is efficient preparation of tiles for coarse rasterization.

## Encoding and layers

The original dream for piet-gpu (and piet-metal) is that the encoding process is as light as possible, that the CPU really just uploads a representation of the scene, and the GPU processes it into a rendered texture. The new design moves even closer to this dream; in the original, the CPU was responsible for computing bounding boxes, but now the GPU takes care of that.

Even more exciting, the string-like representation opens up a simpler approach to layers than the original piet-metal architecture: just retain the byte representation, and assemble them. In the simplest implementation, this is just memcpy CPU-side before uploading the scene buffer, but the full range of techniques for noncontiguous string representation is available. Previously, the idea was that the scene would be stored as a graph, requiring tricky memory management.

An important special case is font rendering. The outline for a glyph can be encoded once to a byte sequence (generally on the order of a few hundred bytes), then a string of text can be assembled by interleaving these retained encodings with transform commands. In a longer term evolution, even this reference resolution might want to move to GPU as well, especially to enable texture caching, but this simpler approach should be viable as well.

Note that at the current code checkpoint, this vision is not fully realized, as flattening is still CPU-side. Thus, a retained subgraph (particularly a font glyph) cannot be reused across a wide range of zooms. But I am confident this can be done.

## Discussion

Performance is somewhat improved over the previous version of piet-gpu ((TODO: present measurements)), but not as much as I was hoping. Why?

My analysis is that it's a solid implementation of the concept, but that there is a nontrivial cost to carrying an element through the pipeline fully in fully sorted order. One observation is that segments within a path need not be sorted at all, simply ascribed to the correct (path, tile position) tuple. I will be exploring that in a forthcoming blog post.

Another important piece is missing from the current snapshot: GPU-side flattening. Without this, the ability to change scale in transforms is of limited usefulness, as the optimum flattening is scale dependent. Thus, the concept of "layers," or of fonts being rendered fully GPU-side, is not yet realized. I am confident it can be implemented; in this architecture, the easiest place to insert it would be a scene-to-scene transform after element processing but before binning. It could keep sorted order either by doing a prefix sum for allocating output elements, or by atomic allocation in chunks with references to the original elements to indicate sort order.

But, these two issues aside, let's take stock of where we are. We've taken a pipeline originally designed for 3D rendering, and adapted it to 2D. The pipeline itself is quite general: the core of it is just logic to pass elements in scene through to threads that work on individual tiles, with each thread given only the elements that touch that tile, and in sorted order.

The performance of this snapshot is not all bad news. One very appealing aspect is what I call "performance smoothness," which is the absence of performance cliffs. The performance of a large number of simple paths or a smaller number of highly complex paths should be about the same, as the cost is mostly proportional to the total number of elements in the scene. An excessive number of transforms is also not a concern; these are just handled normally in the course of element processing. In the piet-metal architecture, performance was good when each node in the graph had a moderate number of children, otherwise it would degrade. And handling a large number of simple items (as might occur in a scatterplot or other data visualization) would degrade performance because each "tilegroup" does its own traversal of the scene graph. In the current architecture, early stages of the pipeline touch each element once and then efficiently send it to the appropriate bins for further processing.

As I write, I am immersed in solving the performance problems named above. Stay tuned.

I have had the good fortune of sharing ideas and analysis with Patrick Walton of [Pathfinder] fame as I do this work, and I am encouraged to see impressive improvements in a compute branch of Pathfinder he is developing. Pay attention to that as well.

[piet-gpu]: https://github.com/linebender/piet-gpu
[Random-Access Rendering of General Vector Graphics]: http://hhoppe.com/ravg.pdf
[Why are 2D vector graphics so much harder than 3D?]: https://blog.mecheye.net/2019/05/why-is-2d-graphics-is-harder-than-3d-graphics/
[High-Performance Software Rasterization on GPUs]: https://research.nvidia.com/publication/high-performance-software-rasterization-gpus
[piet-gpu update]: (rust/graphics/gpu/2020/05/26/piet-gpu-progress.html)
[Unreal 5]: https://www.eurogamer.net/articles/digitalfoundry-2020-unreal-engine-5-playstation-5-tech-demo-analysis
[Z-fighting]: https://en.wikipedia.org/wiki/Z-fighting
[A High-Performance Software Graphics Pipeline Architecturefor the GPU]: https://arbook.icg.tugraz.at/schmalstieg/Schmalstieg_350.pdf
[Pathfinder]: https://github.com/servo/pathfinder
[prefix sum]: https://raphlinus.github.io/gpu/2020/04/30/prefix-sum.html
