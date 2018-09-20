---
layout: post
title:  "Synthesizer progress update"
date:   2018-09-19 09:26:03 -0700
categories: [synthesizer]
---
I've made good progress on my music synthesizer, and wanted to post a quick update.

## A working tech demo

If you're on Windows, clone [raphlinus/synthesizer-io](https://github.com/raphlinus/synthesizer-io), go to `synthesizer-io-win`, and then `cargo run`. You'll get a working synthesizer with a piano keyboard you can play with the mouse. If you have a MIDI keyboard plugged in, you can play that. If that's an Akai MPK Mini, then the knobs are mapped to filter cutoff, resonance, and ADSR for the envelope. It's not a very exciting sound, and it's not a very polished GUI, but I'm still quite happy that these pieces have come together.

There are some interesting things going on under the hood, some of which I'll talk about below. Basically, I wanted to know whether this approach is viable. I'm now convinced it is.

I explored some other alternatives, including compiling the Rust code to wasm and using Web tech to build the UI, but ultimately concluded I didn't want to do that.

## A new synth engine

I'm working toward a new modular synthesizer similar to [Max/MSP](https://en.wikipedia.org/wiki/Max_(software)), and also a bit like [VCV Rack](https://vcvrack.com/).

Why a new one? Existing patching languages like Max are a little intimidating; mine is designed to be easier to learn. I want a stronger focus on visualization and highly responsive feedback. Lastly, I'm excited by the idea of pushing performance to the limit.

A central tenet of the engine is that the real-time audio rendering thread is rigorously non-blocking. As Ross Bencina has eloquently written, ["time waits for nothing"](http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing). To avoid glitches, the audio thread can't block on a mutex (which may be held by a lower priority thread, thus [priority inversion](https://en.wikipedia.org/wiki/Priority_inversion)), access the filesystem, or do IO. All that makes sense, but doesn't sound too restrictive. It also can't allocate, because standard allocators internally have mutexes, and quite nondeterministic time behavior. That's hugely restrictive.

At the same time, I want the behavior to be dynamic. I want to be able to patch the processing graph in real time, without glitches. I want voice allocation to be fully dynamic. These seem like perhaps irreconcilable wishes.

To that end, I've worked out a highly customized lock-free queue, which I think gets me everything I want. The main thread can be very dynamic and allocate all it wants, while the real-time thread renders the audio using objects allocated by the main thread, and then when it's done with those objects (for example, when they're deleted from the graph), sends them back on a return channel, where the main thread will drop them at its leisure. The code seems to work.

I hope to blog about this more, and also see below.

## Audio infrastructure

I've started poking around pieces of audio infrastructure in the Rust ecosystem. There's some pretty good stuff, but I think a lot of scope to make it better. For example, the vst crate is not [thread safe](https://github.com/rust-dsp/rust-vst/issues/49), and I'm participating in the discussion of how to make that better.

Similarly, from my initial experimentation, [cpal](https://github.com/tomaka/cpal) seems to be pretty good on macOS, but I have concerns about performance on Windows, as it seems to run the audio callback on the main thread, use mutexes rather than lock free queues, and [doesn't do exclusive mode](https://github.com/tomaka/cpal/issues/106). My inclination is to dig in and try to bring cpal up to what I want, which I think will benefit the Rust ecosystem as a whole. Maintainers of those crates, be prepared for me to be quite annoying in the coming weeks.

## GUI and porting

For the GUI, I'm continuing the "data oriented" approach I [wrote](/personal/2018/05/08/ecs-ui.html) and [spoke](https://www.youtube.com/watch?v=4YTfxresvS8) about a few months ago. So far it's feeling good.

However, for now it's Windows-only, and I'm very interested in porting to other platforms. Rust itself works fine on pretty much every target I care about, so the main tricky dependency is 2D graphics. Right now, I'm using Direct2D (and DirectWrite for text) pretty much directly (through wrapper crates written by [Connie Hilarides](https://github.com/Connicpu)).

There are a number of options for making this code portable.

* Continuing to use the Direct2D API, but with portable implementations ([Wine](https://www.winehq.org/) has done lots of this).

* Adopting an existing cross-platform graphics library. [Skia](https://skia.org/) and [Cairo](https://cairographics.org/) are possibilities, and [WebRender](https://github.com/servo/webrender) is also promising.

* Creating a cross-platform 2D graphics API layer with multiple back-ends. This feels closest to the Rust Way, and is similar to what [gfx-rs](https://github.com/gfx-rs/gfx) is doing for 3D graphics.

Of course, one of the biggest challenges in 2D graphics is fonts and text, and there are choices there as well. For now, it's very appealing to just use the system text APIs, because on Windows at least, they work well and are mature.

This is an area where I could possibly use some help, though it won't be on my critical path for a while, as I'm happy doing the prototype Windows-first. People who are interested in collaborating, let's talk.

## See you in November

Though I'm posting my synthesizer code to public Github, I'm largely in stealth mode for now. I plan to speak at the November SF Rust meetup, at which I hope to present a more polished version of the synthesizer, and also talk in much more detail about the lock-free work and how it can support truly high performance audio synthesis in Rust.
