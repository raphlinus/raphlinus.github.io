---
layout: post
title:  "More small updates"
date:   2019-02-20 10:02:42 -0700
categories: [personal]
---
This post is actually a collection of updates about pretty big things, but the writeup is small. Several will be expanded into larger blog posts - if there are any that you are especially eager to see, please tweet at me and I'll give the topic priority.

## GUI in Rust

Most of my work in recent weeks has been towards building a native-Rust GUI stack. In particular, [druid] has been migrating from being Windows-only to being cross-platform, and at the heart of that work is [piet], a 2D graphics abstraction. I believe piet is now at a "minimum viable" stage. Right now, we're mostly using Cairo for non-Windows platforms, but I consider that a placeholder. One of the things I'd really like to see from the community is a piet back-end based on [WebRender] and [PathFinder]. However, I don't consider that blocking the rest of the work.

I'll do a talk jointly with Ryan Levick at [Libre Graphics Meeting 2019] on the graphics part of the stack.

### Text layout

One of the missing pieces in the Rust ecosystem is text layout. This is a critically important problem for native GUI, but is also needed in other places. I'm starting a project, supported by Mozilla Research under the Servo banner, to do the low-level parts of text rendering. This work will be done in the open, and I expect to be writing *lots* about it in coming weeks. For now, there is a [roadmap document].

Another thing on the radar: the folks at YesLogic are working on pure-Rust code for OpenType shaping. I'm in touch with them and am hoping to be an early customer.

### Druid on Mac

A major milestone will be getting the druid examples running on macOS. We're not quite there yet, but the druid-shell example does run. I'm hoping soon. A lot of the progress here has been through open source collaboration, and I expect that to continue.

### Why not winit?

One of the deeper topics I've been engaging is whether to use [winit] for cross-platform window creation. It's a popular crate, and lots of work has gone into it.

I have decided to do window creation myself. I believe winit is fundamentally based on an architectural decision which is ok for 3D games but not for general GUI work: a separate Rust event loop thread that coordinates asynchronously with the host's UI loop. However, some UI events require synchronous handling. This comes up visibly in smooth resize, but there are other instances. I filed an [issue against winit] about smooth resizing specifically.

Another reason not to use winit is the VST use case. The [Rust DSP] community has also decided not to use winit, because they need finer grained access to the window creation process; a VST is given a handle to the host UI, and needs to instantiate a view within that, as opposed to creating a window and view from scratch; winit has architectural decisions that basically assume the latter case. I'm in touch with that community and am hoping the druid work will meet their needs.

There's a bit more info in the [druid-shell roadmap].

[Rust DSP]: https://github.com/rust-dsp
[druid-shell roadmap]: https://github.com/xi-editor/druid/issues/16

## Spline

The spline work has been on the back burner, but it has been accepted to [Libre Graphics Meeting 2019] and I expect to polish it up considerably between now and then. Among other things, Jacob Rus has been tweaking the 2-parameter curve family to make it smoother and closer to curvature-monotonic.

I *might* have more news about open source spline work soon, watch this space.

## Markdown parsing

The [pulldown-cmark] project has been in a fairly stuck state for a while, largely because I realized the existing codebase had fundamental problems, so decided to start a new branch ([new_algo]), with the hope of recruiting open source contribution to get to 100% spec compatibility. Recently, [Marcus Klaas de Vries] has taken this up, and gotten it quite close. I'm hopeful we can merge to master and do a release soonish. Here's a [roadmap][pulldown-cmark roadmap] for the work.

## Follow the work

I've been doing more coding than writing about my projects lately. I'm going to be picking up the pace on this blog, as these projects increasingly run on open source collaboration. For earlier, more fine grained info, join the Zulip chat at [xi.zulipchat.com].

Again, if you want me to dig into any of these topics, let me know. Combining that with a [Patreon] donation is an even better way to get my attention.

[Libre Graphics Meeting 2019]: https://libregraphicsmeeting.org/2019/
[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[roadmap document]: https://drive.google.com/open?id=1aw41q_izail-p99mN8dHrJeh9tMQ-Pldi54W6m7MHU8
[Marcus Klaas de Vries]: https://marcusklaas.nl/
[xi.zulipchat.com]: https://xi.zulipchat.com
[druid]: https://github.com/xi-editor/druid
[xi-win]: https://github.com/xi-editor/xi-win
[new_algo]: https://github.com/raphlinus/pulldown-cmark/tree/new_algo
[WebRender]: https://github.com/servo/webrender
[PathFinder]: https://github.com/pcwalton/pathfinder
[winit]: https://github.com/tomaka/winit
[issue against winit]: https://github.com/tomaka/winit/issues/786
[pulldown-cmark roadmap]: https://github.com/raphlinus/pulldown-cmark/issues/154
[Patreon]: https://www.patreon.com/raphlinus
