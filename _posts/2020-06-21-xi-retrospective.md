---
layout: post
title:  "xi-editor retrospective"
date:   2020-06-27 10:16:03 -0700
categories: [xi]
---
A bit more than four years ago I started the [xi-editor] project. Now I have placed it on the back burner (though there is still some activity from the open source community).

The original goal was to deliver a very high quality editing experience. To this end, the project spent a rather large number of "novelty points":

* Rust as the implementation language for the core.
* A rope data structure for text storage.
* A multiprocess architecture, with front-end and plug-ins each with their own process.
* Fully embracing async design.
* [CRDT] as a mechanism for concurrent modification.

I still believe it would be possible to build a high quality editor based on the original design. But I *also* believe that this would be quite a complex system, and require significantly more work than necessary.

I've written the [CRDT part of this retrospective][CRDT comment] already, as a comment in response to a Github issue. That prompted good [discussion][CRDT discussion] on Hacker News. In this post, I will touch again on CRDT but will focus on the other aspects of the system design.

## Origins

The original motivation for xi came from working on the Android text stack, and confronting two problems in particular. One, text editing would become very slow as the text buffer got bigger. Two, there were a number of concurrency bugs in the interface between the EditText widget and the keyboard (input method editor).

The culprit of the first problem turned out to be the [SpanWatcher] interface, combined with the fact that modern keyboards like to put a spelling correction span on each word. When you insert a character, all the successive spans bump their locations up by one, and then you have to send onSpanChanged for each of those spans to all the watchers. Combined with the fact that the spans data structure had a naive O(n) implementation, and the whole thing was quadratic or worse.

The concurrency bugs boil down to synchronizing edits across two different processes, because the keyboard is a different process than the application hosting the EditText widget. Thus, when you send an update (to move the cursor, for example) and the text on the other side is changing concurrently, it's ambiguous whether it refers to the old or new location. This was handled in an "almost correct" style, with timeouts for housekeeping updates to minimize the chance of a race. A nice manifestation of that is that swiping the cursor slowly through text containing complex emoji could cause flashes of the emoji breaking.

These problems have a unifying thread: in both cases there are small diffs to the text, but then the data structures and protocols handled these diffs in a less than optimal way, leading to both performance and correctness bugs.

To a large extent, xi started as an exploration into the "right way" to handle text editing operations. In the case of the concurrency bugs, I was hoping to find a general, powerful technique to facilitate concurrent text editing in a distributed-ish system. While most of the Operational Transformation literature is focused on multiple users collaboratively editing a document, I was hoping that other text manipulations (like an application enforcing credit card formatting on a text input field) could fit into the general framework.

That was also the time I was starting to get heavily into Rust, so it made natural sense to start prototyping a new green-field text editing engine. How would you "solve text" if you were free of backwards compatibility constraints (a huge problem in Android)?

When I started, I knew that Operational Transformation was a solution for collaborative editing, but had a reputation for being complex and finicky. I had no idea how deep the rabbithole would be of OT and then CRDT. Much of that story is told in the [CRDT discussion] previously linked.

## The lure of modular software

There is an extremely long history of people trying to build software as composable modules connected by some kind of inter-module communication fabric. Historical examples include [DCE/RPC], [Corba], [Bonobo], and more recently things like [Sandstorm] and [Fuchsia Modular]. There are some partial successes, including [Binder] on Android, but this is still mostly an unrealized vision. (Regarding Binder, it evolved from a much more idealistic vision, and I strongly recommend reading this 2006 interview about [OpenBinder]).

When I started xi, there were signs we were getting there. Microservices were becoming popular in the Internet world, and of course all Web apps have a client/server boundary. Within Google, [gRPC] was working fairly well, as was the internal process separation within Chrome. In Unix land, there's a long history of the terminal itself presenting a GUI (if primitive, though gaining features such as color and mouse). There's also the tradition of [Blit] and then, of course, [NeWS] and X11.

I think one of the strongest positive models was the database / business logic split, which is arguably the most successful example of process separation. In this model, the database is responsible for performance and integrity, and the business logic is in a separate process, so it can safely do things like crash and hang. I very much thought of xi-core as a database-like engine, capable of handling concurrent text modification much like a database handles transactions.

Building software in such a modular way requires two things: first, infrastructure to support remote procedure calls (including serialization of the requests and data), and second, well-defined interfaces. Towards the end of 2017, I saw the goal of xi-editor as *primarily* being about defining the interfaces needed for large scale text editing, and that this work could endure over a long period of time even as details of the implementation changed.

For the infrastructure, we chose JSON (about which more below) and hand-rolled our own xi-rpc layer (based on JSON-RPC). It turns out there are a lot of details to get right, including dealing with error conditions, negotiating when two ends of the protocol aren't exactly on the same version, etc.

One of the bolder design decisions in xi was to have a process separation between front-end and core. This was inspired in part by [Neovim](https://neovim.io/), in which everything is a plugin, even GUI. But the main motivation was to build GUI applications using Rust, even though at the time Rust was nowhere near capable of native GUI. The idea is that you use the best GUI technology of the platform, and communicate via async pipes.

One argument for process separation is to improve overall system reliability. For example, Chrome has a process per tab, and if the process crashes, all you get is an "Aw, snap" without bringing the whole browser down. I think it's worth asking the question: is it useful to have the front-end continue after the core crashes, or the other way around? I think probably not; in the latter case it might be able to safely save the file, but you can also do that by frequently checkpointing.

Looking back, I see much of the promise of modular software as addressing goals related to project management, not technical excellence. Ideally, once you've defined an inter-module architecture, then smaller teams can be responsible for their own module, and the cost of coordination goes down. I think this type of project management structure is especially appealing to large companies, who otherwise find it difficult to manage larger projects. And the tax of greater overall complexity is often manageable, as these big companies tend to have more resources.

### JSON

The choice of JSON was controversial from the start. It did end up being a source of friction, but for surprising reasons.

The original vision was to write plug-ins in any language, especially for things like language servers that would be best developed in the language of that ecosystem. This is the main reason I chose JSON, because I expected there would be high quality implementations in every viable language.

Many people complained about the fact that JSON escapes strings, and suggested alternatives such as [MessagePack]. But I knew that the speed of raw JSON parsing was a solved problem, with a number of extremely high performance implementations ([simdjson] is a good example).

Even so, aside from the general problems of modular software as described above, JSON was the source of two additional problems. For one, [JSON in Swift is shockingly slow]. There are [discussions on improving it](https://forums.swift.org/t/rearchitecting-jsonencoder-to-be-much-faster/28139) but it's still a problem. This is surprising to me considering how important it is in many workloads, and the fact that it's clearly possible to write a high performance JSON implementation.

Second, on the Rust side, while [serde] is quite fast and very convenient (thanks to proc macros), when serializing a large number of complex structures, it bloats code size considerably. The xi core is 9.3 megabytes in a Linux release build (debug is an eye-watering 88MB), and a great deal of that bloat is serialization. There is work to reduce this, including [miniserde](https://github.com/dtolnay/miniserde) and [nanoserde](https://github.com/not-fl3/nanoserde), but serde is still by far the most mainstream.

I believe it's possible to do performant, clean JSON across most languages, but people should know, we're not there yet.

## The rope

There are only a few data structures suitable for representation of text in a text editor. I would enumerate them as: contiguous string, gapped buffer, array of lines, piece table, and rope. I would consider the first unsuitable for the goals of xi-editor as it doesn't scale well to large documents, though its simplicity is appealing, and memcpy is fast these days; if you know your document is always under a megabyte or so, it's probably the best choice.

Array of lines has performance failure modes, most notably very long lines. Similarly, many good editors have been written using piece tables, but I'm not a huge fan; performance is very good when first opening the file, but degrades over time.

My favorite aspect of the rope as a data structure is its excellent worst-case performance. Basically, there aren't any cases where it performs *badly.* And even the concern about excess copying because of its immutability might not be a real problem; Rust has a [copy-on-write mechanism](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.make_mut) where you can mutate in-place when there's only one reference to the data.

The main argument against the rope is its complexity. I think this varies a lot by language; in C a gapped buffer might be preferable, but I think in Rust, a rope is the sweet spot. A large part of the reason is that in C, low level implementation details tend to leak through; you'll often be dealing with a pointer to the buffer. For the common case of operations that don't need to span the gap, you can hand out a pointer to a contiguous slice, and things just don't get any simpler than that. Conversely, if any of the invariants of the rope are violated, the whole system will just fall apart.

In Rust, though, things are different. Proper Rust style is for all access to the data structure to be mediated by a well-defined interface. Then the details about how that's implemented are hidden from the user. A good way to think about this is that the implementation has complexity, but that complexity is *contained.* It doesn't leak out.

I think the rope in xi-editor meets that ideal. A lot of work went into getting it right, but now it works. Certain things, like navigating by line and counting UTF-16 code units, are easy and efficient. It's built in layers, so could be used for other things including binary editing.

One of the best things about the rope is that it can readily and safely be shared across threads. Ironically we didn't end up making much use of that in xi-editor, as it was more common to share across *processes,* using sophisicated diff/delta and caching protocols.

A rope is a fairly niche data structure. You really only want it when you're dealing with large sequences, and also doing a lot of small edits on them. Those conditions rarely arise outside text editors. But for people building text editing in Rust, I think xi-rope holds up well and is one of the valuable artifacts to come from the project.

There's a good [HN discussion of text editor data structures](https://news.ycombinator.com/item?id=15381886) where I talk about the rope more, and can also point people to the [Rope science](https://xi-editor.io/docs/rope_science_00.html) series for more color.

## Async is a complexity multiplier

We knew going in that async was going to be a source of complexity. The hope is that we would be able to tackle the async stuff once, and that the complexity would be encapsulated, much as it was for the rope data structure.

The reality was that adding async made everything more complicated, in some cases considerably so. A particularly difficult example was dealing with word wrap. In particular, when the width of the viewport is tied to the window, then live-resizing the window causes text to rewrap continuously. With the process split between front-end and core, and an async protocol between them, all kinds of interesting things can go wrong, including races between editing actions and word wrap updates. More fundamentally, it is difficult to avoid tearing-style artifacts.

One early relative success was implementing scrolling. The problem is that, as you scroll, the front-end needs to sometimes query the core to fetch visible text that's outside its cache. We ended up building this, but it took months to get it right. By contrast, if we just had the text available as an in-process data structure for the UI to query, it would have been quite straightforward.

I should note that async in interactive systems is more problematic than the tamer variety often seen in things like web servers. There, the semantics are generally the same as simple blocking threads, just with (hopefully) better performance. But in an interactive system, it's generally possible to observe internal states. You have to display *something*, even when not all subqueries have completed.

As a conclusion, while the process split with plug-ins is supportable (similar to the Language Server protocol), I now firmly believe that the process separation between front-end and core was not a good idea.

## Syntax highlighting

Probably the high point of the project was the successful implementation of syntax highlighting, based on Tristan Hume's [syntect] library, which was motivated by xi. There's a lot more to say about this.

First, TextMate / Sublime style syntax highlighting is not really all that great. It is quite slow, largely because it grinds through a lot of regular expressions with captures, and it is also not very precise. On the plus side, there is a large and well-curated open source collection of syntax definitions, and it's definitely "good enough" for most use. Indeed, code that fools these syntax definitions (such as two open braces on the same line) is a good anti-pattern to avoid.

It may be surprising just how much slower regex-based highlighting is than fast parsers. The library that xi uses, syntect, is probably the fastest open source implementation in existence (the one in Sublime is faster but not open source). Even so, it is approximately 2500 times slower for parsing Markdown than [pulldown-cmark]. And syntect doesn't even parse setext-style lists correctly, because Sublime style syntax definitions have to work line-at-a-time, and the line of dashes following a heading isn't available until the next line.

These facts influenced the design of xi in two important ways. First, I took it as a technical challenge to provide a high-performance editing experience even on large files, overcoming the performance problems through async. Second, the limitations of the regex-based approach argued in favor of a modular plug-in architecture, so that as better highlighters were developed, they could be plugged in. I had some ambitions of creating a standard protocol that could be used by other editors, but this absolutely failed to materialize. For example, Atom instead developed [tree-sitter].

In any case, I dug in and did it. The resulting implementation is impressive in many ways. The syntax highlighter lives in a different process, with asynchronous updates so typing is never slowed down. It's also incremental, so even if changes ripple through a large file, it updates what's on the screen quickly. Some of the sophistication is described in [Rope science 11].

There was considerable complexity in the implementation. Text was synchronized between the main xi-core process and the plug-in, but for large files, the latter stores only a fixed-size cache; the cache protocol ended up being quite sophisticated. Updates were processed through a form of Operational Transformation, so if a highlighting result raced a text edit, it would never color an incorrect region (this is still very much a problem for language server annotations).

As I said, syntax highlighting was something of a high point. The success suggested that a similar high-powered engineering approach could systematically work through the other problems. But this was not to be.

As part of this work, I explored an alternative syntax highlighting engine based on parser combinators. If I had pursued that, the result would have been lightning fast, of comparable quality to the regex approach, and difficult to create syntax descriptions, as it involved a fair amount of manual factoring of parsing state. While the performance would have been nice to have, ultimately I don't think there's much niche for such a thing. If I were trying to create the best possible syntax highlighting experience today, I'd adapt Marijn Haverbeke's [Lezer].

To a large extent, syntax highlighting is a much easier problem than many of the others we faced, largely because the annotations are a history-free function of the document's plain text. The problem of determining indentation may seem similar, but is dependent on history. And it basically doesn't fit nicely in the CRDT model at all, as that requires the ability to resolve arbitrarily divergent edits between the different processes (imagine that one goes offline for a bit, types a bit, then the language server comes back online and applies indentation).

Another problem is that our plug-in interface had become overly specialized to solve the problems of syntax highlighting, and did not well support the other things we wanted to do. I think those problems could have been solved, but only with significant difficulty.

## There is no such thing as native GUI

As mentioned above, a major motivation for the front-end / core process split was to support development of GUI apps using a polyglot approach, as Rust wasn't a suitable language for building GUI. The theory was that you'd build the GUI using whatever libraries and language that was most suitable for the platform, basically the platform's native GUI, then interact with the Rust engine using interprocess communication.

The strongest argument for this is probably macOS, which at the time had Cocoa as basically *the* blessed way to build GUI. Most other platforms have some patchwork of tools. [Windows](https://docs.microsoft.com/en-us/windows/apps/desktop/choose-your-platform) is particularly bad in this respect, as there's old-school (GDI+ based) win32, WinForms, WPF, Xamarin, and most recently [WinUI](https://microsoft.github.io/microsoft-ui-xaml/), which nobody wants to use because it's Windows 10 only. Since xi began, macOS is now catching up in the number of official frameworks, with [Catalyst](https://developer.apple.com/mac-catalyst/) and SwiftUI added to the roster. Outside the realm of official Apple projects, lots of stuff is shipping in Electron these days, and there are other choices including Qt, Flutter, Sciter, etc.

When doing some [performance work](https://www.recurse.com/events/localhost-raph-levien) on xi, I found to my great disappointment that performance of these so-called "native" UI toolkits was often pretty poor, even for what you'd think of as the relatively simple task of displaying a screenful of text. A large part of the problem is that these toolkits were generally made at a time when software rendering was a reasonable approach to getting pixels on screen. These days, I consider GPU acceleration to be essentially required for good GUI performance. There's a whole other blog post in the queue about how some toolkits try to work around these performance limitations by leveraging the compositor more, but that has its own set of drawbacks, often including somewhat ridiculous RAM usage for all the intermediate textures.

I implemented an OpenGL-based text renderer for xi-mac, and did similar explorations on Windows, but this approach gives up a lot of the benefits of using the native features (as a consequence, emoji didn't render correctly). Basically, I discovered that there is a pretty big opportunity to build UI that doesn't suck.

Perhaps the most interesting exploration was on Windows, the [xi-win](https://github.com/xi-editor/xi-win) project. Originally I was expecting to build the front-end in C# using one of the more mainstream stacks, but I also wanted to explore the possibility of using lower-level platform capabilities and programming the UI in Rust. Early indications were positive, and this project gradually morphed into [Druid], a native Rust GUI toolkit which I consider very promising.

If I had said that I would be building a GUI toolkit from scratch as part of this work when I set out, people would have rightly ridiculed the scope as far too ambitious. But that is how things are turning out.

## Fuchsia

An important part of the history of the project is its home in Fuchsia for a couple years. I was fortunate that the team was willing to invest in the xi vision, including funding Colin's work and letting me host Tristan to build multi-device collaborative editing as an intern project. In many ways the goals and visions aligned, and the demo of that was impressive. Ultimately, though, Fuchsia was not at the time (and still isn't) ready to support the kind of experience that xi was shooting for. Part of the motivation was also to develop a better IME protocol, and that made some progress (continued by Robert Lord, and you can read about some of what we discovered in [Text Editing Hates You Too](https://lord.io/blog/2019/text-editing-hates-you-too/)).

It's sad this didn't work out better, but such is life.

## A low point

My emotional tone over the length of the project went up and down, with the initial enthusiasm, stretches of slow going, a renewed excitement over getting the syntax highlighting done, and some other low points. One of those was learning about the [xray](https://github.com/atom-archive/xray) project. I probably shouldn't have taken this personally, as it is *very common* in open source for people to spin up new projects for a variety of reasons, not least of which is that it's fun to do things yourself, and often you learn a lot.

Even so, xray was a bit of a wake-up call for me. It was evidence that the vision I had set out for xi was not quite compelling enough that people would want to join forces. Obviously, the design of xray had a huge amount of overlap with xi (including the choice of Rust and decision to use a CRDT), but there were other significant differences, particularly the choice to use Web technology for the UI so it would be cross-platform (the fragmented state of xi front-ends, especially the lack of a viable Windows port, was definitely a problem).

I'm putting this here because often, how you *feel* about a project is just as important, even more so, than technical aspects. I now try to listen more deeply to those emotional signals, especially valid criticisms.

## Community

Part of the goal of the project was to develop a good open-source community. We did pretty well, but looking back, there are some things we could have done better.

A lot of the friction was simply the architectural burden described above. But in general I think the main thing we could have done better is giving contributors more *agency.* If you have an idea for a feature or other improvement, you should be able to come to the project and do it. The main role of the maintainers should be to help you do that. In xi, far too often things were blocking on some major architectural re-work (we have to redo the plug-in API before you can implement that feature). One of the big risks in a modular architecture is that it is often expedient to implement things in one module when to do things "right" might require it in a different place, or, even worse, require changes in inter-module interfaces. We had these decisions a lot, and often as maintainers we were in a gate-keeping role. One of the worst examples of this was vi keybindings, for which there was a great deal of community interest, and even a [project done off to the side](https://github.com/Peltoche/vixi) to try to achieve it, but never merged into the main project.

So I think monolithic architectures, perhaps ironically, are *better* for community. Everybody takes some responsibility for the quality of the whole.

In 2017 we hosted three Google Summer of Code Students: Anna Scholtz, Dzũng Lê, and Pranjal Paliwal. This worked out well, and I think GSoC is a great resource.

I have been fortunate for almost the entire time to have Colin Rofls taking on most of the front-line community interaction. To the extent that xi has been a good community, much of the credit is due him.

One of the things we have done very right is setting up a Zulip instance. It's open to all with a Github account, but we have had virtually no difficulty with moderation issues. We try to maintain positive interactions around all things, and lead by example. This continues as we pivot to other things, and may be one of the more valuable spin-offs of the project.

## Conclusion

The xi-editor project had very ambitious goals, and bet on a number of speculative research subprojects. Some of those paid off, others didn't. One thing I would do differently is more clearly identify which parts are research and which parts are reasonably straightforward implementations of known patterns. I try to do that more explicitly today.

To a large extent the project was optimized for learning rather than shipping, and through that lens it has been pretty successful. I now know a lot more than I did about building editor-like GUI applications in Rust, and am now applying that to making the [Druid] toolkit and the [Runebender] font editor. Perhaps more important, because these projects are more ambitious than one person could really take on, the community started around xi-editor is evolving into one that can sustain GUI in Rust. I'm excited to see what we can do.

Discuss on [Hacker News](https://news.ycombinator.com/item?id=23663878) and [/r/rust](https://www.reddit.com/r/rust/comments/hgzdu5/xieditor_retrospective/).

[xi-editor]: https://github.com/xi-editor/xi-editor
[CRDT comment]: https://github.com/xi-editor/xi-editor/issues/1187#issuecomment-491473599
[CRDT discussion]: https://news.ycombinator.com/item?id=19886883
[CRDT]: https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type
[SpanWatcher]: https://developer.android.com/reference/android/text/SpanWatcher
[MessagePack]: https://msgpack.org/index.html
[simdjson]: https://github.com/simdjson/simdjson
[JSON in Swift is shockingly slow]: https://github.com/xi-editor/xi-mac/issues/102
[DCE/RPC]: https://en.wikipedia.org/wiki/DCE/RPC
[Corba]: https://en.wikipedia.org/wiki/Common_Object_Request_Broker_Architecture
[Bonobo]: https://en.wikipedia.org/wiki/Bonobo_(GNOME)
[Fuchsia Modular]: https://fuchsia.dev/fuchsia-src/concepts/modular/module
[Binder]: https://developer.android.com/reference/android/os/Binder
[OpenBinder]: https://www.osnews.com/story/13674/introduction-to-openbinder-and-interview-with-dianne-hackborn/
[Blit]: https://en.wikipedia.org/wiki/Blit_(computer_terminal)
[syntect]: https://github.com/trishume/syntect
[tree-sitter]: https://github.blog/2018-10-31-atoms-new-parsing-system/
[pulldown-cmark]: https://github.com/raphlinus/pulldown-cmark
[Rope science 11]: https://xi-editor.io/docs/rope_science_11.html
[Druid]: https://github.com/linebender/druid
[serde]: https://serde.rs/
[Sandstorm]: https://sandstorm.io/
[Runebender]: https://github.com/linebender/runebender
[Lezer]: https://marijnhaverbeke.nl/blog/lezer.html
[NeWS]: https://en.wikipedia.org/wiki/NeWS
[gRPC]: https://grpc.io/
