---
layout: post
title:  "What I’m working on at Recurse Center"
date:   2017-10-12 11:59:03 -0400
categories: personal
---
I’m almost 3/4 of the way through my batch at [Recurse Center](https://recurse.com/).
I’ve been working on a bunch of things, and thought it might be useful to go over
them in some detail.

## Ghilbert

I came here with the goal of re-launching [Ghilbert](http://ghilbert.org). This is
a very ambitious project, basically a new language for representing formal proofs.
I got quite a bit done on it, but my motivation has not been consistently strong.

![Ghilbert screenshot]({{ site.url }}/assets/ghilbert_screenshot.png)

One of the challenges (which I’m realizing is a bad sign), is that I haven’t been
able to articulate the goals of the project very well, especially who it’s for.
I made an attempt at an [essay](http://ghilbert.org/why.html), but am not very
satisfied by it. If this is going to be a viable replacement for a first course
in formal logic, then it’s also going to need the equivalent of a textbook,
written specifically to be accessible to a broader audience. That’s a huge time
investment, and being able to leverage theorems translated from
[Metamath](http://us.metamath.org/) only helps with a small part.

Ultimately, I have decided to put this project on the back burner. I still think
the ideas are good, but it feels like to really accomplish the goals will take a
year or more of full-time work. Some of what I want to do, in particular redesigning
the module system so that theorems can be written independent of foundational
axioms, requires work at the cutting edge of language design (I think dependent
pair types are promising, but the details are tricky at best).

## Xi syntax highlighting performance

While I haven’t made a goal of focusing on [xi](http://github.com/google/xi-editor/)
during my batch, I did start trying to use it as my day-to-day editor. I quickly
found that performance was totally unacceptable because of the batch highlighting.
That’s more than a little ironic, because performance is a stated goal of the
project.

Implementing [rope science
11](https://github.com/google/xi-editor/blob/master/doc/rope_science/rope_science_11.md)
was a juicy and fun project, and it improved performance tremendously, back into
usable territory. There’s lots more to be done, but just this was an encouraging
step.

### Cache visualization

The core of the algorithm is a cache eviction strategy designed to balanced
local access patterns with keeping “gaps” reasonably small (all this is to
keep the data structures small and nimble when highlighting large documents).
I gave a little
[talk](https://docs.google.com/presentation/d/1enR5VYtZoQtxJCjq2h8oeUwGYmyheBDLhi6gaiOr8Lg/edit?usp=sharing)
and, preparing for that, realized that _showing_ the cache policy interactively was
the best way to communicate that. The visualization currently lives as a
[pull request](https://github.com/google/xi-editor/pull/403) for the xi
documentation directory.

I hope to polish up the talk and the visualization, and publish to a wider
audience, as I think it’s a great showcase for xi.

## Snowflake

A long time ago I was motivated to create a “visual hash” that would both be
visually appealing, and also useful to distinguish two different hashes (so that
it would be hard for an attacker to create a visual collision). I dusted that
off, reimplemented in Javascript with SVG rendering, and posted it as as a
[small interactive webpage](http://levien.com/snowflake.html).

[![Rainbow snowflake]({{ site.url }}/assets/snowflake.png)](http://levien.com/snowflake.html)

I also made some progress in doing a more detailed
[explanation](http://levien.com/snowflake-explain.html), intending it to
have an interactive visualization showing the construction step-by-step, but
didn’t follow through. I can imagine coming back to this, though.

I think getting rid of the pseudo-randomness, and just doing fill breadth-first,
would be an improvement (discussions with [Marcus Klass de Vries](https://marcusklaas.nl/)
helped illuminate this question).

## Apple 2 bitmap text rendering

The Recurse Center has an Apple //e in the space, and the second I laid eyes on
it I thought there would be a pretty good chance I’d code something for it. The
[KIM-1](https://en.wikipedia.org/wiki/KIM-1), a primitive 6502-based computer,
was the very first computer I ever programmed, so I was drawn to coding something
in 6502 assembler.

Guided by the thought of maybe putting an emulator on my webpage, I wrote some
fairly simple code to render a proportionally spaced bitmap font. It worked:

![Apple 2 text screenshot]({{ site.url }}/assets/apple2_text.jpg)

The code is not yet published.

Along the way I made some improvements to
[appletoo](https://github.com/nicholasbs/appletoo), which was written at Recurse
a few years ago. This is a good emulator to put on a homepage because it's pretty
simple and can be readily adapted as needed.

## Apple 2 cassette loading

Getting code moved over to the Apple hardware is a challenge. The best supported
(but slow) technique is to use the cassette interface, which rips along at
around 1300 bits per second. There is an open source project called
[c2t](https://github.com/datajerk/c2t) which increases this to around 8k with
a “hi-fi” mode. I looked at the details of the encoding and analog path and came
to the conclusion that much higher data rates are possible. My original goal
was 40kbps.

I used the “retro and physical computing” weekend to finish the project (aided
greatly by an oscilloscope purchased for the weekend), and ultimately achieved
23k. I haven’t published this code yet either, but a bit of a writeup exists as
an [issue](https://github.com/datajerk/c2t/issues/4) on c2t.

## Crosswords

Keiran King has been working on [Phil](https://github.com/keiranking/Phil), a tool
to help construct crosswords, as the main project of his batch. One goal is to
automatically fill the remaining grid with partial words. As it turns out, there
is some [literature](http://abotea.rsise.anu.edu.au/data/cp08.pdf) on this, and
an extremely promising approach is to use a SAT solver.

I’ll be writing lots more about this, but this is now my main project, and I’m
very excited about it. We’ll run the solver in the browser (using wasm and
emscripten), and are collaborating on refining the user experience. I’ve also
figured out ways to prune the search space so the solver can run much faster.

Modern SAT solvers (such as [glucose](http://www.labri.fr/perso/lsimon/glucose/))
are almost magical in their ability to find solutions even in large problems
(a typical crossword translates to hundreds of thousands of variables and millions
of clauses).

There’s also a lot of interesting work in refining the wordlist, as this is
key to quality results. We’re starting from Saul Pwanson’s [xd](http://xd.saul.pw/)
corpus, and thinking of a bunch of ways to filter out low-scoring words as well as
augment it with fresh new ones. A promising approach to the latter is
[collocations](http://matpalm.com/blog/2011/10/22/collocations_1/), and as we speak
I’m replicating those results.

## Smaller things

I pushed my experiment running the [FM synthesizer in a
browser](https://github.com/google/music-synthesizer-for-android/tree/webaudio)
(using emscripten).
I mentored some work towards the rewrite of pulldown-cmark to a
[better algorithm](https://github.com/google/pulldown-cmark/issues/41). I’ve
also been doing minor maintenance on xi and
[fancy-regex](https://github.com/google/fancy-regex).

## Thanks

Thanks to the Recurse Center for providing the space, and for the many talented and
kind Recursers who have shared stimulating conversation, encouragement, and pairing.
