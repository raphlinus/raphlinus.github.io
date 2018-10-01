---
layout: post
title:  "Announcing the xi-editor github organization"
date:   2018-10-01 14:01:03 -0700
categories: [xi]
---
I’m pleased to announce that [xi-editor](https://github.com/xi-editor) is now hosted in its own github organization, moved from being a Google project. The main practical upshot is that a Google Contributor License Agreement is no longer needed. It also signals the community-focused direction of the project.

I’ve already noticed [increased activity](https://github.com/xi-editor/xi-editor/graphs/contributors) from contributors since the change, and am happy to see that. I believe xi-editor is on the cusp of being a self-sustaining community-driven open source project; certainly it feels that way looking at the rate of code contributions.

Since my main focus these days is creating a music synthesis game, I’ll be up-front, I don’t have the bandwidth to do fine-grained code review of every PR, and to guide each detail of architectural decisions, along the lines of the [rope science series](http://xi-editor.github.io/xi-editor/docs/rope_science_00.html). I plan on dedicating one day a week to such matters. Fortunately, I see a path to the project being successful in spite of my somewhat limited bandwidth. For a while, [Colin Rofls](https://github.com/cmyr) has been doing the lion’s share of review, triage, and community interaction. I’m deeply grateful for his work. Also, I invite contributors to help share the load, reviewing each other’s code, discussing desired features and implementation strategies for them, and then assigning issues to me when they need my review. I’m hopeful this will grow a scalable and sustainable structure for the community. To this end the project also has a new set of [contributor guidelines](https://github.com/xi-editor/xi-editor/blob/master/CONTRIBUTING.md), which explain in more detail what our process will be going forward.

A major focus for xi has been research and learning. For the near and medium term, I think it will be a more appealing project for people interested in learning how text editors are made and getting deeper into Rust programming techniques, as opposed to a polished out-of-the-box editing experience. Some of the biggest challenges are in packaging – keeping the front-end, core, and suite of plugins coherent and updated. This is one reason why there are no prebuilts, etc., even though we’ve seen a fair amount of demand. To be honest, I don’t have a very clear vision how to solve those problems, and am hoping the community can come together around them. Even so, I plan on using xi-editor as much as possible as my daily driver.

Some will be curious about the state of the Fuchsia front-end. I still think it has quite a bit of promise, but it’s still early days for Fuchsia and the platform is not really ready for end-user software or self-hosted development. I’m hopeful it will get there in time and feel that xi-editor will be a great fit for it at that time. I look forward to continuing to collaborate with the Fuchsia team and others within Google.

I’d like to thank Google for supporting xi-editor, to their open source team for the administrative support, and of course to all the contributors over the 2.5 years so far.

It’s been quite a journey so far, and I’m excited for the future!

