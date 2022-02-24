---
layout: post
title:  "piet-gpu progress: clipping"
date:   2022-02-18 07:33:42 -0800
categories: [rust, graphics, gpu]
---
Recently piet-gpu has taken a big step towards realizing its [vision], moving the computation of path clipping from partially being done on the CPU to entirely being done on the GPU.

This post explains how that works. It's been quite a journey - I actually started a draft of this more than a year ago. Since then, I've had to come up with a fundamental new parallel algorithm. It's finally done. I think clipping is very much at the heart of what makes piet-gpu different than other 2D rendering engines.

## What is clipping?

The basic idea of clipping is simple, but it has a profound impact on the overall imaging model. For one, it induces a *tree structure,* while drawing without clipping can be seen as a linear sequence of layers. There are different ways of implementing pure vector clipping, but the one I've gone with generalizes to *blending,* which is next.

Clipping with a vector path affects the drawing of all *child objects* of the clip node. Points inside the path are drawn, and points outside the path are not.

![Example of clipping](/assets/clip_demo.svg)

For pure vector path clipping, the effective clip applied to each (leaf) drawing node is the intersection of all clip paths up the tree. Thus, one viable implementation is to do this path intersection in vector space. However, boolean path operations are tricky to implement, and only CPU implementations are known; moving them to the GPU would be difficult, to say the least.

Rather, we treat (antialiased) clipping as a *blend* operation. Conceptually, the children of a clip are rendered into a temporary, intially clear buffer, the clip mask is rendered into an alpha channel, then the temporary buffer is composited on the background, the alpha channel multiplied by the mask. This approach can be done fully on the GPU, and the blending operations generalize.

Clips can nest arbitrarily; a clip node can be the child of another clip node. Thus, a full 2D scene is effectively a *tree,* which profoundly affects the way drawing is done.

## Bounding boxes

A common technique in 2D rendering is to assign a *bounding box* to each drawing operation. The bounding box is an enclosing rectangle (hopefully tight), so that drawing operations affect only pixels inside the rectangle. For rendering any subregion of the target surface, any objects whose bounding box does not intersect that bounding box may be ignored. Piet-gpu uses bounding boxes extensively to organize parallel drawing, first by 256x256 pixel bins, then 16x16 pixel tiles. Bounding boxes exploit the fact that most 2D drawing is *sparse,* in that most draw objects don't touch most tiles.

Bounding boxes of course interact with clipping. Each clip node in the tree has a bounding box, computed from the corresponding clip path. Then, the bounding boxes for all descendants in the tree is intersected with that bounding box. Another way of phrasing this is that the clipped bounding box for a draw object is the object's own bounding box intersected with the bounding boxes of all clip paths in the path from that node to the root of the tree.

Computing these bounding boxes is less work than rendering the objects, but is not free. In particular, we'd like to do that work on the GPU rather than the CPU.

## Per-tile optimization

Things get even more interesting with per-tile optimization. The piet-gpu rendering pipeline is organized as a series of stages, finally writing a per-tile command list (sometimes called "tape") for each 16x16 tile in the target. The last stage is "fine rasterization," which plays these commands for all pixels in the tile. Within a tile there is no control flow; all commands are evaluated for all pixels.

In the general case, clip is rendered as follows. A `BeginClip` operation pushes the current pixel on a *blend stack.* The children of the clip are rendered, compositing into the current pixel. Then, at a matching `EndClip`, the clip mask is rendered into an alpha mask, and the current pixel is rendered onto the top of the stack, using that alpha mask as an additional alpha multiplier, popping the stack.

Much of the time, we don't need the general case, however. Piet-gpu rendering works by 16x16 pixel tiles. Earlier stages in the pipeline computes what should happen within a tile, and the final stage (fine rasterization) performs a sequence of drawing operations for all pixels in the tile. This gives us the opportunity to do some optimization.

In particular, zooming into a single tile, a clip path may be in one of 3 states: zero coverage, partial coverage, or full coverage. Partial coverage only happens with the clip path intersects the tile. Full coverage is for tiles entirely within the clip path, and zero coverage is for tiles outside the clip path.

![Diagram showing zero, partial, and full path coverage by tile](/assets/clip_tiles.svg)

The mask computation and compositing only needs to happen for the partial coverage tiles (shown as gray in the above figure). The others can be rendered much more efficiently. Zero coverage tiles suppress the rendering of child nodes, basically from the BeginClip to the corresponding EndClip. And full coverage tiles are basically a no-op; the mask need not be rendered, and child nodes are rendered as if there were no clip in effect.

## GPU computation of clip bounding boxes

A previous iteration had accounting of bounding boxes done by CPU. A major goal of piet-gpu is to move as much computation as possible to the GPU.

The fundamental task is assigning to each draw object a bounding box rectangle that incorporates the intersection of all enclosing clips. A linearized representation of the scene will have a `BeginClip` element and an `EndClip` element. The `BeginClip` will also reference a path, and that path has an associated bounding box.

Here's what that calculation looks like, as a sequential algorithm:

```python
stack = [viewport_bbox]
for element in scene:
    if element is BeginClip(path):
        stack.push(intersect(stack.last(), path.bbox()))
        element.effective_bbox = stack.last()
    elif element is EndClip:
        element.effective_bbox = stack.last()
        stack.pop()
    else:
        element.effective_bbox = intersect(stack.last(), element.bbox())
```

As a sequential algorithm, this is very straightforward, almost trivial. There's a stack of bounding boxes, and the size of that stack is bounded by the maximum nesting depth. The cost to processing each element is O(1) as well. The only problem is, you really, really don't want to run a sequential algorithm on a GPU.

That raises a question: is there a way to make a parallel version of this algorithm, one that runs efficiently on actual GPU hardware? That question has been a major obsession of mine for at least a year. And I am pleased to say, the answer is yes. I've done it.

The core of my solution is what I call the stack monoid, which is a variant on the well-studied parentheses matching problem. I blogged about an earlier version in [stack monoid revisited]. Since then, I've made an improved version, with almost 5x peak performance and better portability as well. I'm not going to go into the details in this blog post, rather I will just say the solution is available by magic, and focus on the application to the 2D rendering problem.

![Diagram showing parent relationships in a tree](/assets/stack_monoid_parent_tree.svg)

Basically, we use the result of parentheses matching for two things. First, each EndClip is able to access the same path and bounding box data as the corresponding BeginClip. In particular, that lets us do the per-tile optimization in coarse rasterization efficiently, as that shader doesn't need to maintain significant state. Second, 

### Stream compaction

This section is a detail that can be skipped, but may be of interest to people writing fancier GPU algorithms.

The [original piet-gpu design] used an "array of structures" approach to scene description, in particular a single array with fixed size elements, each of which was a tagged union of various drawing element types, including path segments. Processing this array basically requires a large switch statement to deal with the variants in the union. I had contemplated doing a stack monoid over this array, but was very worried about the performance cost of computing the stack monoid for every element in this array. I now have a *very* fast stack monoid implementation, but even so have reworked the architecture so this cannot be a problem.

The new architecture (described in some detail in the [new element processing pipeline] issue) is more of an "array of structures" approach, which is extremely popular in the graphics and game world due to its performance advantage. Every major datatype gets its own stream. Further, as much of the logic for that type gets moved into its own shader dispatch, which works in bulk on only that type of object, with no big switch statement. To stitch these together, we use a bunch of indices into these streams, which are computed using prefix sum of the counts.

Specifically, the draw object stage does a stream compaction and writes a *clip stream,* which is just the BeginClip and EndClip objects. A clip index is an index into this stream. At the same time, it assigns a clip index to each draw object. A sequence of draw objects enclosed by the same clip all have the same clip index.

The clip stage then does parenthesis matching and bbox intersection of the clips in the clip stream. When it's done, it assigns a bounding box to each object in the clip stream (intersecting the bounding boxes that have already been computed for the clip paths), and also sets the path in EndClip to refer to the same path as the corresponding BeginClip.

Thus, the work of the clip stage is proportional to the number of clips in the scene, not to the total number of objects. It would take an enormous number of clips for this work to show up to any significant amount in profiles. We used similar stream compaction techniques to move to a more compact [path encoding], and I plan to apply it to other parts of the pipeline as well.

## Clipping and scrolling

One application of clipping is to define the viewport of a scrolled view in a UI. This can be represented in piet-gpu as a clip node with a transform node as a direct child, then the scrolled contents as children of the transform node. The translation associated with the transform node controls the scroll position (this architecture could do scaling as well).

A design goal of piet-gpu is the most of these contents can be encoded *once* and retained, so a new scene with a different scroll position could reassembled with very little work CPU-side. On the GPU side, there would be a fairly small amount of work to compute clip bounding boxes, which would be able to cull objects quite early in the pipeline, 

Obviously this approach will work for moderate scrolling, where it is practical to have all the resources resident on the GPU. For huge scrolled windows, some virtualization is needed, with resources swapped in and out as they scroll into and out of view. Even so, this is an appealing direction to explore, as smooth scrolling is still a challenge for UI toolkits.

## Related work

This work is perhaps most similar to [Massively Parallel Vector Graphics]. We both represent the scene as a flattened tree, and allow arbitrary nesting depth. However, their tree algorithm is much more simplistic: for a nesting depth of n, they do n scans, each addressing one level of nesting. This work uses a new algorithm that allows arbitrary nesting depth with no slowdown. (GPU tree algorithms with a work factor proportional to the depth of the tree are not unusual; for example)

In a more traditional GPU renderer, the general way to do blends is to allocate a temporary texture, render into that, and then composite by drawing a quad into the render target, sampling from the intermediate texture. GPUs are very highly optimized for this sort of work, with hardware support for texture sampling and "raster ops" for compositing, but even so it requires traffic to main memory. I believe it's faster not to have to do the work at all.

It's fairly dated by now, but Adam Langley's blog post on [clipping in Chromium] makes for interesting reading. The main problem being discussed is "conflation artifacts," which are not fully addressed by the alpha-channel compositing approach to clipping, but even so it remains the standard technique, largely because it's more or less mandated by the W3C [compositing and blending] spec and the [HTML canvas drawing model].

Do you maintain a 2D renderer with interesting clip support? Are there good writeups somewhere I've missed? Let me know, and I'll be happy to add links.

## Next steps

There are some things missing from this blog post, notably performance numbers. Right now, my focus in piet-gpu is to get the architecture right. It feels like that's converging, many of the hard problems are being solved.

Now that the infrastructure for clipping is in place, blending should be relatively straightforward. Most of what needs to happen is additional blending logic in the fine rasterization, plus of course plumbing the relevant metadata through the pipeline. That, plus radial and sweep gradients, are the two major pieces need to support [COLRv1 emoji], the next big milestone.

[stack monoid revisited]: https://raphlinus.github.io/gpu/2021/05/13/stack-monoid-revisited.html

[vision]: https://github.com/linebender/piet-gpu/blob/master/doc/vision.md

[Massively Parallel Vector Graphics]: https://w3.impa.br/~diego/projects/GanEtAl14/
[clipping in Chromium]: http://neugierig.org/software/chromium/notes/2010/07/clipping.html
[compositing and blending]: https://www.w3.org/TR/compositing-1/
[HTML canvas drawing model]: https://html.spec.whatwg.org/multipage/canvas.html#drawing-model
[original piet-gpu design]: https://raphlinus.github.io/rust/graphics/gpu/2020/06/12/sort-middle.html
[new element processing pipeline]: https://github.com/linebender/piet-gpu/issues/119
[COLRv1 emoji]: https://github.com/googlefonts/colr-gradients-spec
[path encoding]: https://github.com/linebender/piet-gpu/blob/master/doc/pathseg.md
