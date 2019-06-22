---
layout: post
title:  "The smooth resize test"
date:   2019-06-21 12:50:42 -0700
categories: [rust, gui]
---
When I was young, as we traveled my dad had a quick test for the quality of a Chinese restaurant: if the tea wasn't good, chances were the food wouldn't be great either. One time, we left before ordering, and I don't think we missed out on much.

Today is an exciting point in the evolution of native GUI in Rust. There is much exploration, and a number of promising projects, but I also think we don't yet know the recipe to make GUI truly great. As I develop my own vision in this space, [druid], I hope more that the efforts will learn from each other and that an excellent synthesis will emerge, more so than simply hoping that druid will win.

In my work, I have come across a problem that is as seemingly simple, yet as difficult to get right, as making decent tea: handling smooth window resizing. Very few GUI toolkits get it perfect, with some failing spectacularly. This is true across platforms, though Windows poses special challenges. It's also pretty easy to test (as opposed to requiring sophisticated latency measurements, which I also plan to develop). I suggest it become one of the basic tests to evaluate a GUI toolkit.

To apply the test, open an app built in your favorite GUI toolkit, and grab the *left* edge of the window. Drag it back and forth, and check to see whether the right edge of the app is stable, especially if it has scrollbars.

Why this particular test? Among other things, it's at the confluence of a number of subsystems, including interfaces with the underlying desktop OS. It also exposes some fundamental architectural decisions, especially regarding asynchrony.

The smooth resizing test also exposes issues at multiple layers – the staging of layout vs drawing within the GUI toolkit, whether requests from the platform can be handled synchronously, and complex interactions between graphics and window management in the platform itself, which the app *may* be able to control to at least some extent. I'll go through these from low level to high level.

## Synchronization with window manager

Resizing a window kicks off two cascades of actions, which are at risk of getting desynchronized. One is the window frame, and the other is the window contents, custom-drawn for the size of the window.

This is what it looks like when it goes wrong. It's a lightly adapted version of the "Hello Triangle" example from Apple.

![wobbly red triangle](/assets/wobbly_hello_triangle.gif)

The fundamental problem here is that the 3D graphics pipeline is essentially asynchronous. Requests go in, get rendered by the GPU, then actually presented on the screen at some later time. For ordinary game-like content when you're not resizing, this is fine, and the asynchronous approach has performance advantages, in particular it lets you do work on the CPU and GPU at the same time. But without some explicit synchronization, you get wobbling as above.

Many people have struggled with this issue, both on macOS and Windows. For example, iTerm2 switches to software rendering while resizing, for this reason. Fortunately, Tristan Hume has recently figured out a recipe to make it work, which you can read in the [glitchless Metal window resizing] blog post. The key insight is to use `CAMetalLayer` instead of `MTKView`, and set `presentsWithTransaction`.

Windows has a similar issue; with newer [flip model] presentation modes optimized for performance, the content goes out of sync with the window drawing. Older presentation modes (which copy the full window contents to a "redirection buffer") are synchronized, so not all UI toolkits are affected. It's mostly an issue when the toolkit is also trying to optimize performance, especially latency, for which there are mechanisms such as wait objects that only work with the flip model.

I spent a *lot* of time experimenting with this on Windows, and finally came up with a [workable recipe]. The short answer is that it switches to rendering in the redirection buffer when sizing (using `WM_ENTERSIZEMOVE`), then back to flip mode at the end of resizing (`WM_EXITSIZEMOVE`). That specific code is tuned for Direct2D, and uses a [HwndRenderTarget](https://docs.microsoft.com/en-us/windows/desktop/api/d2d1/nn-d2d1-id2d1hwndrendertarget) for the sizing case (and on Windows 7), but it could be adapted to Direct3D as well. I think the trick is to use the `DXGI_SWAP_EFFECT_SEQUENTIAL` presentation mode, which in my testing has similar behavior, copying to the redirection buffer and synchronizing with the window manager.

This exploration was a big part of my decision to do my own window creation in [druid], as opposed to using [winit]. I [filed an issue](https://github.com/rust-windowing/winit/issues/786) on winit, but it seems clear they're not able to handle the druid use case yet.

## Synchronous delivery of events

All GUI frameworks are based on an "event loop," which is quite similar to the [game loop]. Unfortunately, the event loop often has complex, messy requirements, especially around threading and reentrancy. I think this is mostly for legacy reasons, as the foundations of UI were laid well before threading entered its modern age.

In an attempt to simplify the programming model, [winit] ran its own event loop in one thread, and the app logic in another, with asychronous coupling using channels between them. For the typical game pipeline, this was indeed a simplification, as the application thread could just be a normal Rust thread, without having to worry about which calls are thread-safe.

However, for smooth window resizing, the completion of draw needs to be synchronized with the window frame resize. This is fundamentally the same issue above, but inside the application logic rather than deep in the platform's swapchain handling.

Fortunately, winit now has an "event loop 2.0" model that gets rid of the extra thread on Windows and allows synchronous events. So it's one step closer to being able to do smooth resize.

## Staging of layout and drawing

In typical immediate mode GUI (imgui), both layout and drawing happen in the same call hierarchy. To keep things reasonably deterministic, it's common for layout to be computed and stored, then drawing based on the *last* frame's layout, in other words a [one-frame delay] for layout to take hold.

I use imgui as an example because this phenomenon is well known, and is a tradeoff for the simplification that imgui brings. But it can happen in any system where there isn't rigorous staging of layout and drawing. To do this right, before any drawing occurs, there needs to be a layout phase where the size and position of each widget is determined, then drawing. Most GUI toolkits do get this right, but some don't.

## Discussion

There are many things that can go wrong when doing window resizing. Therefore, it is a sensitive test for careful platform integration and architectural issues. I've tried hard to get it right in druid, and hope the result of that exploration can be useful to others trying to build desktop GUI.

Thanks to Tristan Hume for making the triangle GIF, and for figuring out the correct recipe on macOS Metal.

[druid]: https://github.com/xi-editor/druid
[areweguiyet]: http://areweguiyet.com/
[game loop]: http://gameprogrammingpatterns.com/game-loop.html
[swap chain]: https://en.wikipedia.org/wiki/Swap_Chain
[double buffering]: https://en.wikipedia.org/wiki/Multiple_buffering#Double_buffering_in_computer_graphics
[flip model]: https://docs.microsoft.com/en-us/windows/desktop/direct3ddxgi/for-best-performance--use-dxgi-flip-model
[one-frame delay]: https://twitter.com/jdryg/status/950640715213795329
[glitchless Metal window resizing]: http://thume.ca/2019/06/19/glitchless-metal-window-resizing/
[workable recipe]: https://github.com/xi-editor/xi-win/pull/21
[winit]: https://github.com/rust-windowing/winit
