---
layout: post
title:  "Small updates"
date:   2018-11-04 07:42:42 -0700
categories: [personal]
---
There's been a lot going on, and I've wanted to focus on coding recently, so the technically deep posts that I've been meaning to write remain in the pipeline. In the meantime, here are some updates on various things.

## Zulip

I've been looking for a better tool for open source community interactions. We've mostly been using IRC for xi-editor, but it's not friendly to newcomers, and a pretty poor experience if you don't use a bouncer (I use [irccloud.com](https://irccloud.com) and pay for it out of my pocket). I'm also very critical of most social media style tools, which optimize for engagement rather than quality of interaction. Evan Czaplicki talks about those problems extremely eloquently in his talk, [The hard parts of open source].

I thought a bit about the question, "what tool today is the best fit for fostering an open source community?" After some consideration, I settled on Zulip. It works really well for [Recurse Center], has linkifiers and bot integrations that make it pleasant, and I like the way it's easy to catch up on things quickly if you're not always-on.

Of course, what really makes a community is the people and the choices made about how to interact, rather than the tool. If you're working on xi-editor or projects (even somewhat loosely) related, I hope you'll find [xi.zulipchat.com] a welcoming and productive space. So far it seems to be working great.

## Druid

The UI toolkit brewing inside [xi-win] is turning out nicely, and I think it has promise. I decided to split it out and give it a new identity, so behold [druid]. This is not a formal release yet, it's still early days. Among other things, it needs a logo and nicely polished screenshots.

A few months ago, I made a patch to xi-win proper to use the toolkit, but didn't commit it because it had some regressions (file save and mouse scrolling). I actually managed to lose the patch in the repo move, which was certainly frustrating. [Hilmar Gústafsson] kindly offered to reconstruct it, and working with him, we've done that and gone beyond.

The biggest single problem with druid is that it's Windows-only. I have hopes we'll fix that before long.

## Synthesizer

I haven't had _quite_ as much time to focus on the synthesizer as I would have liked. I've been working more on visuals and UI than the sound engine.

This is a very rough draft of what the the patcher interface looks like:

![Screenshot of patcher](/assets/patcher.png)

There's a bunch I want to change (the little control squares need to become knobs, icons on the chips), but this feels like it's maybe halfway toward the vision I have for it.

I also implemented an analog oscilloscope emulation:

{% include youtubePlayer.html id="atbIvpSUUt0" %}

I'm really happy with the way this looks (the youtube upload is degraded a bit from the way I made the screenshot, and the actual version will be sharper, but it contributes in a way to the retro feel).

I'll be talking about all this at the [SF Rust Meetup] on the 13th. I want to get all these pieces together for the demo, and of course I need to start seriously working on the slides.

## Patreon

How can open source work be sustainable? I'm struggling with this question. Obviously, being at a big Internet company is one solution, but I wanted more autonomy to choose what to work on. One _possible_ solution is asking for donations. I set up a [Patreon page](https://www.patreon.com/raphlinus), more to see how I feel about it than to actually expect to bring in a lot of revenue.

And I'm still not sure how I feel about it. The process of setting it up was interesting; shortly after, I realized that it is parallel in many ways to performance reviews at companies like Google. You're marketing yourself, making the case that your work is worthwhile and that people should fund it. Of course, there are differences, such as granularity of dollar amounts, and the ability to set your own tiers as opposed to working from a given ladder and rubrics, but I was struck by the similarities.

Also, I'm not sure the description I wrote is most accurate in capturing the value I expect to create. Yes, I hope the software artifacts are useful, but I also feel that a lot of my time and energy is going into mentoring a subcommunity, which is basically a form of teaching (with, I think, particular advantages over traditional academic teaching roles, something I hope to blog about more). This blog also takes effort, and from the responses I've been getting, is something people appreciate.

I expect to use the Patreon more as signal, what kinds of things I work on that are valued enough by the community that people are willing to chip in a little money. In any case, if you want to support my work and encourage me to continue focus more on open source (as opposed to the game, which will be proprietary), my Patreon page is the way to do that.

## Vote

I'm volunteering as a pollworker on Election Day. If you are eligible to vote, please do.

[The hard parts of open source]: https://www.thestrangeloop.com/2018/the-hard-parts-of-open-source.html
[Recurse Center]: https://recurse.com
[xi.zulipchat.com]: https://xi.zulipchat.com
[druid]: https://github.com/xi-editor/druid
[xi-win]: https://github.com/xi-editor/xi-win
[Hilmar Gústafsson]: https://github.com/LiHRaM
[SF Rust Meetup]: https://www.meetup.com/Rust-Bay-Area/events/255058428/
