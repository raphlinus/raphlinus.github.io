---
layout: post
title: Swapchains and frame pacing
date:   2021-10-20 11:25:42 -0700
categories: [ui, graphics, gpu]
---
This is something of a followup to [the compositor is evil], but more of a guide on how to optimize latency, and less of a rant. I'm starting to collect performance measurements of [piet-gpu] running on Android, and ran into unexpectedly poor frame rates, which re-opened this topic. Simply increasing the number of images in the swapchain fixed the throughput problem, but I was also worried about the impact on latency. Exploring that led me down a deep rabbit-hole.

My primary interest is improving the performance of (2D) UI graphics, but the modern graphics stack is strongly driven by the needs of 3D graphics, especially games. Graphics performance is a tradeoff between throughput, latency, and power, with smoothness sometimes also a consideration (for movie playback it makes sense to use a deeper buffer to minimize the chance of a dropped frame). Different applications tend to use different strategies for swapchain presentation, but in this post I will propose something of a unified strategy, suitable for both UI and 3D intensive applications.

## Front and back buffers

Let's wind the clock back about 30 years, to a video game console or PC that is generating 3D content for fullscreen display.

At that time, it was traditional for the UI to be drawn using a *single* display buffer. The application would draw its graphics by writing into that buffer, and concurrently the video would be *scanned out* by reading from that memory. Sometimes that led to flickering artifacts, but generally not too bad, as an application would generally write the final pixels for a region fairly quickly, rather than leaving it in an intermediate partly-drawn state for long enough for the scan to catch it.

But such an approach would be completely unworkable for 3D. There, you start with a completely blank buffer, then draw in triangles one at a time, using Z testing so if the newly drawn triangle is farther away than the one in the buffer, the pixel is discarded. After all triangles are drawn, the scene is complete. But if you were to scan out during this process, you'd see a terrible flickery half-drawn mess.

The standard solution is *double buffering.* One buffer is designated the "front buffer," and video scanout occurs from that. The other buffer is the "back buffer," and it is free for the application to scribble on. Once painting is complete, the app *swaps* the two buffers, the front buffer becoming back and vice versa, so scanout occurs from.

There are two choices regarding syncronization with the frame rate. With "vsync off," scanout immediately switches to the newly rendered frame, and the formerly front buffer immediately becomes available for rendering the next one. This is ideal for both throughput and latency, but the downside is a "tearing" artifact, as the frame presented to the user contains horizontal bands of different rendered frames.

With "vsync on," the actual swap is deferred until the end of scanout of the current frame, so scanout of the next frame begins at the top. At the same time, the formerly-front, newly-back buffer is not available for writing until that vsync moment, so any operations that would write onto the frame must block for the frame to become available. In a well-written app, the time between the frame being *presented* and the next vsync where the other buffer is *available* can be used for other things, for example processing inputs or running game systems such as physics simulation, as long as those things don't write the buffer. In OpenGL, this is arranged by a [synchronization point]. The swap itself (for example, glXSwapBuffers) returns immediately, but the next call to OpenGL that requires write access blocks.

A typical [game loop] looks like this then:

```python
while true:
    process inputs
    do simulation etc
    glClear()
    draw the game
    glXSwapBuffers()
```

In systems of this era, generally the 3D acceleration functions as a co-processor to the main CPU, basically synchronously executing commands (or with a small queue), so the latency between the CPU initiating a drawing command and it completing with a write to buffer memory is short. Good models include the [Playstation 2] and [Nintendo 64]. (Note though that even in these early days, there was signficant architectural diversity, and  the [Dreamcast] employed tile-based deferred rendering, which lives on today in many mobile GPUs).

Overall, it looks like this:

** TODO need picture **

### Performance analysis

Under reasonable assumptions, the performance of this simple system is good. Generally you want the "draw the game" part of the game loop to take just shy of one frame time (16ms), otherwise you're leaving graphical richness on the table. If that's the case, then performance, measured as both throughput and latency, is as good as possible.

But consider this entirely plausible case: let's say you run the same game on the next generation of hardware which is exactly twice as fast. With vsync off, latency improves as expected, as a frame takes half as long to render (8ms), and scanout can begin as soon as rendering is complete. But with vsync on, latency is no better than the previous generation console. Looking at the timeline, the `glClear()` call now spends 8ms blocking, and there's no sensitivity to inputs during that time.

** need picture **

Surprisingly, adding a `sleep(8ms)` call right before "process inputs" would improve the situation considerably. The game and GPU are doing the same amount of work, but the inputs are 8ms fresher than if the time is spent blocking in `glClear()`. But how do we know that 8ms is right? If it's too much, say 10ms, then the frame rate would suddenly drop to 30fps. And if it's too little, then we're leaving latency on the table.

** need picture **

## Triple buffering

One approach to this problem is [triple buffering]. Instead of a single back buffer, there are two; one ready for display, and the other being drawn. The "swap" call now swaps between these two without blocking. At vsync, the display engine grabs the buffer that's ready and swaps it with the front buffer.

In the limit as GPU performance approaches infinitely fast, this approach gives "as good as possible" latency without tearing. However, there are at least two problems.

The most obvious is the wasted power consumption for rendering frames which will then be entirely discarded. Even ignoring the carbon footprint, keeping the GPU busy like this can result in thermal throttling, which reduces performance in other ways.

A more subtle issue is that smoothness isn't as good as it might be; the delay between the start of the game loop and the scanout jitters by a uniform distribution of zero to one GPU rendering interval. When the GPU is very fast, that might not be noticeable, but when the FPS is just under the refresh rate of the monitor, triple buffering can add a significant amount of jitter (compared to double-buffered vsync on) with little improvement in latency.

While triple buffering is at least moderately popular for games, I know of no UI toolkit that employs it, and the power concerns basically take it off the table for mobile. Thus, I am *not* recommending it, and will suggest other techniques for attaining the same goal.

## Asynchronous rendering

(Maybe do some explanation here about how in Vk you queue work to be done asynchronously? Pipelining can improve throughput but can hurt performance. But that might be too general and orthogonal to the swapchain pacing concerns. The Call of Duty presentation goes into all this stuff in *much* more detail)

## Blocking synchronization in Vulkan

While the GPU of the Playstation 2 era is more or less synchronous, a modern GPU is very much *asynchronous,* and the resulting pipelining can improve performance a lot. Modern graphics APIs such as Vulkan give the programmer explicit control over the this asynchrony, with an emphasis on explicit [synchronization][Understanding Vulkan Synchronization] primitives. Of course, that also extends to interactions with the swapchain.

The classic game loop in Vulkan looks like this:

```python
while true:
    process inputs; run game systems
    vkAcquireNextImageKHR(swapchain, acquire_semaphore)
    render game by building command buffers
    vkQueueSubmit(cmdbuf, acquire_semaphore, present_semaphore)
    vkQueuePresentKHR(present_semaphore)
```

Let's break this down, as there's a fair amount going on.

The acquireNextImage call requests a writable image from the swapchain. It *may* block for some time, but generally returns somewhat in advance of the image actually becoming available; if you were to immediately write to the image, that would be invalid. That is the purpose of the acquire semaphore - acquireNextImage promises to signal the semaphore when the frame is actually available.

That's why the queue submit call takes an acquire semaphore. The call *schedules* the work onto the queue, but it may or may not begin running immediately. One reason it may be delayed is that the semaphore hasn't been signaled yet, indicating that the swapchain image is not yet available for writing. (Other reasons it might be delayed include other GPU work in the queue that hasn't completed yet)

The presentation call is similar. It's a request to present the rendered image, but at the time the application makes the call, rendering may not have completed (indeed, it might not even have started).

This strategy basically tries to keep queues as full as possible. That's great for throughput, but not necessarily ideal for latency. In addition, the actual behavior might be pretty different from one GPU to another. In my experimentation, I've seen both acquireNextImage and queuePresent block, depending on details of swapchain settings and workload, but it's possible.

## Enter the compositor

The above discussion assumes fullscreen mode, ie scanout directly from the swapchain to the display. Modern systems generally add a compositor to the mix, so that the output from the application can be composited with other windows (and notifications and so on), and scanout happens from the compositor's target surface (not visible to applications at all).

In theory, adding the compositor doesn't change things much, other than adding one frame time of latency (more or less unavoidably) and taking the "vsync off" option off the table. Because of those limitations, it's also common to offer a "fullscreen" option specialized for gaming that bypasses the compositor, often with special affordances for compositing notifications (the FLIP_DISCARD mode in DXGI is designed for this, see the note on "reverse composition" in this doc [recommending flip modes](https://devblogs.microsoft.com/directx/dxgi-flip-model/)).

The major complication is the presence of hardware overlays, which are a factor on both [Windows][direct flip] and [Android][HWC] (and possibly other systems). The idea of a hardware overlay is that *sometimes* the contents of a window can be scanned out by the video hardware directly from an app-owned swapchain, rather than that swapchain simply being used as the texture source by the compositor (most of the time resulting in bit-identical RGB pixels in the compositor's target buffer). The display adapter usually has a very small number of overlays available (3 or 4 is common), so it's generally only available for the frontmost window. The compositor makes the decision heuristically, and the app is generally not involved.

On Windows, from my understanding, a hardware overlay window has one frame less latency. That's generally good, but not necessarily good for smoothness, as transitioning between the overlay and fallback compositing can cause jank (and very likely audio synchronization issues for video playback). The details are not clear to me, and don't seem to be well documented. If timing is critical to your Windows application, I recommend spending a lot of quality time in PresentMon in conditions where direct flip may be enabled and disabled.

*[Some of this paragraph is based on speculation and needs to be fact-checked]* Android takes a different approach. A hardware overlay window is scheduled for scanout at the same time as if it were composited using OpenGL, in other words the choice is effectively invisible to the user; the main benefit is reduced power consumption. However, one consequence is that the compositor may need to hold on to the swapchain buffer longer than would be the case in a fullscreen or Windows/mac style compositor, as it might be read until the end of actual scanout. Thus, the minimum acceptable value for the swapchain size (minImageCount in Vulkan-speak) to sustain a smooth framerate is 3 for Android, while 2 has been fine on desktop.

However, if we use blocking-based synchronization and a swapchain size of 3, latency is quite bad. Basically, blocking-based synchronization tries to keep pipelines as full as possible, and the blocking calls cause input to grow stale while they block. To understand this better, it might be good to look at an actual systrace captured by [Android GPU Inspector] (from the current version of my code, actually):

![Example systrace from Android GPU inspector](/assets/agi_systrace.png)

Here, frame 408 begins in the "game loop" around 1700ms - the input has been processed, and the app calls acquireNextImage. That blocks for 7ms until the system dequeues the buffer (the "D" diamond in the third buffer trace). However, the buffer is not actually available, it's still in use for scanout of frame 405 to the display. Even so, ANI returns control back to the app at 1707ms, which then does about 2ms of CPU-side work to fill a command buffer, and calls vkQueueSubmit at about 1709ms, followed by vkQueuePresentKHR a few hundred µs later ("Q" for queue). The command buffer submission is controlled by a semaphore shared with the ANI call; basically ANI can return any time it likes, and promises to signal the semaphore when the buffer is actually available for writing, ie when it's released from its use for scanout of frame 405. That happens at 1713ms (the "R" diamond), and triggers GPU work starting at 1713ms and lasting 5ms. Completion of that command buffer of GPU work signals the semaphore shared with the QP call, which lets SurfaceFlinger acquire the buffer at 1718ms (the "A" diamond). The next vsync for composition is at 1727ms, at which point SurfaceFlinger latches ("L") the buffer. The buffer is eligible for hardware composition, so is held until scanout of frame 408 begins, at 1737ms ("P" for present).

How might we have done better? The actual deadline for the GPU to finish its rendering work is the "L" at 1727ms. That work is about 2ms of CPU and 5ms of GPU. Thus, if we had processed input and started rendering at 1720ms, we would have just barely made the deadline. And latency (from input to start of scanout) would have been reduced from about 37ms to about 17ms.

Another way of looking at this is the three intervals of blocking, in each of which the input grows increasingly stale: the blocking call to ANI (7ms), the wait from command buffer submission to the GPU actually starting work (4ms), and the time between rendering being complete and the deadline for it to be accepted by the compositor (9ms). The first two can and should go to 0 (it's just waiting for a buffer to be available), but some safety margin should be retained in the third; if it gets too close to 0, the risk of dropping a frame goes up.

## Frame pacing

Basically, relying on blocking calls for scheduling rendering is the old way, and gives particularly bad results on Android. The new way is to implement some form of "frame pacing."

From first principles, you have a deadline for presenting a certain frame, and also an estimated probability distribution for how long the rendering will take. To optimize for input lag, that should take into account input processing as well, not just the drawing. The optimum time for starting the work is the deadline minus the (say) 99.99 percentile of that distribution (this would result in a jank event every minute or so).

Figuring out the deadline should be pretty straightforward; on a display with fixed refresh rate, it should just be multiples of the refresh interval. Things can get more complicated with variable refresh rate, but that's largely out of the scope of this post (ideally, a good implementation of variable refresh lets the app miss the frame deadline by a small amount with only minimal consequences, as opposed to janking an entire frame).

The challenge, of course, is accurately estimating that distribution. A major component to that is observing past behavior, but that's not all there is to it, either: input processing might be highly variable (especially depending on the inputs), and rendering time might be as well. Games *tend* to have fairly smoothly varying scene complexity, but that's a design choice, and may be much less true for UI-centric applications.

But let's put that aside and focus on analyzing past performance. While such a mechanism is not part of core Vulkan, there are extensions. In particular, on Android there is the [VK_GOOGLE_display_timing] extension. This has a call to get the display rate (for predicting future deadlines), and another to query the past presentation timings, in particular by exactly how much the presentation beat the deadline. Further, it extends queuePresent so that you can request presentation in the future, not necessarily the next vsync. That's especially important when rendering at a slower pace than the display's refresh (increasingly important as high-refresh displays become common).

If you're writing an Android game, it might be easier to just use the [frame pacing library] (also known as Swappy), which wraps these lower-level calls in a convenient interface. It makes assumptions (*fact-check*) most suited to games, but is also battle-tested and should work well.

There is work towards evolving this Android-specific extension into a Khronos standard, driven in large part by Wayland. See this [Phoronix article on present timing] for more details. There's also lots of detailed discussion in the [VK_EXT_present_timing proposal] thread itself.

It is also possible to get past presentation statistics on Windows, using [GetFrameStatistics], and games that have been optimized for latency use it to control the scheduling of rendering work. An excellent case study is [Controller to display latency in Call of Duty], which goes into great detail (and covers some of the same ground as this blog). I should note, the official documentation [recommends][UWP reduce latency] the use of a [latency waitable object] so that you can wait asynchronously rather than blocking on a Present call, but I haven't seen compelling evidence that this technique actually yields optimal results.

There is evidence that a lot of Windows games are suboptimal when it comes to frame pacing. One such indication is the the existence of SpecialK, an open-source tool that dynamically tweaks games (their [SwapChain science] page goes into some of the same issues as this post), in many cases showing dramatic improvement.

### Some control theory

Predicting the total rendering time is a really hard problem, for a variety of reasons. For one, it can depend on completely external factors, such as load from other processes running on the same system.

But a special reason it's hard is that rendering time may depend on when frames are scheduled, which in turn depends on the output of the predictor. A particularly likely manifestation of that feedback loop is clock frequency control on the GPU. That's usually its own controller, applying its own heuristics based on the workload. It's not possible in general for the app to even query the clock frequency, though of course there is device-specific access, to support profiling tools.

For example, an app might decide to downgrade FPS based on some lost frames (let's say due to transient workload, such as an assistant running some machine learning workloads on the GPU). The GPU driver will happily scale down its frequency, which is good for battery, but that also gives feedback to the controller that the GPU performance has degraded, so it should sustain the low frame rate. Restarting the game would restore the higher frame rate, but there's no way for the controller to know that, so it stays stuck at the low rate. I'm not sure if there's any good solution to these kinds of system-wide performance factors, but I think it's worth looking into.

## Conclusion

Frame pacing is a poorly understood and oft overlooked aspect of the total performance story, especially as it affects latency. Doing things the simplest way will generally optimize for throughput at the expense of latency.

On Android, the Swappy library provides a good solution. On other platforms, you need to dig deeper. While most of the attention has been on games, applying these kinds of frame pacing techniques should also significantly improve UI latency.

Thanks to Ian Elliott for explaining some of the arcane details of how Android manages swapchains.

[the compositor is evil]: https://raphlinus.github.io/ui/graphics/2020/09/13/compositor-is-evil.html
[piet-gpu]: https://github.com/linebender/piet-gpu
[synchronization point]: https://stackoverflow.com/questions/26162745/glxswapbuffers-blocks-until-vblank
[Playstation 2]: https://www.copetti.org/writings/consoles/playstation-2/
[Nintendo 64]: https://www.copetti.org/writings/consoles/nintendo-64/
[Dreamcast]: https://www.copetti.org/writings/consoles/dreamcast/
[game loop]: https://gameprogrammingpatterns.com/game-loop.html
[triple buffering]: https://www.anandtech.com/show/2794
[HWC]: https://source.android.com/devices/graphics/implement-hwc
[direct flip]: https://docs.microsoft.com/en-us/windows-hardware/drivers/display/direct-flip-of-video-memory
[SurfaceFlinger]: https://source.android.com/devices/graphics/surfaceflinger-windowmanager
[Android GPU Inspector]: https://gpuinspector.dev/
[VK_GOOGLE_display_timing]: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VK_GOOGLE_display_timing.html

[Phoronix article on present timing]: https://www.phoronix.com/scan.php?page=news_item&px=VK_EXT_present_timing
[VK_EXT_present_timing proposal]: https://github.com/KhronosGroup/Vulkan-Docs/pull/1364
[latency waitable object]: https://docs.microsoft.com/en-us/windows/win32/api/dxgi1_3/nf-dxgi1_3-idxgiswapchain2-getframelatencywaitableobject
[UWP reduce latency]: https://docs.microsoft.com/en-us/windows/uwp/gaming/reduce-latency-with-dxgi-1-3-swap-chains
[frame pacing library]: https://developer.android.com/games/sdk/frame-pacing
[GetFrameStatistics]: https://docs.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgiswapchain-getframestatistics
[QueryPerformanceCounter]: https://docs.microsoft.com/en-us/windows/win32/api/profileapi/nf-profileapi-queryperformancecounter
[SwapChain science]: https://wiki.special-k.info/en/SwapChain
[Understanding Vulkan Synchronization]: https://www.khronos.org/blog/understanding-vulkan-synchronization
[Controller to display latency in Call of Duty]: https://www.activision.com/cdn/research/Hogge_Akimitsu_Controller_to_display.pdf
