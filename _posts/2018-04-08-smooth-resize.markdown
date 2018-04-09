---
layout: post
title:  "Smooth resize in Direct2D"
date:   2018-04-08 16:58:03 -0700
categories: personal
---
In my fun time I've been working on [xi-win](https://github.com/google/xi-win), a Windows front-end for xi-editor. The idea is to write at a fairly low level so I can optimize performance. I've had fun experimenting but have become stuck on achieving smooth window resize. I have two solutions that almost work, but both have flaws. I'd still like to make forward progress on xi-win, but don't want to burn more time on this resizing issue. Solving it also affects my motivation; for me, putting time into xi-win directly relates to being able to provide a superior user experience.

First, I'll present the challenge, then share notes about why it's so hard. I offer $2500 to anyone who can provide a PR against xi-win that achieves the following:

1. The spinner in the [perftest] example spins smoothly at 60fps without artifacts, on both the laptop display and the external monitor. In particular, it must not display a [diagonal tearing](https://www.reddit.com/r/nvidia/comments/5qj5xx/new_workaround_tool_for_nvidia_optimus_diagonal/) artifact on the laptop display.

2. Resizing the window "sticks to" the mouse, with no artifacts and no lag, on both laptop and external. The best way to validate lack of artifacts is to grab the _left_ edge of the window, and observe that the top right of the diagonal line stays glued to the window corner.

3. Both of the above properties hold true whether the primary display is set to internal or external.

4. Acceptance is based on my personal laptop, a Gigabyte Aero 14 (with GTX 1060), connected to a 4k external monitor, running latest Windows 10. However, it's better to provide evidence that the technique will work across a range of devices. In particular, I'm interested in refresh rates above 60Hz.

5. I'm more interested in [flip-model](https://msdn.microsoft.com/en-us/library/windows/desktop/hh706346(v=vs.85).aspx) presentation, because [incremental present](https://msdn.microsoft.com/en-us/library/windows/desktop/hh706345(v=vs.85).aspx) works (reducing latency and power), the window can display actual contents at startup (rather than displaying the gdi-drawn background color), and more precise frame timing (especially in multi-monitor setups).

6. It might not be possible at all. I'll pay out half for a compelling analysis arguing the impossibility.

## What I've tried

I've tried a bunch of stuff. Here are notes on how the approaches almost work.

### HWND render targets

The current xi-win master is based on an [hwnd render target](https://msdn.microsoft.com/en-us/library/windows/desktop/dd371461(v=vs.85).aspx). This is older tech, and generally people are [recommended](https://msdn.microsoft.com/en-us/magazine/dn198239.aspx) to upgrade to swapchains using dxgi 1.1, but it works ok.

From my experimentation, it seems to use the GPU that's associated with the primary display. Thus, if the primary display is external and the window is on the laptop, it'll render on the 1060 and then transfer the frames to the integrated HD 630. In addition to not being awesome for performance, NVidia's Optimus architecture seems to suffer from the diagonal tearing artifacts linked above, so fail on the artifact requirement (1).

Conversely, if the primary display is internal, then the poor 630 is trying to render 4k content, which it can't quite do, so fail on the 60fps requirement of (1).

However, hwnd seems to work quite well on resize, better than any swapchain approach. In cases where the GPU is matched with the display, I'm happy. Thus, getting hwnd to use the most appropriate GPU would count as a solution.

### Sequential swap effect

I've also tried DXGI swapchains in a number of combinations. The experiments are in the [dxgi](https://github.com/google/xi-win/tree/dxgi) branch of xi-win. The DXGI_SWAP_EFFECT_SEQUENTIAL flip mode behaves very similarly to hwnd. One difference is that it is possible to set the adapter programatically. The current branch always tries to choose discrete graphics (using the heuristic of maximum dedicated vram). Having it automatically choose the 

One thing I've observed is that incremental present has no effect in the SEQUENTIAL swap effect, it seems to always copy the entire surface to the redirection bitmap even if the dirty rectangle is specified smaller. I'd like to use incremental present to minimize the work (and thus latency and power) when typing and for cursor blink, but it's not a dealbreaker. I think it's likely that this worked in earlier versions of Windows.

For this swap effect, I handle resizes in a fairly straightforward way; on WM_SIZE I resize the swapchain and invalidat the window, so the next WM_PAINT causes a [Present](https://msdn.microsoft.com/en-us/library/windows/desktop/bb174576(v=vs.85).aspx) with a SyncInterval of 1 frame. It seems like Windows is internally synchronizing that present with the display of the new window size, as they land in the same frame even if the rendering time is long.

### Flip swap effect

The flip swap effect is considered the most advanced, but it also creates significant challenges. The fundamental problem is that there's no synchronization between the flip of the swapchain and the window size.

Yet, I've been able to make it work reasonably when using the 1060 to drive an external monitor case (effectively, it works when the render time is very small). On WM_SIZE I do a Present with a SyncInterval of 0, followed by a DwmFlush. I believe what happens is that the DwmFlush delays until just after vsync. Then, on the next cycle of resize, both the window size change and the corresponding frame render get issued early in the frame timeline, so if both are fast they both land within the same frame.

However, either using integrated graphics or an internal monitor, it doesn't work well, and when the contents and window border change end up in different frames, the result is not pretty. Definitely fail on requirement (2). This observation leads me to believe that it won't work well on hardware that isn't high end.

Another issue with the flip swap effect is that it works best with the redirection surface disabled. A consequence of that is that any content drawn with GDI, including the classic menubar, disappears. It'd be handy to use native menus, but if giving up GDI is a price to pay for good Direct2D performance, I'm willing to pay it.

The dxgi branch also has a `PresentStrategy::FlipRedirect` that leaves the redirection surface on, but the performance is significantly degraded, and there is serious artifacting on resize, where it reveals either or both of the GDI background and the swapchain background, depending. It's there as a stopgap but I wouldn't feel comfortable.

## Other resources

* [PresentMon](https://github.com/GameTechDev/PresentMon): a useful tool for diagnosing swapchain presentation

* [Smooth window resizing in Windows (using Direct2D 1.1)?](https://stackoverflow.com/questions/21816323/smooth-window-resizing-in-windows-using-direct2d-1-1): Stack Overflow thread

