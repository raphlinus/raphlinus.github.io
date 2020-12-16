---
layout: post
title:  "An Old New Adventure"
date:   2020-12-16 09:28:42 -0700
categories: [personal]
---
After two and a half years of being independent, I am returning to Google.

The time off was really valuable. I was still feeling residual effects from burnout on the Android team in late 2015, and also drained by family and personal things that were happening and needed more time and energy. I got that, and return recharged and with some insights that I hope will be useful. I'll touch on a few of those in this post. Each could probably be its own blog post, but today I want to briefly note the event.

I am now a research software engineer on the Google Fonts team, working on a number of topics in font technology, including font design tools, GPU-accelerated font rendering, and evolution of font file formats to be more efficient and capable.

## On open source sustainability

Much has been written on open source sustainability, notably [Nadia Eghbal]'s *Working in Public.* I won't speak to open source more broadly (except to note how impressed I am with Blender and Krita), but for the specific task of building an ecosystem for a library, I think there is one model that actually works: being hired by a company that depends on that ecosystem.

To some extent, that's an indictment of our capitalist system. In an ideal universe, there would be strong institutions dedicated to the public interest where open source developers could develop, researchers could research, and spend an absolute minimum of time and energy hustling for support. For software, in any case, universities are not that (as demonstrated by [William Stein's experience with Cocalc at UCSD][stein on leaving ucsd]), otherwise I'd be quite tempted. In the actual world, working for a company like Google is about as close as you can come.

I remain skeptical of patronage-style platforms such as Patreon or Github Sponsors. I think it's possible to make them work, but only for a small number of fortunate people, and even then, the incentives for creating maximum value aren't that well aligned with the incentive structure of hustling on social media.

So me (re-)joining Google full time is basically a statement of confidence in this model of being employed to work on open source. Other models can work, and people should definitely find what works for them, but particularly for the projects I'm interested in, it makes sense.

## On Rust

I continue to love Rust, and believe it offers a stronger foundation for building software. I feel like I started my Rust journey in the early '90s, when I was working on retrofitting [static memory management] to ML, using explicit lifetime regions.

Rust adoption is trending up, including at Google. The language is in good shape, but the library ecosystem is still fairly immature, missing a number of critical pieces. Building up that ecosystem is an incredibly rewarding project.

I am particularly excited about Rust for font technology and infrastructure. Today, Python rules on the font design and production side, partly to the connection of typeface designer [Just van Rossum] being Guido's brother. The flexibility and expressiveness of Python makes it a good fit, but we've also gotten to a place where the *production* of fonts is done in Python, and the *consumption* is in C++.

Rust lets us build reliable, performant code that can also be deployed in production, and can be the basis of fluidly interactive UI tools. I'm not the only one who sees this potential; YesLogic is building their next-generation font shaper [Allsorts] in Rust, for many of the same reasons.

The Google Fonts team has been interested in adopting more Rust for a while, and part of my role is to facilitate that. I'm really looking forward to it.

## On research

I have rebranded myself somewhat as a researcher, but that doesn't *quite* capture the whole story either. I have always loved research, and that love sustained the energy to complete my [PhD], but I also love building real things, and actually feel that many of these practical problems are more interesting than many of the abstract topics fashionable in academia. Just as much as writing papers and so on, I'm trying to build open source software and community around that. There isn't really a word for this role, but even without such a word I'm trying to consciously create it for myself, and am grateful that Google is allowing me to try.

## On the work

This is the most exciting part for me. I have a very long-term interest in 2D graphics, font technology, and UI, and have been doing a bunch of interesting things on all these fronts. I expect to spend most of my time continuing to advance research on all these frontiers.

The scope of these projects is large, and more ambitious than one person could really do. That's one reason I've been consciously developing an open source community around them. That will continue.

Most of the day-to-day work on [Druid] and [Runebender] will be done by Colin Rofls, though I very much enjoy getting my elbows in the code too and will be doing some of that.

A major focus will be building out the [piet-gpu vision]. I believe a high-performance 2D rendering engine will be a great thing for the Rust ecosystem and with potential for large impact. It feels like good research; whether it goes into production at scale or not, I expect the things we learn from doing it will help inform the next generation of UI technology. That's equally true for research into fundamental UI principles, for example the [Crochet] architecture for Druid.

There are also really exciting advances in [spline] technology in the pipeline. I think these have the potential to be a more appealing and productive basis for drawing fonts than cubic Béziers. The next big step is to validate whether they actually work as well as I'm hoping. That involves polishing the UX and integrating them into Runebender. If that turns out really well, a longer term (but more speculative) aspiration is to get them into a font format, where they could reduce binary size while increasing quality. It's obvious the Google Fonts team is the best home for this work.

I have a lot of work in front of me, but am more excited than ever. On to an old new adventure, and may 2021 be a time of healing and renewed energy for all.

[stein on leaving ucsd]: https://blog.cocalc.com/2019/04/12/should-i-resign-from-my-full-professor-job-to-work-fulltime-on-cocalc.html
[Nadia Eghbal]: https://nadiaeghbal.com/
[Druid]: https://github.com/linebender/druid
[Runebender]: https://github.com/linebender/runebender
[piet-gpu vision]: https://github.com/linebender/piet-gpu/blob/master/doc/vision.md
[Crochet]: https://raphlinus.github.io/rust/druid/2020/09/25/principled-reactive-ui.html
[spline]: https://github.com/linebender/spline
[static memory management]: https://theory.stanford.edu/~aiken/publications/papers/pldi95.pdf
[PhD]: https://levien.com/phd/phd.html
[Just van Rossum]: https://medium.com/type-thursday/learning-python-makes-you-a-better-designer-an-interview-with-just-van-rossum-8d4758c192d8
[Allsorts]: https://github.com/yeslogic/allsorts
