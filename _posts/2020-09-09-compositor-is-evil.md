---
layout: post
title:  "The compositor is evil"
date:   2020-09-09 16:23:42 -0700
categories: [ui, graphics]
---
First, I want to make it clear that I'm not accusing the compositor of true evil, which I define roughly as deliberately causing human suffering. There's unfortunately too much of that in the world. I mean it in the more metaphorical sense that it causes serious problems and causes other parts of the system to be more complex to work around its limitations.

I will also outline a better design that might be implemented in the future, as well as some possibilities for how applications can deal with the current situation. But first, to understand better how we got into this mess, we'll look a bit into the past.

## 8-bit systems: video processing engines

Early home computers and video games had 8-bit CPUs and only a few kilobytes of RAM. Some computers, like the Apple II, allocated some of the RAM as a frame buffer and used the CPU to fill it, but this approach is quite limiting. So designers of most other systems of the time got creative - in particular, they augmented the CPU with a graphics processing engine. This engine did a fair amount of processing during *scanout,* in other words on the fly as the system generated a video signal.

The details of these early graphics processors varied, but most were a combination of four basic operations:

* Indirect lookup of tile images (also useful for text).
* Expansion of low bit depth pixels into more colors through a palette.
* Control over the offset in video memory to read (for scrolling effects).
* Overlay of small graphic elements (sprites).

The combination of even a modest CPU and one of these video processing engines led to creative explosion of games, with a unique aesthetic formed from the primitives provided by the video chip: colorful scrolling backgrounds formed from tiles, with the player avatar and other characters overlaid, also moving dynamically. One of the most popular (and best documented) such chip is the [C64 VIC-II], and of course the [NES PPU] is another classic example. People still create software for these beloved systems, learning from tutorials such as [VIC-II for beginners].

While the constraints of the hardware served as artistic inspiration for a diverse and impressive corpus of games, they also limited the kind of visual expression that was possible. For example, any attempt at 3D was primitive at best (though there were driving games that did impressive attempts, using the scrolling and color palette mechanisms to simulate a moving roadway).

The most ambitious games and demos agressively poked at the hardware, "racing the beam" to dynamically manipulate the registers in the video chip, unlocking more colors and other graphical effects that no doubt were not even anticipated by the original designers of the hardware.

Also worth mentioning as an extreme example is the Atari 2600, which had all of 20 bits (that's right, two and a half bytes) devoted to what might be considered a frame buffer. Basically all graphical effects needed to be implemented by racing the beam. Fortunately, the programmer could make use of the deterministic cycle counts for instructions on the 6502 microprocessor, so that writes to the video registers would happen with timing considerably finer than one scan line.

An important aspect of all these systems, even those as underpowered as the Atari 2600, is that latency was *very* low. A well coded game might process the inputs during the vertical blanking interval, updating scroll and sprite coordinates (just a few register pokes), so they apply to the next frame being scanned out - latency under 16ms. Similar math applies to the latency of typing, which is why the Apple IIe [scores so well](https://danluu.com/input-lag/) on latency tests compared to modern computers.

## 16 and 32 bit home computers: CPU + framebuffer

Through the late 80s, while arcade games continued to evolve their graphics hardware, the IBM PC became ascendant as a home computer. From its roots as a "business" computer, its CPU performance and RAM grew dramatically, but video output was generally a framebuffer with almost none of the capabilities listed above. Even so, display resolutions and bit depths improved (VGA was 640x480 with 16 colors per pixel). A reasonably powerful CPU, especially in the hands of an expert coder, can produce rather impressive graphics. An example is Microsoft Flight Simulator, which had 3D graphics, fully drawn in software.

Another landmark release was the original Doom, released in 1993, which also was entirely software-rendered graphics, complete with lighting and textures.

The rendering of UI and 2D graphics continued to evolve during this time as well, with proportional spaced fonts becoming the standard, and [antialiased font rendering](http://www.truetype-typography.com/ttalias.htm) slowly becoming standard during the 90s. (Likely the [Acorn Archimedes](https://telcontar.net/Misc/GUI/RISCOS/#text) was the first to ship with antialiased text rendering, around 1992)

## Multiple windows and process separation

A trend at the same time was the ability to run multiple applications, each with their own window. While the idea had been around for a while, it burst on to the scene with the Macintosh, and the rest of the world caught up shortly after.

Early implementations did *not* have "preemptive multitasking," or what we would call process separation. Applications, even when running in windowed mode, wrote directly into the framebuffer. The platform basically provided a library for applications to keep track of visible regions, so they wouldn't paint in regions where the window was occluded. Related, when an occluding window went away, the system would notify the application it needed to repaint (WM_PAINT on Windows), and, because computers were slow and this took a while, the "damage region" was visible for a bit (this is explained and animated in Jasper St. Johns' [X Window System Basics](https://magcius.github.io/xplain/article/x-basics.html) article).

In this early version of windowing GUI, latency wasn't seriously affected. Some things were slower because resolution and bit depth was going up, and of course text needed to be rendered into bitmaps (more computationally expensive with antialiasing), but a well-written application could still be quite responsive.

However, running without process separation was a serious problem, if for no other reason than the fact that a misbehaving application could corrupt the entire system. System crashes were extremely common in the days of pre-X MacOS, and pre-NT Windows. Certainly in the Unix tradition applications would run in their own process, with some kind of client-server protocol to combine the presentation of the multiple applications together. The X System (aka X11) came to dominate in Unix, but before that there were many other proposals, notably [Blit](https://en.wikipedia.org/wiki/Blit_(computer_terminal)) and [NeWS](https://en.wikipedia.org/wiki/Blit_(computer_terminal)). Also common to the Unix tradition, these would commonly run across a network.

## Apple's Aqua compositor

When [OS X](https://en.wikipedia.org/wiki/Mac_OS_X_10.0) (now macOS) first shipped in 2001, it was visually striking in a number of ways. Notably for this discussion, the contents of windows were blended with full alpha transparency and soft shadows. At the heart of this was a *compositor.* Applications did not draw directly to the screen, but to off-screen buffers which were then composited using a special process, [Quartz Compositor].

In the first version, all the drawing and compositing was done in software. Since machines at the time weren't particularly powerful, performance was bad. According to a [retrospective review], "its initial incarnation, Aqua was unbearably slow and a huge resource hog."

Even so, things improved. By 10.2 (Jaguar) in August 2002, Quartz Extreme did the compositing in the GPU, finally making performance comparable to pre-compositor designs.

While there have been changes (discussed in some detail below), Quartz Compositor is fundamentally the modern compositor design used today. Microsoft Vista adopted a similar design with [DWM], first in Vista, and made non-optional by Windows 8.

## Doing more in the compositor

Using the GPU just bitblt window contents is using a tiny fraction of its capabilities. In the compositing process, it could be fading in and out alpha transparency, sliding subwindows around, and applying other effects without costing much at all in performance (the GPU is already reading all the pixels from the offscreen buffers and writing to the display surface). The only trick is exposing these capabilities to the application.

On the Apple side, this was driven by iOS and its heavy reliance on [Core Animation]. The idea is that "layers" can be drawn using relatively slow software rendering, then scrolling and many other effects can be done smoothly at 60fps by compositing these layers in the GPU.

Core Animation was also made available in macOS 10.5 (Leopard). The corresponding Windows version was [DirectComposition], introduced in Windows 8 and a core feature of the [Metro] design language (which was not very popular at the time).

## Hardware overlays

Since the compositor adds latency and is hungry for bandwidth, over time there's been an increasing trend to transfer some of its functions to hardware. The earliest example is special-purpose circuitry in scanout to superimpose a mouse cursor, sometimes known as "silken mouse." On top of that, video playback often directed to an overlay window instead of going through the compositor. In that case, the performance benefits are twofold compared with a desktop app or game being the source of the pixels: specialized scaling and color conversion hardware can be *dramatically* faster and lower power than doing the same thing in software, even using GPU compute capabilities.

Mobile phones were the next major advance in hardware overlays. They tend to use a small number of windows; the [Implementing Hardware Composer HAL] document lists four as the minimum requirement (status bar, system bar, app, and wallpaper). Having been on the Android UI toolkit team, I'd have liked to add keyboard to that list, but four is still a reasonable number for probably 99% of the time the screen is on. When that number is exceeded, the overflow is handled by GLES. For anyone curious about the details, go read that document, as it explains the concerns pretty clearly.

On the desktop side, Windows 8.1 brought [Multiplane overlay support], which seems to be motivated primarily by the needs of video games, particularly running the game at lower resolution than the monitor and scaling, while allowing UI elements such as notifications to run at the full resolution. Doing the scaling in hardware reduces consumption of scarce GPU bandwidth. Browers also [use](https://chromium.googlesource.com/chromium/src/+/b6d0e4eb43481c191e0189025820bea8f87c7049/ui/gl/direct_composition_surface_win.cc) overlays for other purposes (I think mostly video), but overall their use is pretty arcane.

### DirectFlip

The main limitation of multiplane overlays is that they only cover some specialized use cases, and applications have to opt in explicitly, using advanced APIs. The observation motivating [DirectFlip] is that in many cases, there's an application window at the front of the composition stack, happily presenting to its swapchain, that could be promoted to a hardware overlay rather than going through the compositor. And on some hardware (I believe Kaby Lake and later integrated Intel graphics), DirectFlip is turned on.

There are some problems with DirectFlip as well, of course. It's enabled using a heuristic, and I don't think there's an easy way for an app to tell that it's presenting through DirectFlip rather than the compositor, much less request that. And, though I haven't done experiements to confirm, I strongly expect that there is jankiness as the latency suddenly changes when a window is promoted to DirectFlip, or back again. Popping up a context menu is likely to disrupt a smoothly running animation.

One consequence of DirectFlip becoming more common is that it presents a difficult tradeoff for a graphically complex app such as a browser: either it can leverage the compositor for tasks such as scrolling and cursor blinking, which ordinarily would reduce power consumption, or it can do all compositing itself and hope the window is promoted to DirectFlip, in which case it's likely (depending on details of workload, of course) that the total power consumption will go down.

## Smooth resize

Any discussion of the evils of the compositor would be incomplete without a section on smooth window resize, a topic I've [covered before][smooth resize test].

In ordinary gameplay, the graphics pipeline, including swapchain presentation and composition, can be seen as a linear pipeline. However, when the user is resizing the window (usually using the mouse), the pipeline branches. One path is the app, which gets a notification of the size change from the platform, but is generally rendering and presenting frames to it swapchain. The other path is the window manager, which is tasked with rendering the "chrome" around the window, drop shadows, and so on. These two paths re-converge at the compositor.

In order to avoid visual artifacts, these two paths must synchronize, so that the window frame and the window contents are both rendered based on the same window size. Also, in order to avoid additional jankiness, that synchronization must not add significant additional delay. Both these things can and frequently do go wrong.

It used to be that Windows engineers spend considerable effort getting this right. The DX11 present modes ([DXGI_SWAP_EFFECT_SEQUENTIAL](https://docs.microsoft.com/en-us/windows/win32/api/dxgi/ne-dxgi-dxgi_swap_effect), which in my testing behaves the same as HwndRenderTarget), which are also used by most Direct2D applications, do a copy of the swapchain to the window's "redirection surface," which is pretty horrible for performance, but is an opportunity to synchronize with the window size chain event. And in my testing, using these present modes, plus the recommended [ResizeBuffers](https://docs.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgiswapchain-resizebuffers) method on the swapchain, works pretty well. I know I'm leaving performance on the table though; upgrading to the [DX12 present modes] is recommended, as it avoids that copy, unlocks the use of [latency waitable objects], and, I believe, is a prerequisite to DirectFlip.

But apparently the DX12 engineers forgot to add synchronization logic with window resize, so artifacting is pretty bad. [Windows Terminal uses the new modes](https://github.com/microsoft/terminal/issues/6436), and, sure enough, artifacting is pretty bad.

I believe they could fix this if they wanted to, but very likely it would add extra burden on app developers and even more complexity.

I'm focusing mostly on Windows here, but macOS has its own issues; those are largely covered in my previous post.

## What is to be done?

We can just put up with bad compositor performance, accepting the fact that windows just aren't meant to resize smoothly (especially on Windows), latency is much worse than in the Apple 2 days, and batteries won't last as long as they could.

But a better design is possible, with some hard engineering work, and I'll outline what that might look like here. It's in two stages, the first focused on performance, the second on power.

First, the compositor could be run to [race the beam], using techniques similar to what is now done in the high performance emulation community. Essentially, this would resolve the latency issues.

In fact, with a beam racing design, latency improvement could be even greater than one frame, *without* reintroducing tearing. It could actually get close to Apple 2 standards, and here's how. When pressing a key in, say, a text editor, the app would prepare and render a *minimal damage region,* and send that present request to the compositor. It is then treated as an atomic transaction. If the request arrives before the beam reaches the *top* of the damage region, it is scheduled for the current frame. Otherwise, the entire request is atomically deferred to the next frame. With this arrangement, updated pixels can begin flowing out of hardware scanout almost immediately after the keypress arrives. And unlike the days of Apple 2, it would do so without tearing.

Keep in mind that the existing compositor design is much more forgiving with respect to missing deadlines. Generally, the compositor *should* produce a new frame (when apps are updating) every 16ms, but if it misses the deadline, it's just jank, which people are used to, rather than visual artifacts.

The major engineering challenge of a beam racing design is ensuring that the compositing work reliably completes before scanout begins on the image data. As the experience of realtime audio illuminates, it's hard enough scheduling CPU tasks to reliably complete before timing deadlines, and it seems like GPUs are even harder to schedule with timing guarantees. The compositor has to be scheduled with higher priority than other workload, and has to be able to preempt it. As with audio, there are also interactions with power management, so it needs to be scheduled earlier when the GPU is clocked down, and the downclocking step needs to be deferred until after the chunk of work is done. All this requires deep systems-level engineering work. I'm definitely not saying it's easy, but it should nonetheless be possible. And I believe that the benefits of doing this work will apply in many other domains, not least of which is VR, so perhaps there is some hope it might happen someday.

### Hardware tiling

A beam racing compositor design has appealing performance characteristics, but is more than a bit scary when it comes to power consumption. In addition, it's not clear how it would interact with hardware overlays.

I'm not a hardware designer, but it seems to me there is a solution to both issues. Instead of having hardware overlays be based on large rectangular regions, updated only at vsync boundaries, make them work at finer granularity, let's say for concreteness 16x16 pixel tiles. The metadata for these tiles would be a couple orders of magnitude less than the pixel data, so they can reasonably be computed even on CPU, though I see no reason not to use GPU compute for the task.

Basically, when there are dynamic changes such as new window or resize, the compositor task would recompute a classification of tiles into "simple" and "complex". A simple tile is one that can be computed in hardware. Let's say for concreteness that it consists of a rectangular subregion from one texture buffer superimposed over a background from another texture buffer; this arrangement would ensure that the vast majority of tiles in simple overlapping window configurations would be simple. I'm ignoring shadows for now, as they could either be pre-rendered (shadows are likely to fall on non-animating content), or there could be more hardware layers.

For a simple tile, the compositor just uploads the metadata to the hardware. For a complex tile, the compositor schedules the tile to be rendered by the beam racing renderer, then uploads metadata just pointing to the target render texture. The only real difference between simple and complex tiles is the power consumption and contention for GPU global memory bandwidth.

I think this design can work effectively without much changing the compositor API, but if it really works as tile under the hood, that opens up an intriguing possibility: the interface to applications might be redefined at a lower level to work with tiles. A sophisticated application such as a browser might be able to express animations and nested scrolling behavior at a more fine grained level using these tiles than traditional CALayer style API calls. But that's speculative.

## Related Linux work

The Linux community is not standing still, and there is a lot of interesting work in improving compositor performance. Following is a bit of a raw dump of resources. [TODO: more careful editing?]

* https://www.collabora.com/about-us/blog/2015/02/12/weston-repaint-scheduling/ - Good blog post with diagrams explaining how Weston reduces compositor latency by delaying compositing a configurable amount after the last vblank, and how game style rendering only gets the full latency improvement if it renders using the presentation time feedback rather than the normal frame callback.

* https://github.com/swaywm/sway/pull/4588 - PR introducing the same model in Sway, except with a configurable per-app property to adjust when the frame callbacks are to trade off between available time in the deadline and latency. Seems to default to off.
* https://github.com/swaywm/sway/issues/4734 - Unimplemented issue for automatically setting these delays based on timing or frame drops

* https://cgit.freedesktop.org/wayland/wayland-protocols/tree/stable/presentation-time/presentation-time.xml - Wayland presentation time feedback protocol with comments. Includes flags for if the presentation time is an estimate or a hardware timestamp from the display, as well as if presentation was done zero-copy.

* https://github.com/swaywm/sway/issues/5076 - Sway has VRR support but on some monitors but not others instantly changing frame rate from minimum to maximum causes brightness flicker, for example because of strobed backlights not being able to predict the display time. The linked issue is for fixing that. A good comment mentions playing a 60fps video on a 144hz monitor and what to do when you move the mouse, if you go to 144hz then the 60fps video judders because 144%60!=0, but there's no protocol for knowing that something is animating at a given frame rate, so unclear what to do.

* https://lists.freedesktop.org/archives/dri-devel/2020-March/259111.html - mailing list thread about similar VRR issues and how and where to limit slew

* https://lwn.net/Articles/814587/ - Interesting article about a plan for moving towards explicit synchronization primitives in Linux Vulkan-style rather than GL-style implicit synchronization.

* https://github.com/emersion/libliftoff - library for hardware overlay promotion logic

* https://github.com/emersion/glider - experimental compositor using wlroots and libliftoff to do hardware overlays

* https://github.com/swaywm/wlroots/pull/2092 - PR Implementing ability for Wayland clients to ask the compositor to crop and scale a buffer, mentions eventually wanting this to be done in scanout.

* https://github.com/Plagman/gamescope - an unusual compositor design that might be using hardware overlays for compositing VR games, and may already have support for direct scan-out of fullscreen windows.

The Linux community has perhaps more leeway than the proprietary platforms to explore different approaches to these problems, so it's possible good answers will come from those directions. Unfortunately, a full solution seems to need hardware as well, which depends on mainstream suppliers and support.

## Workarounds in applications

In the meantime, applications need to make very difficult tradeoffs. Using the full capabilities of the compositor API can make scrolling, transitions, and other effects happen more smoothly and with less GPU power consumption than taking on the rendering tasks themselves. About a year ago, [Mozilla made changes to leverage the compositor more][Mozilla using Core Animation] on macOS, and was able to show fairly dramatic power usage improvements.

At the same time, exposing compositor capability in a cross-platform way seems harder than exposing GPU rendering, and as WebGPU lands that will be even more true. And, as mentioned, it may get in the way of the benefits of DirectFlip.

Making the right tradeoff here is a complex balance also depending on the type of application and overall complexity. If the compositor weren't evil, it wouldn't force us to make these kinds of tradeoffs. Maybe someday.

## Conclusion

Thanks to Patrick Walton for a series of provocative conversations around these topics. Also thanks to Tristan Hume for doing a bunch of the legwork tracking down references, particularly recent Linux work. Check out Tristan's [latency measuring project] as well.

[piet]: https://github.com/linebender/piet
[druid]: https://github.com/linebender/druid
[the xi Zulip]: https://xi.zulipchat.com/
[Desktop compositing latency is real and it annoys me]: http://www.lofibucket.com/articles/dwm_latency.html

HN discussion of that : <https://news.ycombinator.com/item?id=15747650>
[Mozilla using Core Animation]: https://mozillagfx.wordpress.com/2019/10/22/dramatically-reduced-power-usage-in-firefox-70-on-macos-with-core-animation/

[NES PPU]: https://en.wikipedia.org/wiki/Picture_Processing_Unit
[C64 VIC-II]: https://en.wikipedia.org/wiki/MOS_Technology_VIC-II
[VIC-II for beginners]: https://dustlayer.com/vic-ii/2013/4/22/when-visibility-matters

[Carmack's front buffer experiment]: https://twitter.com/id_aa_carmack/status/370368432924942337?lang=en

[Mac OS X 10.0]: https://en.wikipedia.org/wiki/Mac_OS_X_10.0
[retrospective review]: https://arstechnica.com/gadgets/2011/05/mac-os-x-revisited/2/
[Quartz Compositor]: https://en.wikipedia.org/wiki/Quartz_Compositor

[Desktop Window Manager]: https://en.wikipedia.org/wiki/Desktop_Window_Manager
https://en.wikipedia.org/wiki/Hardware_overlay

https://en.wikipedia.org/wiki/Compositing_window_manager
credit to Amiga

[DWM]: https://docs.microsoft.com/en-us/windows/win32/dwm/dwm-overview
[Core Animation]: https://developer.apple.com/documentation/quartzcore
[DirectComposition]: https://docs.microsoft.com/en-us/windows/win32/directcomp/directcomposition-portal
[Metro]: https://en.wikipedia.org/wiki/Metro_(design_language)

[Implementing Hardware Composer HAL]: https://source.android.com/devices/graphics/implement-hwc
[Multiplane overlay support]: https://docs.microsoft.com/en-us/windows-hardware/drivers/display/multiplane-overlay-support
[DirectFlip]: https://docs.microsoft.com/en-us/windows-hardware/test/hlk/testref/8d80da43-4da5-45e9-a629-b3defd3f52ee

[latency measuring project]: https://thume.ca/2020/05/20/making-a-latency-tester/
[smooth resize test]: https://raphlinus.github.io/rust/gui/2019/06/21/smooth-resize-test.html
[DX12 present modes]: https://docs.microsoft.com/en-us/windows/win32/direct3ddxgi/for-best-performance--use-dxgi-flip-model
[latency waitable objects]: https://docs.microsoft.com/en-us/windows/uwp/gaming/reduce-latency-with-dxgi-1-3-swap-chains
[race the beam]: https://blurbusters.com/blur-busters-lagless-raster-follower-algorithm-for-emulator-developers/
