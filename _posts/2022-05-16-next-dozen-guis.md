---
layout: post
title:  "Advice for the next dozen Rust GUIs"
date:   2022-07-01 14:26:42 -0700
categories: [rust, gui]
---
A few times a week, someone asks on the #gui-and-ui channel on the Rust Discord, "what is the best UI toolkit for my application?" Unfortunately there is still no clear answer to this question. Generally the top contenders are egui, Iced, and Druid, but web-based approaches are in the running, and of course there's always the temptation to just build a new one. And every couple or months or so, a post appears with a new GUI toolkit.



// Old text below

This post is something of a sequel to [Rust 2020: GUI and community]. I hope to offer a clear-eyed survey of the current state of affairs, and suggestions for how to improve it. It also includes some lessons so far from Druid.

The motivations for building GUI in Rust remain strong. While Electron continues to gain momentum, especially for desktop use cases, there is considerable desire for a less resource-hungry alternative. That said, a consensus has not emerged what that alternative should look like. Rather, it seems there is a new experimental Rust UI toolkit every couple months or so.

In addition, unfortunately there is fragmentation at the level of infrastructure as well.

Fragmentation is not entirely a bad thing. To some extent, specialization can be a good thing, resulting in solutions more adapted to the problem at hand, rather than a one-size-fits-all approach. More importantly, the diversity of UI approaches is a rich ground for experimentation and exploration, as there are many aspects to GUI in Rust where we still don't know the best approach.

## A large tradeoff space

One of the great truths to understand about GUI is that there are few obviously correct ways to do things, and many, many tradeoffs. At present, this tradeoff space is very *sensitive,* in that small differences in requirements and priorities may end up with considerably different implementation choices. I believe this is a main factor driving the fragmentation of Rust GUI.

## On winit

All GUIs (and all games) need a way to create windows, and wire up interactions with that window - primarily drawing pixels into the window and dealing with user inputs such as mouse and keyboard, but potentially a much larger range. The implementation is platform specific and involves many messy details. There is one very popular crate for this function – [winit] – but I don't think a consensus, at least yet, so there are quite a few other alternatives, including the [tao] fork of winit used by Tauri, druid-shell, baseview (which is primarily used in audio applications because it supports the VST plug-in case), and handrolled approaches such as the one used by [makepad].

I would describe the tension this way (perhaps not everyone will agree with me). The *stated* scope of winit is to create a window and leave the details of what happens inside that window to the application. For some interactions (especially GPU rendering, which is well developed in Rust space), that split works well, but for other interactions it is not nearly as clean. In practice, I think winit has evolved to become quite satisfactory for game use cases, but less so for GUI. Big chunks of functionality, such as access to native menus, are missing (the main motivation behind the tao fork), and keyboard support is [persistently below][winit keyboard issue] what's needed for high quality text input in a GUI.

I think resolving some of this fragmentation is possible and would help move the broader ecosystem forward. For the time being, the Druid family of projects will continue developing druid-shell, but is open to collaboration with winit. One way to frame this is that the extra capabilities of druid-shell serve as a set of requirements, as well as guidance and experience how to implement them well.

In the meantime, which windowing library to adopt is a tough choice, and I wouldn't be surprised to see yet another one pop up if the application has specialized requirements. Consider the situation in C++ world: for games, both [GLFW] and [SDL] are good choices, but both primarily for games. Pretty much every serious UI toolkit has its own platform abstraction layer; while it would be possible to use something like SDL for more general purpose GUI, it wouldn't be a great fit.

Advice: think through the alternatives when considering a windowing library. After adopting one, learn from the others to see how yours might be improved. Plan on dedicating some time and energy into improving this important bit of infrastructure.

### Tradeoff: use of system compositor

One of the more difficult, and I think underappreciated tradeoffs is the extent to which the GUI toolkit relies on the system compositor. See [The compositor is evil] for more background on this, but here I'll revisit the issue from the point of view of a GUI toolkit trying to navigate the existing space.

All modern GUI platforms have a compositor which composites the (possibly alpha-transparent) windows from the various applications running on the system. As of Windows 8, Wayland, and Mac OS 10.5, the platform exposes richer access to the compositor, so the application can provide a tree of composition surfaces, and update attributes such as position and opacity (generally providing an additional animation API so the compositor can animate transitions without the application having to provide new values every frame).

If the GUI can be decomposed into the schema supported by the compositor, there are significant advantages. For one, it is decidedly the most power-efficient method to accomplish effects such as scrolling. The system pays the cost of the compositor anyway (in most cases), so any effects that can be done by the compositor (including scrolling) are "for free."

As an illustration of how a Rust UI app may depend on the compositor, see the [Minesweeper using windows-rs] demo. Essentially all presentation is done using the compositor rather than a drawing API (this is why the numbers are drawn using dots rather than fonts). This sample application depends on the [Windows.UI] namespace to be provided by the operating system, so will only run on Windows 10 (build 1803 or later).

TODO: is this a good time for a brief tutorial on Windows deployment choices? This is *massively* confusing from MS. The default choice is WinAPI, which goes back to Windows NT and Windows 95 (when it was called Win32; writing code that works on both 32 and 64 bit targets is relatively straightforward). There are also various iterations of WinRT, of which UWP was heavily promoted then deprecated. Today Windows APP SDK is one of the 

All that said, there are significant *disadvantages* to the compositor as well. One is cross-platform support and compatibility. There is currently no good cross-platform abstraction for the compositor ([planeshift] was an attempt, but is abandoned). Further, older systems (Windows 7 and X11) cannot rely on the compositor, so there has to be a compatibility path, generally with degraded behavior.

There are other more subtle drawbacks. One is a lowest-common-denominator approach, emphasizing visual effects supported by the compositor, especially cross-platform. As just one example, translation and alpha fading is well-supported, but scaling of bitmap surfaces comes with visual degradation, compared with re-rendering the vector original. [TODO: need to explain what it means to depend on WinRT APIs such as Windows.UI] There's also the issue of additional RAM usage for all the intermediate texture layers.

Perhaps the biggest motivation to use the compositor extensively is stitching together diverse visual sources, particularly video, 3D, and various UI embeddings including web and "native" controls. If you want a video playback window to scroll seamlessly and other UI elements to blend with it, there is essentially no other game in town. These embeddings were declared as out of scope for Druid, but people request them often.

Building a proper cross-platform infrastructure for the compositor is a huge and somewhat thankless task. The surface area of these interfaces is large, I'm sure there are lots of annoying differences between the major platforms, and no doubt there will need to be a significant amount of compatibility engineering to work well on older platforms. Browsers have invested in this work (in the case of Safari without the encumbrance of needing to be cross-platform), and this is actually one good reason to use Web technology stacks.

Advice: new UI toolkits should figure out their relationship to the system compositor. If the goal is to provide a truly smooth, native integration of content such as video playback, then they must invest in the underlying mechanisms, much as browsers have.

### Tradeoff: platform text rendering

One of the most difficult aspects of building UI from scratch is getting text right. There are a lot of details to text rendering, but there's also the question of matching the system appearance and being able to access the system font fallback chain. The latter is especially important for non-Latin scripts, but also emoji. Unfortunately, operating systems generally don't have good mechanisms for enumerating or efficiently querying the system fonts. Either you use the built-in text layout capabilities (which means having to build a lowest common denominator abstraction on top of them), or you end up replicating all the work, and finding heuristics and hacks to access the system fonts without running into either correctness or efficiency problems.

There's so much work involved in making a fully functional text input box that it is something of a litmus test for how far along a UI toolkit has gotten. Rendering is only part of it, but there's also IME (including the emoji picker), copy-paste (potentially including rich text), access to system text services such as spelling correction, and one of the larger and richer subsurfaces for integrating accessibility.

Again, browsers have invested a massive amount of work into getting this right, and it's no one simple trick. Druid, by comparison, *does* use the system text layout capabilities, but we're seeing the drawbacks (it tends to be slow, and hammering out all the inconsistencies between platforms is annoying to say the least), so as we go forward we'll probably do more of that ourselves.

Over the longer term, I'd love to have Rust ecosystem infrastructure crates for handling text well, but it's an uphill battle. Just how to design the interface abstraction boundaries is a hard problem, and it's likely that even if a good crate was published, there'd be resistance to adoption because it wouldn't be trivial to integrate. There are thorny issues such as rich text representation, and how the text layout crate integrates with 2D drawing.

Advice: figure out a strategy to get text right. It's not feasible to do that in the short term, but longer term it is a requirement for "real" UI. Potentially this is an area for the UI toolkits to join forces as well.

## On architecture

One constant I've found is that the developer-facing architecture of a UI toolkit needs to evolve. We don't have a One True architecture yet, and in particular designs made in other languages don't adapt well to Rust.

Druid itself has had three major architectures: an intial attempt at applying ECS patterns to UI, the current architecture relying heavily on lenses, and the [Xilem] proposal for a future architecture. In between were two explorations that didn't pan out. Crochet was an attempt to provide an immediate mode API to applications on top of a retained mode implementation, and lasagna was an attempt to decouple the reactive architecture from the underlying widget tree implementation.

There are a number of triggers that might motivate large scale architectural changes in GUI toolkits. Among them, support for multi-window, accessibility, virtualized scrolling (and efficient large containers in general), async.

### The crochet experiment

Now is a good time to review an architectural experiement that ultimately we decided not to pursue. The [crochet prototype][Towards principled reactive UI] was an attempt to emulate immediate mode GUI on top of a retained mode widget tree. The theory was that immediate mode is easier for programmers, while retained mode has implementation advantages including making it easier to do rich layouts. There were other goals, including facilitating language bindings (for langages such as Python) and also better async integration. Language bindings were a pain point in the existing Druid architecture.

Ultimately I think it would be possible to build UI with this architecture, but there were a number of pain points, so I don't believe it would be a good experience overall. One of the inherent problems of an immediate mode API is what I call "state tearing." Because updates to app state (from processing events returned by calls to functions representing widgets) are interleaved with rendering, the rendering of any given frame may contain a mix of old and new state. For some applications, when continuously updating at a high frame rate, an extra frame of latency may not be a serious issue, but I consider it a flaw. I had some ideas for how to address this, but it basically involves running the app logic twice.

There were other ergonomic paper cuts. Because Rust lacks named and optional parameters in function calls, it is painful to add optional modifiers to existing widgets. Architectures based on simple value types for views (as is the case in the in the greater React family, including Xilem) can just use variations on the fluent pattern, method calls on those views to either wrap them or set optional parameters.

Another annoyingly tricky problem was ensuring that begin-container and end-container calls were properly nested. We experimented with a bunch of different ways to do try to enforce this nesting at compile time, but none were fully satisfying.

A final problem with emulating immediate mode is that the architecture tends to thread a mutable context parameter through the application logic. This is not especially ergonomic (adding to "noise") but perhaps more seriously effectively enforces the app logic running on a single thread.

Advice: of course try to figure out a good architecture, but also plan for it to evolve.

## What of Druid?

I admit I had hopes that Druid would become the popular choice for Rust GUI, though I've never explicitly had that as a goal. In any case, that hasn't happened, and now is a time for serious thought about the future of the project.

We now have clearer focus that Druid is primarily a research project, with a special focus on high performance, but also solving the problems of "real UI." The research goals are longer term; it is *not* a project oriented to shipping a usable toolkit soon. Thus, we are making some changes along these lines. We hope to get [piet-gpu] to a "minimum viable" 0.1 release soon, at which point we will be switching drawing over to that, as opposed to the current strategy of wrapping platform drawing capabilities (which often means that drawing is on the CPU). We change the reactive architecture to Xilem.

Assuming we do a good job solving these problems, over time Druid might evolve into a toolkit usable for production applications. In the meantime, we don't want to create unrealistic expectations. The primary audience for Druid is people learning how to build UI in Rust. This post isn't the appropriate place for a full roadmap and vision document, but I expect to be writing more about that in time.

## Conclusion

I don't want to make too many predictions, but I am confident in asserting that there will be a dozen new UI projects in Rust in the next year or two. Most of them will be toys, though it is entirely possible that one or more of them will be in support of a product and will attract enough resources to build something approaching a "real" toolkit. I do expect fragmentation of infrastructure to continue, as there are legitimate reasons to choose different approaches, or emphasize different priorities. It's possible we never get to a "one size fits all" solution for especially thorny problems such as window creation, input (including keyboard input and IME), text layout, and accessibility.

Meanwhile we will be pushing forward with Druid. It won't be for everyone, but I am hopeful it will move the state of Rust UI forward. I'm also hopeful that the various projects will continue to learn from each other and build common ground on infrastructure where that makes sense.

And I remain very hopeful about the potential for GUI in Rust. It seems likely to me that it will be the language the next major GUI toolkit is written in, as no other language offers the combination of performance, safety, and high level expressiveness. All of the issues in this post are problems to be solved rather than obstacles why Rust isn't a good choice for building UI.

[Rust 2020: GUI and community]: https://raphlinus.github.io/rust/druid/2019/10/31/rust-2020.html
[winit]: https://github.com/rust-windowing/winit
[tao]: https://github.com/tauri-apps/tao
[makepad]: https://github.com/makepad/makepad
[Xilem]: https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html
[winit keyboard issue]: https://github.com/rust-windowing/winit/pull/1890
[iced revert system menus]: https://github.com/iced-rs/iced/pull/1047
[Vizia]: https://github.com/vizia/vizia
[GUI framework ingredients]: https://www.cmyr.net/blog/gui-framework-ingredients.html
[piet-gpu]: TODO
[The compositor is evil]: https://raphlinus.github.io/ui/graphics/2020/09/13/compositor-is-evil.html
[DirectComposition]: https://docs.microsoft.com/en-us/windows/win32/directcomp/directcomposition-portal
[Windows.UI.Composition]: https://docs.microsoft.com/en-us/uwp/api/windows.ui.composition?view=winrt-22621
[Minesweeper using windows-rs]: https://github.com/robmikh/minesweeper-rs
[Windows.UI]: https://docs.microsoft.com/en-us/uwp/api/windows.ui?view=winrt-22621
[SDL]: https://www.libsdl.org/
[GLFW]: https://www.glfw.org/
[planeshift]: https://github.com/pcwalton/planeshift
[Towards principled reactive UI]: https://raphlinus.github.io/rust/druid/2020/09/25/principled-reactive-ui.html