---
layout: post
title:  "Skribo progress update"
date:   2019-04-26 15:05:42 -0700
categories: [rust, skribo, text]
---
I'm wrapping up my contract from Mozilla Research on Skribo. I was hoping to be farther along. Even so, I've made good progress; mostly the problems where harder than I anticipated. But I think I've moved the ball quite a bit down the field, and I'll talk about the current state of play here.

I'm moving my main focus on to other things (about which more soon), so this leaves the skribo work very much in an unfinished state. I think it is interesting to a lot of people, so the major point of this blog post is a call to action for community involvement. If you're interested in working on this, please get in touch.

For some more of the nitty-gritty details, I've made a [draft design document], which is unfinished but likely useful. Feel free to comment on that PR asking for more clarification if it hasn't been merged yet. (Or file issues if it has!)

## Paragraph layout

One of the missing pieces in the puzzle is paragraph layout, specifically line breaking. Paragraph layout also has to work with rich text. At the skribo level, a layout is *one* style, including one font (though this may be rendered with different fonts due to fallback). If a paragraph uses multiple styles, then that must translate to multiple skribo layouts.

Doing the [BiDi algorithm] is also in scope for paragraph layout.

### Greedy layout

To a first approximation, greedy layout works like this:

* Start with the first word of a line.

* Try adding another word. If it fits, accept it.

* When the line is filled, start a new line with the overflow word.

In the simple case, the width of the line is the sum of the widths of the words, plus the total width of the spaces. However, there are a number of reasons it might not be so simple.

* The space character might participate in shaping.

* The line breaks might not be indicated by spaces. This is most common in scripts such as Korean and Thai, but even in English breaks can happen between punctuation.

* Maybe we're trying to do hyphenation.

In all three cases, the breaks can interact with shaping. In pathological cases, the width of the line can diverge wildly from just adding up the widths of the substrings. In such cases, the correct answer is defined as the width of the substring making up the line.

Thus, to do this properly in the general case, you have to do shaping of longer and longer substrings until the line overflows. If shaping is expensive (it is), this is really inefficient. It's also quadratic in line length, which is not good. In the common case (where space does not participate in shaping), the same words get laid out over and over.

Android deals with this in two ways. First, it makes the hard assumption that space does not participate in shaping. Therefore, when there are spaces, it can always calculate a word at a time. Even then, though, there's multiple re-layout, once to calculate the line breaks, and again when actually drawing the text. Android relies very heavily on a cache for this.

The cache works pretty well, but relies on some assumptions. In particular, it doesn't work well for scripts like Thai where most word breaks are not indicated by space. Further, the cache itself is not free - it takes memory, the cost of hashing the cache keys is nontrivial, and then there needs to be locking for concurrent access.

## Layout sessions

Inspired largely by more recent work on Chromium layout, we reconsidered the entire approach to retaining layout state in a global cache, with a focus on resuing work during layout (and rendering) of a single paragraph. The idea is a [`LayoutSession`], which retains that state as long as the client needs. In a single session, the client can measure substrings for line breaking, query cursor positions, and iterate glyphs for rendering, ideally while doing very little additional work or allocation.

I was thinking that a fast-case query for substring width can be done in O(log n) time, where n is the size of the full string. Basically, you binary search for the endpoints of the string, and take the difference of the cumulative width to each point. But, in the unlikely case that the binary search takes more than a trivial amount of the total time, it's possible to improve asymptotic complexity even further, likely at the cost of more memory.

The key is the "unsafe to break" flag, provided by HarfBuzz for this purpose. If this flag is unset at both endpoints, then the advance is valid. Otherwise, then likely some re-layout is needed. The session retains additional information (such as the result of itemization) to help reduce the overhead on re-layout.

This approach *doesn't* depend on spaces not participating in shaping, though of course re-layout is more likely in those cases.

It's worth looking into how things can be improved further in the re-layout case – maybe only a small substring really needs re-layout. For example, in the simple case of adding a hyphen, maybe it's enough to add the width of the hyphen and the result of a kern pair lookup between the last glyph and the hyphen. But the logic for determining that is not simple. The idea is to support such optimizations in the future.

A special case that's probably worth optimizing is realizing that the space does not participate in shaping. For this, it's possible to get a list of glyphs that did participate in shaping; see [skribo#4] for more discussion. I do think this is a refinement, though. The unsafe-to-break is much more important because it should make a big difference for a lot of common cases.

## Abstraction

One thing I'm backing away from is the idea of making skribo an abstraction over platform text capabilities. It's not clear to me there's a strong need for it, and I'm getting the sense it can increase overall complexity.

The place where I think abstraction is most useful is at the higher level, where there is strong value in being able to access [DirectWrite] on Windows and [CoreText] on macOS, for smaller codebase and better consistency with the rest of the platform. Then skribo-based layout simply becomes one of the alternatives. To this end, though, it might make sense to split out some of the types, in particular representations of text style, so that the higher level abstraction can use the types without having to pull in all of skribo and its dependencies.

## Other work

I spent some time on lower-level libraries. One of those was [fluent-locale-rs], working both on [performance](https://github.com/projectfluent/fluent-locale-rs/pull/8) (I was worried about the cost of locale representation, because a list of locales is part of text style), and also an implementation of ["likely subtags"](https://github.com/projectfluent/fluent-locale-rs/pull/11). Both of those PR's are still in review.

I needed to supply Unicode functions for HarfBuzz to consume, and found that existing Rust Unicode didn't quite do what I needed. As part of my research, I looked deeply into [unicode-normalization], a very widely used crate, and found that it used huge `match` statements for many lookups, relying on LLVM's optimizer. The result is very fast code, but at the expense of big binaries and slow compiles (about 5-6s). A simpler technique like binary search or even the [phf] crate would have been a big slowdown. I ended up hand-rolling a minimal perfect hash function optimized for extreme speed; the hash function has to be just barely good enough to allow successful generation of the minimal hashing. This did get landed as [unicode-normalization#37].

The perfect hashing work probably deserves its own writeup, as I think it has value in other contexts; certainly I see phf pop up a lot in dependencies when compiling Rust projects.

This work was perhaps not on the critical path for skribo (it would certainly have been possible to prototype using slower lookups), but I'm still glad I spent time on these pieces, as they are needed for really high performance text layout, and also benefit the rest of the Rust ecosystem.

## Conclusion

I would have loved to get skribo into shape where other people could start using it. However, the water was deeper than it appeared from the shore. Even so, I've made good progress, and am hopeful that between community contribution and finding time in the future to get back to it, we will get good text layout in Rust. If you're interested in being part of that community, please get in touch, as I'd be quite keen to mentor others and guide the work.

And again, thanks to Mozilla Research for sponsoring the work.

[skribo]: https://github.com/linebender/skribo
[skribo#4]: https://github.com/linebender/skribo/issues/4
[BiDi algorithm]: https://www.w3.org/International/articles/inline-bidi-markup/uba-basics
[unicode-normalization]: https://github.com/unicode-rs/unicode-normalization
[unicode-normalization#37]: https://github.com/unicode-rs/unicode-normalization/pull/37
[phf]: https://crates.io/crates/phf
[minimal perfect hashing]: http://stevehanov.ca/blog/?id=119
[fluent-locale-rs]: https://github.com/projectfluent/fluent-locale-rs
[draft design document]: https://github.com/linebender/skribo/pull/13
[DirectWrite]: https://docs.microsoft.com/en-us/windows/desktop/directwrite/direct-write-portal
[CoreText]: https://developer.apple.com/documentation/coretext
[`LayoutSession`]: https://github.com/linebender/skribo/pull/11
