---
layout: post
title:  "Moving from Rust to C++"
date:   2023-04-01 06:00:42 -0700
categories: [rust]
---
**Note to readers: this is an April Fools post, intended to satirize bad arguments made against Rust. I have mixed feelings about it now, as I think people who understood the context appreciated the humor, but some people were confused. That's partly because it's written in a very persuasive style, and I mixed in some good points. For a *good* discussion of C++ safety in particular, see JF Bastien's talk [Safety and Security: The Future of C++](https://www.youtube.com/watch?v=Gh79wcGJdTg).**

I’ve been involved in Rust and the Rust community for many years now. Much of my work has been related to creating infrastructure for building GUI toolkits in Rust. However, I have found my frustrations with the language growing, and pine for the stable, mature foundation provided by C++.

## Choice of build systems

One of the more limiting aspects of the Rust ecosystem is the near-monoculture of the Cargo build system and package manager. While other build systems are used to some extent (including [Bazel][Scaling Rust builds with Bazel] when integrating into larger polyglot projects), they are not well supported by tools.

In C++, by contrast, there are many choices of build systems, allowing each developer to pick the one best suited for their needs. A very common choice is CMake, but there’s also Meson, Blaze and its variants as well, and of course it’s always possible to fall back on Autotools and make. The latter is especially useful if we want to compile Unix systems such as AIX and DEC OSF/1 AXP. If none of those suffice, there are plenty of other choices including SCons, and no doubt new ones will be created on a regular basis.

We haven’t firmly decided on a build system for the Linebender projects yet, but most likely it will be CMake with plans to evaluate other systems and migrate, perhaps Meson.

## Safety

Probably the most controversial aspect of this change is giving up the safety guarantees of the Rust language. However, I do not think this will be a big problem in practice, for three reasons.

First, I consider myself a good enough programmer that I can avoid writing code with safety problems. Sure, I’ve been responsible for some CVEs (including [font parsing code in Android]), but I’ve learned from that experience, and am confident I can avoid such mistakes in the future.

Second, I think the dangers from memory safety problems are overblown. The Linebender projects are primarily focused on 2D graphics, partly games and partly components for creating GUI applications. If a GUI program crashes, it’s not that big a deal. In the case that the bug is due to a library we use as a dependency, our customers will understand that it’s not our fault. Memory safety bugs are not fundamentally different than logic errors and other bugs, and we’ll just fix those as they surface.

Third, the C++ language is evolving to become safer. We can use modern C++ techniques to avoid many of the dangers of raw pointers (though string views can outlive their backing strings, and closures as used in UI can have very interesting lifetime patterns). The [C++ core guidelines] are useful, even if they’re honored in the breach. And, as discussed in the next section, the language itself can be expected to improve.

## Language evolution

One notable characteristic of C++ is the rapid adoption of new features, these days with a new version every 3 years. C++20 brings us modules, an innovative new feature, and one I’m looking forward to actually being implemented fairly soon. Looking forward, C++26 will likely have stackful coroutines, the ability to embed binary file contents to initialize arrays, a safe range-based for loop, and many other goodies.

By comparison, the pace of innovation in Rust has become more sedate. It used to be quite rapid, and async has been a major effort, but recently landed features such as generic associated types have more of the flavor of making combinations of existing features work as expected rather than bringing any fundamentally new capabilities; the forthcoming “type alias impl trait” is similar. Here, it is clear that Rust is held back by its commitment to not break existing code (enacted by the edition mechanism), while C++ is free to implement backwards compatibility breaking changes in each new version.

C++ has even more exciting changes to look forward to, including potentially a new syntax as proposed by Herb Sutter [cppfront], and even the new C++ compatible Carbon language.

Fortunately, we have excellent leadership in the C++ community. Stroustrup's [paper on safety][Stroustrup on safety] is a remarkably wise and perceptive document, showing a deep understanding of the problems C++ faces, and presenting a compelling roadmap into the future.

## Community

I’ll close with some thoughts on community. I try not to spend a lot of time on social media, but I hear that the Rust community can be extremely imperious, constantly demanding that projects be rewritten in Rust, and denigrating all other programming languages. I will be glad to be rid of that, and am sure the C++ community will be much nicer and friendlier. In particular, the C++ subreddit is well known for their [sense of humor].

The Rust community is also known for its code of conduct and other policies promoting diversity. I firmly believe that programming languages should be free of politics, focused strictly on the technology itself. In computer science, we have moved beyond prejudice and discrimination. I understand how some identity groups may desire more representation, but I feel it is not necessary.

## Conclusion

Rust was a good experiment, and there are aspects I will look back on fondly, but it is time to take the Linebender projects to a mature, production-ready language. I look forward to productive collaboration with others in the C++ community.

We’re looking for people to help with the C++ rewrite. If you’re interested, please join our [Zulip instance].

Discuss on [Hacker News](https://news.ycombinator.com/item?id=35400047) and [/r/rust](https://old.reddit.com/r/rust/comments/128m5wn/moving_from_rust_to_c/).

[Stroustrup on safety]: https://www.open-std.org/jtc1/sc22/wg21/docs/papers/2023/p2739r0.pdf
[Scaling Rust builds with Bazel]: https://mmapped.blog/posts/17-scaling-rust-builds-with-bazel.html
[font parsing code in Android]: https://nvd.nist.gov/vuln/detail/CVE-2016-2414
[C++ core guidelines]: https://isocpp.github.io/CppCoreGuidelines/CppCoreGuidelines
[Zulip instance]: https://xi.zulipchat.com
[cppfront]: https://github.com/hsutter/cppfront
[sense of humor]: https://www.reddit.com/r/cpp/comments/128xpps/comment/jekwww0
