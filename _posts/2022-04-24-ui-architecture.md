---
layout: post
title:  "Xilem: an architecture for UI in Rust"
date:   2022-05-07 08:17:42 -0700
categories: [rust, gui]
---
Rust is an appealing language for building user interfaces for a variety of reasons, especially the promise of delivering both performance and safety. However, finding a good *architecture* is challenging. Architectures that work well in other languages generally don't adapt well to Rust, mostly because they rely on shared mutable state and that is not idiomatic Rust, to put it mildly. It is sometimes asserted for this reason that Rust is a poor fit for UI. I have long believed that it is possible to find an architecture for UI well suited to implementation in Rust, but my previous attempts (including the current [Druid] architecture) have all been flawed. I have studied a range of other Rust UI projects and don't feel that any of those have suitable architecture either.

This post presents a new architecture, which is a synthesis of existing work and a few new ideas. The goals include expression of modern reactive, declarative UI, in components which easily compose, and a high performance implementation. UI code written in this architecture will look very intuitive to those familiar with state of the art toolkits such as SwiftUI, Flutter, and React, while at the same time being idiomatic Rust.

The name "Xilem" is derived from a [xylem], a type of transport tissue in vascular plants, including trees. The word is spelled with an "i" in several languages including Romanian and Malay, and is a reference to [xi-editor], a starting place for explorations into UI in Rust (now on hold).

Like most modern UI architectures, Xilem is based on a *view tree* which is a simple declarative description of the UI. For incremental update, successive versions of the view tree are *diffed,* and the results are applied to a widget tree which is more of a traditional retained-mode UI. Xilem also contains at heart an incremental computation engine with precise change propagation, specialized for UI use.

The most innovative aspect of Xilem is event dispatching based on an *id path,* at each stage providing mutable access to app state. A distinctive feature is Adapt nodes (an evolution of the lensing concept in Druid) which facilitate composition of components. By routing events *through* Adapt nodes, subcomponents have access to a different mutable state reference than the parent.

## A quick tour of existing architectures

This architecture is designed to address limitations and problems with the existing state of the art, both the current [Druid] architecture and other attempts. It's a little hard to understand some of the motivation without some knowledge of those architectures. That said, a full survey of reactive UI architectures would be quite a long work; this section can only touch the highlights.

The existing Druid architecture has some nice features, but we consistently see people struggle with common themes.

* There is a big difference between creating static widget hierarchies and dynamically updating them.
* The app data must have a [Data](https://docs.rs/druid/latest/druid/trait.Data.html) bound, which implies cloning and equality testing. Interior mutability is effectively forbidden.
* The "lens" mechanism is confusing and it is not easy to implement complex binding patterns.
* We never figured out how to integrate async in a compelling way.
* There is an environment mechanism but it is not efficient and doesn't support fine-grained change propagation.

Another common architecture is immediate mode GUI, both in a relatively pure form and in a modified form. It is popular in Rust because it doesn't require shared mutable state. It also benefits from overall system simplicity. However, the model is oversimplified in a number of ways, and it is difficult to do sophisticated layout and other patterns that are easier in retained widget systems. There are also numerous papercuts related to sometimes rendering stale state. (I experimented with a retained widget backend emulating an immediate mode API in the "crochet" architecture experiment and concluded that the result was not compelling). The popular [egui] crate is solidly an implementation of immediate mode, and [makepad] is also based on it, though it differs in some important ways.

A particularly common architecture for UI in Rust is [The Elm Architecture], which also does not require shared mutable state. Rather, to support interactions from the UI, gestures and other related UI actions creates *messages* which are then sent to an `update` method which takes central app state. [Iced], [relm](https://github.com/antoyo/relm), and [Vizia](https://github.com/vizia/vizia) all use some form of this architecture. Generally it works well, but the need to create an explicit message type and dispatch on it is verbose, and the Elm architecture does not support cleanly factored components as well as some other architures. The [Elm documentation](https://guide.elm-lang.org/webapps/structure.html) specifically warns against components, saying, "actively trying to make components is a recipe for disaster in Elm."

Lastly, there are a number of serious attempts to port React patterns to Rust, of which I find [Dioxus] most promising. These rely on interior mutability and other patterns that I think adapt poorly to Rust, but definitely represent a credible alternative to the ideas presented here. I think we will have to build some things and see how well they work out.

## Synchronized trees

The Xilem architecture is based around generating trees and keeping them synchronized. In that way it is a refinement of the ideas described in my previous blog post, [Towards a unified theory of reactive UI].

In each "cycle," the app produces a view tree, from which rendering is derived. This tree has fairly short lifetime; each time the UI is updated, a new tree is generated. From this, a widget tree is built (or rebuilt). The view tree is retained only long enough to assist in event dispatching and then be diffed against the next version, at which point it is dropped. The widget tree, by contrast, persists across cycles. In addition to these two trees, there is a third tree containing *view state,* which also persists across cycles. (The view state serves a very similar function as React hooks)

Of existing UI architectures, the view tree most strongly resembles that of SwiftUI - nodes in the view tree are plain value objects. They also contain callbacks, for example specifying the action to be taken on clicking a button. Like SwiftUI, but somewhat unusually for UI in more dynamic languages, the view tree is statically typed, but with a typed-erased escape hatch (Swift's AnyView) for instances where strict static typing is too restrictive.

The Rust expression of these trees is instances of the `View` trait, which has two associated types, one for view state and one for the associated widget. The state and widgets are *also* statically typed. The design relies *heavily* on the type inference mechanisms of the Rust compiler. In addition to inferring the type of the view tree, it also uses associated types to deduce the type of the associated state tree and widget tree, which are known at compile time. In almost every other comparable system (SwiftUI being the notable exception), these are determined at runtime with a fair amount of allocation, downcasting, and dynamic dispatch.

## A worked example

We'll use the classic counter as a running example. It's very simple but will give insight into how things work under the hood. For people who want to follow along with the code, check the idiopath directory of the [idiopath branch] (in the Druid repo); running `cargo doc --open` there will reveal a bunch of Rustdoc.

Here's the application.

```rust
fn app(count: &mut u32) -> impl View<u32> {
    v_stack((
        format!("Count: {}", count),
        button("Increment", |count| *count += 1),
    ))
}
```

This was carefully designed to be clean and simple. A few notes about this code, then we'll get in to what happens downstream to actually build and run the UI.

This function is run whenever there are significant changes (more on that later). It takes the current app state (in this case a single number, but in general app state can be anything), and returns a view tree. The exact type of the view tree is not specified, rather it uses the [impl Trait] feature to simply assert that it's something that implments the View trait (parameterized on the type of the app state). The full type happens to be:

```rust
VStack<u32, (), (String, Button<u32, (), {anonymous function of type FnMut(u32) -> ()}>)>
```

For such a simple example, this is not too bad (other than the callback), but would get annoying quickly.

Another observation is that three nodes of this tree implement the View trait: VStack and its two children String and Button. Yes, that's an ordinary Rust string, and it implements the View trait. So will colors and shapes (following SwiftUI; implementation is planned in the prototype but not complete).

The view tree returned by the app logic can be visualized as a block diagram:

![Box diagram showing view hierarchy](/assets/xilem_view.svg)

The type of the associated widget is `widget::VStack`. Purely as a pragmatic implementation detail, we've chosen to type-erase the children of containers. Among other things, this avoids excessive monomorphization. If we wanted fully-typed widget trees, the architecture would support that, and indeed an earlier prototype chose that approach.

### Identity and id paths

A specific detail when building the widget tree is assigning a *stable identity* to each view. These concepts are explained pretty well in the [Demystify SwiftUI] talk. As in SwiftUI, stable identity can be based on *structure* (views in the same place in the view tree get to keep their identity across runs), or an *explicit key.* To illustrate the latter, assume a list container, and that two of the elements in the list are swapped. That might play an animation of the visual representations of those two elements changing places.

The idea of assigning stable identities is quite standard in declarative UI (it's also present in basically all non-toy immediate mode GUI implementations), but Xilem adds a distinctive twist, the use of *id path* rather than a single id. The id path of a widget is the sequence of all ids on the path from the root to that widget in the widget tree. Thus, the id path of the button in the above is `[1, 3]`, while the label is `[1, 2]` and the stack is just `[1]`.

To continue our running example, given the view tree, the build step produces an associated view state tree as well as a widget tree. Here, we've annotated the view tree with ids, and also shown how ids and id paths are stored in the newly built structures. Most importantly, the view state for the VStack contains the ids of the child nodes, and the Button widget contains its id path, which is `[1, 3]`.

![Box diagram showing view hierarchy](/assets/xilem_expanded.svg)
 
As it turns out, neither text labels nor buttons require any special view state other than what's retained in the widget, so those view states are just (), the unit type. Of course, other view nodes may require more associated state, in which case the type would be something other than the unit type.

<details>
<summary>Advanced topic: does id identify view or widget?</summary>

In writing this explanation, I realized there is some ambiguity whether the id identifies a node in the view tree, or one in the widget tree. In many cases, there is a 1:1 correspondence between the two, so the distinction is not important. In addition, it's certainly possible to construct a widget tree so that the id of each node is provided during the method call that constructs that widget, and if so, it's more than reasonable to use the Xilem ids as generated during the reconciliation process. However, that's not required, and the identity of widgets could be from a disjoint space from Xilem ids assigned to views.

Further, it's possible to have a view node that doesn't directly correspond to a widget. That happens with holders of async futures, environment getters, and potentially other nodes that have reason to receive events. In those cases, ids are associated with views, not widgets. It's important to be clear, though, that the *lifetime* of an id is persistent across multiple cycles, while the lifetime of nodes in the view tree is shorter.

Thus, the most accurate description of Xilem ids is this: they are *persistent* identifiers that correspond to either structural or explicit identity of nodes in the view tree.
</details>

### Event propagation

Now let's click that button. Raw platform events such as mouse movement and keystrokes are handled by the UI infrastructure, and don't necessarily result in events propagated to the app logic. For example, the mouse can hover over the button (changing the appearance to a hover state), or the window can be resized, and that will all be handled without involving the app. But clicking the button *does* generate an event to be dispatched to the app.

Obviously the goal will be to run that callback and increment the count, but the details of how that happens are subtly different than most declarative UI systems. Probably the "standard" way to do this would be to attach the callback to the button, and have it capture a reference to the chunk of state it mutates. Again, in most declarative systems but not Xilem, setting the new state would be done using some variant of the [observer pattern], for example some kind of `setState` or other "setter" function to not only update the value but also notify downstream dependencies that it had changed, in this case re-rendering the label.

This standard approach works poorly in Rust, though it can be done (see in particular the [Dioxus] system for an example of one of the most literal transliterations of React patterns, including observer-based state updating, into Rust). The problem is that it requires *shared mutable* access to that state, which is clunky at best in Rust (it requires interior mutability). In addition, because Rust doesn't have built-in syntax for getters and setters, invoking the notification mechanism also requires some kind of explicit call (though perhaps macros or other techniques can be used to hide it or make it less prominent).

<details>
<summary>Advanced topic: comparison with Elm</summary>

The observer pattern is not the *only* way event propagation works in declarative UI. Another very important and influential pattern is [The Elm Architecture], which, being based on a pure functional language, also does not require shared mutable state. Thus, it is also used successfully as the basis of several Rust UI toolkits, notably Iced.

In Elm, app state is centralized (this is also a fairly popular pattern in React, using state management packages such as Redux), and events are given to the app through an `update` call. Dispatching is a three-stage process. First, the user defines a *message* type enumerating the various actions that are (globally) possible to trigger through the UI. Second, the UI element *maps* the event type into this user-defined type, identifying which action is desired. Third, the `update` method dispatches the event, delegating if needed to a child handler. Some people like the explicitness of this approach, but it is unquestionably more verbose than a single callback that manipulates state directly, as in React or SwiftUI.
</details>

So what does Xilem do instead? The view tree is also parameterized on the *app state,* which can be any type. This idea is an evolution of Druid's existing architecture, which also offers mutable access to app state to UI callbacks, but removes some of the limitations. In particular, Druid requires app state to be clonable and diffable, a stumbling block for many new users.

When an event is generated, it is annotated with the path of the UI element that originated it. In the case of the button, `[1, 3]`, and this is sent to the app logic for dispatching. In the case of a button click, there is no *payload,* but for a slider it would be the numeric slider value, or for a text input it would be the string.

Event dispatching starts at the root of the view tree, and calls to the `event` method on the `View` trait also contain a mutable borrow of the app state. In this example, the root view node is VStack. It examines the id path of the event, consults its associated view state, and decides that the event should be dispatched to the second child, as the id of that child (3) matches the corresponding id in the id path of the event. It recursively calls `event` on that child, which is the button. That is a *leaf node,* meaning there are no further ids in the id path, and the button handles that event by calling its callback, passing in the mutable borrow of app state that was propagated through the `event` traversal. That callback in turn just increments the count value.

Note that the UI framework retains the view tree a "fairly short" time - long enough to do any event dispatching that's needed, and also for diffing (see below), but not longer than that.

### Re-rendering

After clicking the button and running the callback, the app state consists of the number 1, formerly 0. The app logic function is run, producing a new view tree, and this time the string value is "Count: 1" rather than "Count: 0". The challenge is then to update the widget tree with the new data.

As is completely standard in declarative UI, it is done by diffing the old view tree against the new one, in this case calling the `rebuild` method on the `View` trait. This method compares the data, updates the associated widget if there are any changes, and also traverses into children. The view tree is retained *just* long enough to do event propagation and to be diffed against the next iteration of the view tree, at which point it is dropped. At any point in time, there are at most two copies of the view tree in existence.

In the simplest case, the app builds the full view tree, and that is diffed in full against the previous version. However, as UI scales, this would be inefficient, so there are *other* mechanisms to do finer grained change propagation, as described below.

## Components

The above is the basic architecture, enough to get started. Now we will go into some more advanced techniques.

It would be very limiting to have a single "app state" type throughout the application, and require all callbacks to express their state mutations in terms of that global type. So we won't do that.

The main tool for stitching together components is the `Adapt` view node. This node is so named because it adapts between one app state type and another, using a closure that takes mutable access to the parent state, and calls into a child (through a "thunk", which is just a callback for continuing the propagation to child nodes) with a mutable reference to the child state. We can then define a "component" in the Xilem world as a body of code that outputs a view tree with a different app state type than its parent component.

In the simple case where the child component operates independently of the parent, the adapt node is a couple lines of code. It is also an attachment point for richer interactions - the closure can manipulate the parent state in any way it likes. Event handling callbacks of the child component are also allowed to return an arbitrary type (unit by default), for propagation of data upward in the tree, including to parent components.

In Elm terminology, the Adapt node is similar to [Html map][Elm Html map], though it manipulates mutable references to state, as opposed to being a pure functional mapping between message types. It is also quite similar to the "lens" concept from the existing Druid architecture, and has some resemblance to [Binding][SwiftUI Binding] in SwiftUI as well.

## Finer grained change propagation: memoizing

Going back to the counter, every time the app logic is called, it allocates a string for the label, even if it's the same value as before. That's not too bad if it's the only thing going on, but as the UI scales it is potentially wasted work.

Ron Minsky has [stated][Signals and Threads: Building a UI framework] "hidden inside of every UI framework is some kind of incrementalization framework." Xilem unapologetically contains at its core a lightweight change propagation engine, similar in scope to the attribute graph of SwiftUI, but highly specialized to the needs of UI, and in particular with a lightweight approach to *downward* propagation of dependencies, what in React would be stated as the flow of props into components.

In this particular case, that incremental change propagation is best represented as a *memoization* node, yet another implementation of the View trait. A memoization node takes a data value (which supports both `Clone` and equality testing) and a closure which accepts that same data type. On rebuild, it compares the data value with the previous version, and only runs the closure if it has changed. The signature of this node is very similar to [Html.Lazy] in Elm.

Comparing a number is extremely cheap (especially because all this happens with static typing, so no boxing or downcasting is needed), but the cost of equality comparison is a valid concern for larger, aggregate data structures. Here, immutable data structures (adapted from the existing Druid architecture) can work very well.

Let's say there's a parent object that contains all the app state, including a sizable child component. The type would look something like this:

```rust
#[derive(Clone)]
struct Parent {
    stuff: Stuff,
    child: Arc<Child>,
}
```

And at the top of the tree we can use a memoize node with type `Arc<Parent>`, and the equality comparison [pointer equality] on the `Arc` rather than a deep traversal into the structure (as might be the case with a derived `PartialEq` impl). The child component attaches with both a Memoize and an Adapt node.

The details of the Adapt node are interesting. Here's a simple approach:

```rust
adapt(
    |data: &mut Arc<Parent>, thunk| thunk.call(&mut Arc::make_mut(data).child),
    child_view(...) // which has Arc<Child> as its app data type
)
```

Whenever events propagate into the child, `make_mut` creates a copy of the parent struct, which will then not be pointer-equal to the version stored in the memoize node. If such events are relatively rare, or if they nearly always end up mutating the child state, then this approach is reasonable. However, it is possible to be even finer grain:

```rust
adapt(
    |data: &mut Arc<Parent>, thunk| {
        let mut child = data.child.clone();
        thunk.call(&mut child),
        if !Arc::ptr_eq(&data.child, &child) {
            Arc::make_mut(data).child = child;
        }
    },
    child_view(...) // which has Arc<Child> as its app data type
)
```

This logic propagates the change up from the child object to the parent object in the app state *only if* the child state has actually changed.

The above illustrates how to make the pattern work for structure fields (and is very similar to the "lensing" technique in the existing Druid architecture), but similar ideas will work for collections. Basically you need immutable data structures that support pointer equality and cheap, sparse diffing. I talk about that in some detail in my talk [A Journey Through Incremental Computation] ([slides]), with a focus on text layout and a list component. Also note that the [druid_derive] crate automates generation of these lenses for Druid, and no doubt a similar approach would work for adapt/memoize in Xilem. For now, though, I'm seeing how far we can get just using vanilla Rust and not relying on macros. I think all this is a fruitful direction for future work.

Also to note: while immutable data structures work *well* in the Xilem architecture, they are not absolutely required. The `View` trait itself can be implemented by anyone, as long as the `build`, `rebuild`, and `event` methods have correct implementation; change propagation is especially the domain of the `rebuild` method.

## Type erasure

This section is optional but contains some interesting bits on advanced use of the Rust type system.

Having the view tree (and associated view state and widget tree) be fully statically typed has some advantages, but the types can become quite large, and there are cases where it is important to *erase* the type, providing functionality very similar to [AnyView][SwiftUI AnyView] in SwiftUI.

For a simple trait, the standard approach would be `Box<dyn Trait>`, which boxes up the trait implementation and uses dynamic dispatch. However, this approach will not work with Xilem's View trait, because that trait is not [object-safe][object safety]. There are actually two separate problems - first, the trait has associated types, and second, the `rebuild` method takes the previous view tree for diffing purposes as a `Self` parameter; though the contents of the view trees might differ, the type remains constant.

Fortunately, though simply `Box<dyn View>` is not possible due to the View trait not being object-safe, there is a pattern (due to David Tolnay) for [type erasure][erased-serde]. You can look at the code for details, but the gist of it is a separate trait (`AnyView`) with `Any` in place of the associated type, and a blanket implementation (`impl<T> AnyView for T where T: View`) that does the needed downcasting.

The details of type erasure in Xilem took a fair amount of iteration to get right (special thanks to Olivier Faure and Manmeet Singh for earlier prototypes). An [earlier iteration](https://github.com/linebender/druid/pull/1669) of this architecture used the Any/downcast pattern everywhere, and I also see that pattern for associated state in [rui] and [iced_pure], even though the main view object is statically typed.

In the SwiftUI community, `AnyView` is [frowned on][Avoiding SwiftUI’s AnyView], but it is still useful to have it. While `impl Trait` is a powerful tool to avoid having to write out explicit types, it doesn't work in all cases (specifically as the return type for trait methods), though there is [work to fix that][impl Trait in type aliases]. There is an additional motivation for type erasure, namely language bindings.

### Language bindings for dynamic languages

As part of this exploration, I wanted to see if Python bindings were viable. This goal is potentially quite challenging, as Xilem is fundamentally a (very) strongly typed architecture, and Python is archetypally a loosely typed language. One of the limitations of the existing Druid architecture I wanted to overcome is that there was no satisfying way to create Python bindings. As another negative data point, no dynamic language bindings for SwiftUI have emerged in the approximately 3 years since its introduction.

Yet I was able to create a fairly nice looking proof of concept for Xilem Python bindings. Obviously these bindings rely heavily on type erasure. The essence of the integration is `impl View for PyObject`, where the main instance is a Python wrapper around `Box<dyn AnyView>` as stated above. In addition, `PyObject` serves as the type for both app state and messages in the Python world; an `Adapt` node serves to interface between these and more native Rust types. Lastly, to make it all work we need `impl ViewSequence for PyTuple` so that Python tuples can serve as the children of containers like VStack, for building view hierarchies.

I should emphasize, this is a [proof of concept](https://github.com/linebender/druid/pull/2185). To do a polished set of language bindings is a fairly major undertaking, with care needed to bridge the impedance mismatch, and especially to provide useful error messages when things go wrong. Even so, it seems promising, and, if nothing else, serves to demonstrate the flexibility of the architecture.

## Async

The interaction between async and UI is an extremely deep topic and likely warrants a blog post of its own. Even so, I wanted to explore it in the Xilem prototype. Initial prototyping indicates that it can work, and that the integration can be fine grained.

Async and change propagation for UI have some common features, and the Xilem approach has parallels to [Rust's async ecosystem][Rust async]. In particular, the id path in Xilem is roughly analogous to the "waker" abstraction in Rust async - they both identify the "target" of the notification change.

In fact, in the prototype integration, the waker provided to the Future trait is a thin wrapper around an id path, as well as a callback to notify the platform that it should wake the UI thread if it is sleeping. Somewhat unusually for Rust async, each `View` node holding a future calls `poll` on it itself; in some respects, a future-holding view is like a tiny executor of its own. A UI built with Xilem does not provide its own reactor, but rather relies on existing work such as [tokio] (which was used for the prototype).

We refer the interested reader to [the prototype code](https://github.com/linebender/druid/pull/2184) for more details. Clearly this is an area that deserves to be explored much more deeply.

## Environment

A standard feature of declarative UI is a dynamically scoped *environment* or *context* in which child nodes can retrieve information, usually in some form of key/value format, from ancestors somewhere higher up in the tree. Most Rust UI architectures have this (including existing Druid), but you should ask: is there fine-grained change propagation, or does it have to rebuild all children when *any* environment key changes?

We have a design for this, not fully implemented yet. The `Cx` structure threaded through the reconciliation process gains an environment, which is a map from key to (subscribers, value) tuples. The subscribers field is a set of id paths (of child nodes). There are then two `View` nodes, one for setting an environment value, which is then available to descendants, and one for retrieving that value. Let's look at the `build` and `rebuild` methods for both of those nodes, the setter first.

The associated state of the setter is the value and the subscriber set. On `build` it creates that state, with the subscriber set empty. On `rebuild` it compares the new value with that stored in the state (these values must be equality-comparable), and, if they differ, sends a notification to each of the subscribers in the subscriber set, using the id path to dispatch those notifications. In both cases, it recurses to the child node, then pops the key from the environment stack in `Cx` before it returns (here, "pop" means either removing the key from the environment map, or setting the value to what it was before traversing into the setter node, depending on that previous value).

In the `build` case, it fetches the value and subscriber set from the environment map, and *adds* itself to that subscriber set. In either the `build` or `rebuild` case, it retrieves the value and calls the closure for its child view tree, passing in the value retrieved from the environment. Note that the "subscriber set" concept is an adaptation of the classic observer pattern to the Xilem architecture.

One reason this hasn't been prototyped is that the implementation details are fairly different depending on whether reconciliation can be multithreaded (a note on that below). It's possible to do but requires an immutable map data structure to avoid high cloning cost, as well as interior mutability for the subscription set.

We have a similar prototype of a `useState` mechanism, but not enough data on its usefulness to say for sure whether it carries its weight. Such a mechanism does not seem to be needed in the Elm architecture.

## Other topics

So far, I haven't deeply explored styling and theming. These operations also potentially ride on an incremental change propagation system, especially because dynamic changes to the style or theme may propagate in nontrivial ways to affect the final appearance.

Another topic I'm *very* interested to explore more fully is accessibility. I *expect* that the retained widget tree will adapt nicely to accessibility work such as Matt Campbell's [AccessKit], but of course you never know for sure until you actually try it.

An especially difficult challenge in UI toolkits is sparse scrolling, where there is the illusion of a very large number of child widgets in the scroll area, but in reality only a small subset of the widgets outside the visible viewport are materialized. I am hopeful that the tight coupling between view and associated widget, as well as a lazy callback-driven creation of widgets, will help with this, but again, we won't know for sure until it's built.

Another very advanced topic is the ability to exploit parallelism (multiple threads) to reduce latency of the UI. The existing Druid architecture threads a mutable context to almost all widget methods, basically precluding any useful parallelism. In the Xilem architecture, creation of the View tree itself can easily be multithreaded, and I *think* it's also possible to do multithreaded reconciliation. The key to that is to make the `Cx` object passed to the `build` and `rebuild` methods `Clone`, which I think is possible. Again, actually realizing performance gains from this approach is a significant challenge.

## Prospects

The work presented in this blog post is conceptual, almost academic, though it is forged from attempts to build real-world UI in Rust. It comes to you at an early stage; we haven't *yet* built up real UI around the new architecture. Part of the motivation for doing this writeup is so we can gather feedback on whether it will actually deliver on its promise.

One way to test that would be to try it in other domains. There are quite a few projects that implement reactive UI ideas over a TUI, and it would also be interesting to try the Xilem architecture on top of Web infrastructure, generating DOM nodes in place of the associated widget tree.

I'd like to thank a large number of people, though of course the mistakes in this post are my own. The Xilem architecture takes a lot of inspiration from Olivier's [Panoramix] and Manmeet's [olma] explorations, as well as Taylor Holliday's [rui]. Jan Pochyla provided useful feedback on early versions, and conversations with the entire Druid crew on [xi.zulipchat.com] were also informative. Ben Saunders provided valuable insight regarding Rust's async ecosystem.

[Druid]: https://github.com/linebender/druid
[Xylem]: https://en.wikipedia.org/wiki/Xylem
[xi-editor]: https://xi-editor.io/
[impl Trait]: https://doc.bccnsoft.com/docs/rust-1.36.0-docs-html/edition-guide/rust-2018/trait-system/impl-trait-for-returning-complex-types-with-ease.html
[Demystify SwiftUI]: https://developer.apple.com/videos/play/wwdc2021/10022/
[Observer pattern]: https://en.wikipedia.org/wiki/Observer_pattern
[Dioxus]: https://dioxuslabs.com/
[The Elm Architecture]: https://guide.elm-lang.org/architecture/
[Iced]: https://iced.rs/
[Elm Html map]: https://package.elm-lang.org/packages/elm/html/latest/Html#map
[Html.Lazy]: https://guide.elm-lang.org/optimization/lazy.html
[SwiftUI Binding]: https://developer.apple.com/documentation/swiftui/binding
[erased-serde]: https://github.com/dtolnay/erased-serde#how-it-works
[impl Trait in type aliases]: https://github.com/rust-lang/rust/issues/63063
[SwiftUI AnyView]: https://developer.apple.com/documentation/swiftui/anyview
[object safety]: https://huonw.github.io/blog/2015/01/object-safety/
[Avoiding SwiftUI’s AnyView]: https://www.swiftbysundell.com/articles/avoiding-anyview-in-swiftui/
[rui]: https://github.com/audulus/rui
[iced_pure]: https://docs.rs/iced_pure/latest/iced_pure/
[Data Driven UIs, Incrementally]: https://www.youtube.com/watch?v=R3xX37RGJKE
[Salsa]: https://docs.rs/salsa/latest/salsa/
[Adapton]: https://docs.rs/adapton/latest/adapton/
[Signals and Threads: Building a UI framework]: https://signalsandthreads.com/building-a-ui-framework/#1523
[pointer equality]: https://doc.rust-lang.org/std/sync/struct.Arc.html#method.ptr_eq
[A Journey Through Incremental Computation]: https://www.youtube.com/watch?v=DSuX-LIAU-I
[slides]: https://docs.google.com/presentation/d/1opLymkreSTFfxygjzSLYI_uH7j1YFfE6DLl8RfCiw7E/edit
[druid_derive]: https://docs.rs/druid-derive/latest/druid_derive/
[Panoramix]: https://github.com/PoignardAzur/panoramix
[Olma]: https://github.com/Maan2003/olma
[xi.zulipchat.com]: https://xi.zulipchat.com/
[Rust async]: https://rust-lang.github.io/async-book/08_ecosystem/00_chapter.html
[tokio]: https://tokio.rs/
[egui]: https://github.com/emilk/egui
[makepad]: https://github.com/makepad/makepad
[idiopath branch]: https://github.com/linebender/druid/pull/2183
[Towards a unified theory of reactive UI]: https://raphlinus.github.io/ui/druid/2019/11/22/reactive-ui.html

