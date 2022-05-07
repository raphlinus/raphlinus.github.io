---
layout: post
title:  "Towards principled reactive UI"
date:   2020-09-25 07:44:42 -0700
categories: [rust, druid]
---
**Update 7 May 2022:** A followup to this post containing significant conceptual advance is [Xilem: an architecture for UI in Rust](https://raphlinus.github.io/rust/gui/2022/05/07/ui-architecture.html). I consider the Crochet experiment to be mostly negative, as there were a number of ergonomic and functional "paper cuts," though several of the research goals were met, at least to some extent.

This is a followup to my post about a year ago, [Towards a unified theory of reactive UI]. It is a deeper exploration of the question: "what is the best way to express reactive UI in Rust?"

## Introduction

There is an astonishing diversity of "literature" on reactive UI systems. I put "literature" in quotes here, because, with some exceptions, the best primary sources are the code of open source projects. Some of the diversity comes from differences in goals, but some of it is accidental. In many cases, it's simply because the designers didn't have insight at the time about better solutions. My post last year attempted to put some order to this diversity, by discovering common patterns.

I believe it's likely that the answer to "the best way to express reactive UI in Rust" is to be found in the existing literature, at least by combining major themes, if not in a single existing system to copy. It seems unlikely we'll have to invent something completely new. But sorting through it is not easy. It is not the intent of this post to provide a comprehensive review of the literature (though I think such a thing would be interesting), it is to guide inquiry into the most promising avenues. I want to do mining, not stamp collecting. Where is the richest vein of ore?

To focus the inquiry, I will start by listing some goals. While in general these goals all seem like good things, it's important to understand them as tradeoffs. Prioritizing different goals will lead you to different places, and not everyone has the same needs. Take overall system complexity, for example. If you're a trillion dollar corporation, then a complex system is merely a question of allocating resources, and may even be strategically useful as a moat to discourage competition. But if you're an indie game developer trying to integrate basic UI, you have a very different perspective.

Each goal will be presented primarily as a way to introduce design decisions made by existing reactive systems, and filter the ones that seem most promising as sources and inspiration.

Then I will go into deeper into three principles, which I feel are critically important in any reactive UI framework: whether to use "observable objects," how to express the mutation of the render object tree (or trees in general), and the notion of stable identity of nodes in that tree.

Finally, I will introduce [Crochet], a research prototype built for the purpose of exploring these ideas.

## Goals

### Concise expression of application logic

The main point of a reactive UI architecture is so that the app can express its logic clearly and concisely, and the results can drive the rest of the UI stack in a reasonable way.

A central feature of reactive UI is for the app to declaratively express the current state of the view tree. In traditional object-oriented UI, it's more common to specify the initial state (often as a static document, not even code), plus additional logic for state changes. I think the debate is now essentially over, the reactive approach is winning.

SwiftUI has gained considerable attention due to its excellent ergonomics in this regard. But other approaches are also worth studying. In particular, immediate mode GUI ([imgui]) is nearly as declarative, it just achieves it in a very different way (about which more below). And React and its many derivatives are also "good enough." [Svelte] is another example from the JS world that deserves praise, though considerably more difficult to adapt to Rust because of its reliance on a sophisticated compiler.

It's very popular in Rust GUI land to adapt [Elm] patterns; we see clear influence in [relm], [Iced], [vgtk], and others. But I think much of the conciseness and friendliness of Elm comes from the language itself, particularly its facility with higher-order composition. When adapting to a more pragmatic language such as Rust, I consider each subtask of view building and dispatching messages to components as each a half-lens, requiring the writing out of two pieces of logic to integrate a component. For this reason, I find Rust UI code adapted from Elm to be not as clear and concise as possible.

A great resource for comparing the concision of different toolkits is [7GUIs]. We don't have these ported to Crochet yet, except for counter, but plan to. For reference, here's the `run` method for that:

```rust
    fn run(&mut self, cx: &mut Cx) {
        Label::new(format!("current count: {}", self.count)).build(cx);
        if Button::new("Increment").build(cx) {
            self.count += 1;
        }
    }
```

### Actually being incremental

While imgui can express UI concisely, it cheats somewhat by not being incremental. Generally, it makes up for this by being able to repaint the world very quickly (using GPU acceleration), but there are downsides, including power consumption. In the context of a game which is actively using the GPU anyway, it's fine, but is a good reason not to choose imgui outside that context.

The documentation of [Conrod] expresses a goal fairly well: "Conrod aims to adopt the best of both worlds by providing an immediate mode API over a hidden, retained widget state graph." And once you have that, doing efficient incremental updates in the retained widget graph is a solved problem, though the details can be intricate. Unfortunately, I do not believe Conrod delivers on this promise, because app logic does an awful lot of explicit graph construction and juggling of node id's, neither of which you would find in an actual immediate mode API.

On the other side, [Iced], while having many desirable properties, does not satisfy the "actually incremental" goal: while there is caching in the renderer, it builds a full view tree every 16ms (actually twice when there are incoming events). This is fine when the view tree is relatively small, but is a serious problem at scale.

The popularity of "virtual DOM" approaches requires a discussion of diffing, which I feel is (modulo escape hatches for lower level direct tree mutation) a form of half-incrementality. The idea is that it should be cheap to produce a full view tree, then a reconciliation engine computes a minimal diff between that and the old tree, which is then be applied, through DOM mutation or some other means. Because DOM is slow, it's certainly faster than ham-handed direct DOM mutation (which is difficult to make truly minimal), but still creates performance problems as the view tree grows. React programmers should be well familiar with this issue.

When escape hatches are provided, are they a reasonably principled way to achieve lower level access to tree mutation, simply shifting some tracking of state to the component (often a list view or similar collection), or are they a dirty hack to work around fundamental architectural decisions that limit performance? Both are represented in the literature.

### Tab focusing

It seems like a relatively simple feature, but proper implementation of tab focusing requires fairly deep architectural support (or, in the case of Web-based UI, it is punted to the browser). Basically, it requires the toolkit to maintain state of which widget is focused, and to query enough of the entire view tree to determine which is the next in tab focus order. Proponents of imgui have suggested a very hacky partial solution (see [johno on IMGUI]), but I find such things unsatisfying.

Again, Iced is an example of an existing Rust GUI toolkit that is lacking this feature, and I think would require nontrivial architectural work to address, at least in a systematic way that would satisfy similar future needs. Down the line, those needs include accessibility, a serious Achilles heel for imgui-flavored designs in particular.

I believe a proper approach to this problem involves stable identity of widgets, about which much more below.

### Simple types

Now we get into more controversial goals. One that I personally am finding increasingly important is expressing the interface between app logic and UI toolkit using simple types.

Rust in particular invites the use of complex types, largely because it has a rich type system that is capable of expressing many concepts as types. By contrast, it's traditional in object oriented UI to have very loosely coupled dynamic typing; a lot of the values being passed around have a type which is some variant of "any."

A great example of complex types is SwiftUI, in which a component returns a statically-known type implementing the [`View`] protocol. As a consequence, the concrete return type of a component generally encodes its entire view hierarchy, including special conditional and looping combinators; this is explained well in the [Static Types in SwiftUI] blog.

There are advantages to such a scheme, but also serious downsides. Error messages from the compiler get... interesting. And it also has a serious impact on compile times, as (at least in Rust) the compiler has to monomorphize the types before even starting to generate code.

To me, one of the most serious drawbacks to the complex type approach is that scripting languages can't easily play, as the types must be known at compile time.

Again, imgui is an example of an architecture that avoids complex types, by drawing the UI directly rather than constructing an intermediate tree of view objects. But imgui is not the only such; another compelling example to learn from is [Jetpack Compose].

### Simple control flow

It is very tempting to use complex control flow patterns: putting significant logic in callbacks, using higher order composition techniques, or using a compiler to significantly transform the code. Yet, such techniques have downsides.

The first is simply that this complexity leaks out into the app. In current Druid, we use some higher order composition techniques such as lenses. While fairly simple by Haskell standards, and our users with Haskell background tend to like them, a lot of people coming to Druid find them confusing.

The simplest mechanism for composition of UI elements is function composition. This position is well argued in [Jetpack Compose], and the experience of React hooks vs class-based components is further evidence.

Another reason to prefer simple control flow is performance. Not that it's always faster, but that it's easier to reason about and measure. Perhaps one of the more controversial aspects of the Crochet prototype is that it relies on explicit application logic to decide when to skip subtrees. This is a bit of a burden, but also an opportunity for the application to apply its own context, for example status from a stateful database connection. Also, straightforward profiling and tracing should quickly reveal opportunities for more aggressive skipping.

### Overall system complexity

This one is even more subjective, but I think is still important. A UI toolkit is an ambitious task as it is. Doing a full-scale incremental computation engine, of similar scope as [Adapton] or [Incremental], is a serious additional burden. (Fans of Incremental should also be aware of its successor [Bonsai]). What I know of SwiftUI suggests that it has a similar engine under the hood (which shows up as "ViewGraph" or "AttributeGraph" in stack traces), and that's in addition to its integration with the public-facing [Combine].

As a result, though I sometimes hear people suggest adapting SwiftUI to Rust, which on the surface makes sense due to its excellent ergonomics and other advantages, I fundamentally do not think it would work, at least without expending enormous resources.

Again, imgui is an impressive example of how much is possible without such complexity. It does cut many corners, but I think it's possible to explore how to implement a more fully-featured retained widget tree backing an API that truly does capture much of the simplicity of imgui. A promising step in this direction is [Makepad], which has also served as the inspiration for many of my ideas.

My review of [Iced] also found its overall simplicity to be appealing, though I worry whether that would survive architectural rework needed for multiwindow, fully featured tab focusing (much less accessibility), ability to scale to large list views, etc.

## Principles

While the above stated goals illuminate important differences between existing reactive UI systems, I also believe there are principles common to essentially all such systems, though each might add its own spin to how it implements these principles. My ["towards a unified theory"][Towards a unified theory of reactive UI] blog post proposed some of those principles, particularly the model of a pipeline of tree transformations. In this section I will focus on three more, only touched on briefly in the previous post: observable objects vs future-like polling, how trees and tree mutations are represented, and the tricky question of stable node identity in the view tree.

### Observable objects

UI requires expressing dependency relationships. You click a button here, something in the internal state changes, then that's visible in a different widget over there.

The standard object-oriented approach to these dependencies is an "observable object," of which there are many many implementations. While there are lots of variations, generally it involves the object keeping track of some number of subscriptions, each of which boils down to a callback that is invoked when something happens. Probably the most familiar example is onclick listeners and the like in the JavaScript/DOM world.

The observable pattern is so common, it is often taken for granted as a required building block of UI. But I think we should be looking at alternatives, especially in Rust, where the object-oriented underpinnings do not translate well.

In languages with getter/setter notation, the setter method of an observable object calls the callbacks of the currently subscribed listeners, in addition to updating the field of the object. In fact, it is probably one of the main motivations for languages to have this syntax. While Swift notably does have getter/setter, Rust, for better or worse, does not.

The problems with observable boil down to the fact that the callback requires a *lot* of context to know what change should happen in response to the event. In this way, I think it is fundamentally in tension with the goals of reactive UI, though some systems have managed to reconcile the two, often with compiler help to translate fairly straightforward declarative logic into an observable-based implementation: SwiftUI and Svelte come to mind.

I'm not sure there is a good name, or literature I can cite, for alternatives to the observable pattern, but the general principle is for the notification to carry much less context. Rather than "this specific thing changed, update your state in response," the notification says, "something in this area changed, you're going to need to recompute."

A particularly extreme example is imgui, which barely tracks change notifications if at all, and instead assumes the world will be redrawn 60 frames a second. But, while getting rid of the entire mechanism for notification tracking is a huge simplification, it throws the incremental baby out with the bathwater.

The existing [Druid] architecture is another data point proving the existence of efficient, ergonomic change notification without observables. It relies on diffing of trees, using pointer equality to skip parts that haven't changed at all, and Haskell-like lenses to apply this skipping logic to subtrees. However, we have found that the heavy reliance on diffing creates its own problems, depending on how closely the app state fits into the paradigm of immutable (and therefore easily diffed) tree data structures.

I think a promising inspiration is Rust's async infrastructure. [Futures][Future] ultimately solve a similar problem as callback-based observables, but work using a very different mechanism. Basically, the notification is provided to a "waker," which is a kind of callback, but carries a very narrow amount of context regarding what actually changed. In general, it references a *task* which was blocking on the future, and an opaque token identifying what it was blocking on (such as a network connection providing some data). The Rust async architecture then invokes the task from its root, and the task quickly dispatches to the specific future that was waiting, based on its own internal state machine and the token provided through the waker.

The Crochet prototype has a specific implementation of this idea, but there are likely other viable variants. In general I think it is one of the most important architectural decisions to be made for a reactive UI framework.

Integration with Rust's async ecosystem is a major feature for a UI toolkit, and something the existing Druid architecture struggles with. Based on early experimentation with the Crochet prototype (though there is much more to be done), it seems like the task-waking approach will integrate very nicely. The details are beyond the scope of this post, but involve the app logic *conceptually* traversing the view tree from the root, while in practice having the opportunity to efficiently skip subtrees other than the one that contains the waking token, at which point it has exactly the context it needs to advance its state. Getting this to work is one of the major reasons I'm excited about this architectural approach.

### Trees and tree mutation

As argued in the ["unified theory" post][Towards a unified theory of reactive UI], the logic of reactive UI is well-expressed as a series of tree transformations. A typical pipeline consists of the transform from the app state to the view tree (this stage is basically the view part of the "app logic", the other part being response to UI actions), from the view tree to a render object (widget) tree, and from the widget tree to drawing graphics primitives of some kind or other, ideally GPU-friendly display lists.

Aside from custom widgets, most of the rest of the pipeline behind the view tree is the responsibility of the toolkit. In the existing Druid architecture, the app state itself is also expected to conform to tree structure, though I think relaxing this is important for the "concise expression" goal.

This leaves us with a rather critically important piece: expressing the view tree, and, because we want the computation to be incremental, in particular expressing *mutations* of the view tree.

In the JavaScript world, the [DOM] (Document Object Model) is the standard way to express such a tree. To summarize, each node in the tree is actually a graph node, which has references to its children and parent (and immediate siblings), and on top of that implements an observable protocol, where changes get propagated both through a C++ interface to the browser engine, and also to JavaScript-language listners. Ownership of individual nodes is subject to garbage collection, and then there's additional logic for CSS processing. In short, it is *enormously* expensive per-node, and a major source of the performance problems of UI built on Web technology.

Mutation of the DOM is expressed through a well-specified and reasonably ergonomic, if inefficient, interface: a collection of methods like appendChild, removeChild, setAttribute, etc. While there certainly has been a lot of JS written over the years to do these mutations by hand, it's basically a given that a major role (if not the primary role) of a reactive UI framework is to convert changes expressed declaratively by app logic into these tree mutation method calls.

On the other extreme is the HTML serialization of a tree. This is efficient enough that it is the chosen mechanism for sending trees over the net, and parsers can expand it readily enough into DOM, but it has the disadvantage of only really being good for static trees. In theory it might be possible to send a [diff] and apply the corresponding patch, but in practice that would be very fragile, and the only reliable way to generate diffs would be to compare the whole old tree with the whole new one; it's not easy to see how to generate diffs programatically.

I believe strongly we should be looking for alternatives to DOM, especially within Rust native GUI toolkits, as it is both terribly expensive and not idiomatic Rust. The need for garbage collection is a particular source of friction, which is not be the case in more traditional GC'ed languages.

One especially promising alternative is a preorder traversal of the tree, expressed as nodes stored in a sequence data structure. This is not at all a novel idea, it's what [Jetpack Compose] uses. I find it appealing partly because it adapts very neatly and efficiently to Rust, and because we can use the entire, very well developed technology of sequence editing to express tree mutation. Note that Jetpack Compose uses the time-honored "gap buffer" technique to optimize the sequence storage for editing, but obviously there are many choices here; long-time readers will not be surprised to learn that I favor B-Tree (or [RRB-Trees]) for storage augmented with monoids to accelerate navigation of tree structure.

There is another thing to say about tree mutation. The classic imperative style (adopted by JS and DOM, among other systems) is to mutate the tree in place. This style relies on the nodes being observable objects, or some other mechanism such as diffing, to notify downstream pipeline stages of exactly what changed. Given that we're trying to avoid observables, the Crochet prototype takes a different approach. It uses a tree mutation cursor, which holds an *immutable* reference to the original tree, and builds up a tree mutation data structure (basically a variant of the "diff and patch" concept) as it goes along. In the common case where not much has actually changed, the mutation should be a small object, with very little allocation cost.

Then the mutation, expressed as a diff to be applied the tree as a whole, is passed explicitly to downstream stages, which then update the internal widget state as needed in response. In experience so far with the Crochet prototype, consumption of these explicit mutation objects is both low-friction and efficient. A container widget in general recursively passes a subtree mutation to its child, but can skip the recursion altogether if the mutation for the corresponding subtree is empty.

The idea of the app logic producing an explicit tree mutation while holding an immutable reference to the old state of the tree feels very functional, though the app state itself is mutable. Even so, I haven't seen this particular pattern in the functional programming UI literature (hopefully readers can fill me in). My direct inspiration for the idea is the proxy object pattern in [automerge], where capturing precise, efficient mutations is essential for distributed collaboration.

### Stable identity

Another foundational concept in reactive UI is how to express *stable identity* of nodes in the view tree.

If widgets in the UI are essentially stateless, their appearance being determined entirely by data flowing from the app logic, such as the text of a label widget, then stable identity hardly seems a problem worth solving. When producing a diff (tree mutation) from the old state of the view tree to the new, it only matters that this diff is minimal, and then only for efficiency. If you rebuilt the tree every time from scratch, the behavior would be the same, only slower.

The real world is, as is so often the case, considerably messier. Widgets often have internal state, and it is important for good user experience that this state is preserved. A classic example is the cursor position in a text input, but there are lots of other cases: the position of a splitter in a split-pane widget, tab focus state, etc.

Often when I read explanations of reactive UI systems (I'll pick on Elm in particular, as it is otherwise well-explained and principled), I am left unsatisfied, as the question of whether a node in the view tree will have stable identity from run to run of the app logic almost seems left to chance. Certainly in the case of static structure, there is nothing to worry about, but as the UI becomes more dynamic, more can go wrong.

At the DOM level, the solution to the problem of stable node identity is object identity at the JavaScript language level, in other words it is consistent with the [strict equality]() (`===`) operator.

Stable identity of nodes in the view tree is very closely related to memoization state and component-scoped state ([useState] in React). And here, React gives at least a partial answer: the [Rules of Hooks] constrain the app logic in a fairly significant way (a component cannot invoke a hook conditionally or in a loop), in exchange for a guarantee that successive runs of the app logic will access consistent bits of state.

Other toolkits, such as Flutter, have a more explicit way to address the problem of stable identity. By default, you get diffing behavior based on the index of the child relative to its parent, but if you want to explicitly express that a node should have a stable identity, you provide it a [Key].

But I think it's possible to do even better, as Jetpack Compose shows. It has a very explicit concept of stable identity *tied to the position in the tree.* And one important function of its compiler plugin is to annotate the code with unique caller locations so the toolkit (by means of the [Composer] object) can track this accurately, even when the app logic has conditionals and loops of the kind disfavored by React. They call the concept "positional memoization" and I think it's well worth understanding their motivation for it.

While Jetpack Compose relies on a compiler plugin, fortunately stable Rust (as of [1.46]) has a `#[track_caller]` feature that also gives access to unique caller locations. [Moxie] uses this feature to track unique identities (in fact the creator of Moxie drove stabilization of the feature), providing access to state very similar to Jetpack Compose.

Crochet adopts this concept wholeheartedly, taking it a step further even. In Crochet, actions from widgets (like button presses) are presented to the app logic based on their position in the tree, an idiom very similar in fact to imgui (`if button() { ... }`), and internally relies on stable node identity to make this work. By contrast, Jetpack Compose and Moxie both rely on observable objects to propagate actions from widgets to the app logic.

## About the prototype

A purely academic inquiry into these themes might be somewhat interesting, but I feel like I only really understand things when I code them up and play with them. To that end, I've coded up a prototype, using Druid as essentially the render object (widget) engine. It's very crude when it comes to actual UI functionality, but does explore most of the main reactive themes, including prototypes of scripting through Python, async integration, and a sketch of how to handle large collections (list views) efficiently.

A detailed description of the theory of how Crochet works is beyond the scope of this post; hopefully that will follow when the time is right. But I do think it is useful that this post at least sets out the motivations and concerns. Essentially, it is a statement of what problems I'm trying to solve. The curious will find more information in the README in the repo, and the very curious will find their thirst slaked by reading the code.

Of the systems I've studied, Crochet is most similar to Jetpack Compose, with the following exceptions:

* Getting actions from widgets is more like imgui, and does not use an observable.
* Crochet doesn't use a compiler. The need for it is reduced by two factors:
   * No need to transform data access into observables.
   * Location tracking is through #[track_caller], as in Moxie.
* The low level tree mutation API is beefed up, for use by list views, etc.
* Crochet doesn't do "recompose", but always starts from the root.
* There is a hook for integrating with Rust's async.

I think getting rid of recompose is a good simplification, especially within the context of Rust. I don't expect performance issues, because I expect components will be more aggressive in explicit skipping of subtrees, with help from the low-level API and greater use of immutable data structures. Of course, we don't know for sure how this will work out until we get more experience with it.

I encourage people to experiment with the [Crochet] code. In addition, I'm experimenting with a policy of [optimistic merging], as I want people to be able to try things with low friction.

So far, experience with the prototype is positive. The thing I'm most interested in is informed criticism, in particular app logic patterns that are *not* easy to express.

Obviously there is a long journey from such a rough, early prototype to a proper implementation, and there are many other problems to be solved. So I make no promises of any kind when, or even if, that will happen. In the meantime, I hope people find the research exploration to be interesting.

Work on Druid is generously funded by Google Fonts. The ideas and designs in this post were influenced by discussions with many friends, including but not limited to Adam Perry, Colin Rofls, Chris Morgan, and Tristan Hume.

Discuss on [Hacker News](https://news.ycombinator.com/item?id=24599560) or [/r/rust](https://www.reddit.com/r/rust/comments/j0840q/towards_principled_reactive_ui/).

[Towards a unified theory of reactive UI]: https://raphlinus.github.io/ui/druid/2019/11/22/reactive-ui.html
[Druid]: https://github.com/linebender/druid
[Crochet]: https://github.com/raphlinus/crochet
[diff]: https://github.com/google/diff-match-patch
[Jetpack Compose]: https://medium.com/androiddevelopers/under-the-hood-of-jetpack-compose-part-2-of-2-37b2c20c6cdd
[RRB-Trees]: http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.592.5377
[strict equality]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Strict_equality
[useState]: https://reactjs.org/docs/hooks-state.html
[Rules of Hooks]: https://reactjs.org/docs/hooks-rules.html
[Key]: https://api.flutter.dev/flutter/foundation/Key-class.html
[Composer]: https://developer.android.com/reference/kotlin/androidx/compose/runtime/Composer
[Moxie]: https://moxie.rs/
[imgui]: https://github.com/ocornut/imgui
[johno on IMGUI]: http://www.johno.se/book/imgui.html
[Conrod]: https://github.com/PistonDevelopers/conrod
[Iced]: https://github.com/hecrj/iced
[Static Types in SwiftUI]: https://www.objc.io/blog/2019/11/05/static-types-in-swiftui/
[`View`]: https://developer.apple.com/documentation/swiftui/view
[Adapton]: http://adapton.org/
[Incremental]: https://blog.janestreet.com/introducing-incremental/
[Bonsai]: https://github.com/janestreet/bonsai
[Combine]: https://developer.apple.com/documentation/combine
[Makepad]: https://github.com/makepad/makepad
[Elm]: https://guide.elm-lang.org/architecture/
[relm]: https://github.com/antoyo/relm
[vgtk]: https://github.com/bodil/vgtk
[Future]: https://rust-lang.github.io/async-book/02_execution/02_future.html
[DOM]: https://developer.mozilla.org/en-US/docs/Web/API/Document_Object_Model/Introduction
[automerge]: https://github.com/automerge/automerge
[1.46]: https://blog.rust-lang.org/2020/08/27/Rust-1.46.0.html
[optimistic merging]: http://hintjens.com/blog:106
[7GUIs]: https://eugenkiss.github.io/7guis/
[Svelte]: https://svelte.dev/
