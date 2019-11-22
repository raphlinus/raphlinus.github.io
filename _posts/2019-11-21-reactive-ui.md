---
layout: post
title:  "Towards a unified theory of reactive UI"
date:   2019-11-22 8:44:42 -0800
categories: [ui, druid]
---
In trying to figure out the best reactive structure for [druid], as well as how to communicate that to the world, I've been studying a wide range of reactive UI systems. I've found an incredible diversity, even though they have fairly consistent goals. This post is an attempt to find common patterns, to characterize the design space as a whole. It will be rough, at some points almost a stream of consciousness. If I had the time and energy, I think it could be expanded into an academic paper. But, for now, perhaps these rough thoughts are interesting to some people working in the space.

## Reactive UI

Without going into a complicated definition of declarative or reactive UI (or the distinctions between these two), it can be characterized as enabling application logic to be written like this:

```swift
struct AppState {
    count: i32
}

VStack {
    Text("count: \(state.count)")
    Button("Increment") {
        state.count += 1
    }
}
```


This is not in any particular syntax, but written to draw attention to the contrast from the more traditional ("object-oriented") style:


```swift
    var count = 0
    let stack = new VStack
    let text = new Text("Count: \(count)")
    stack.add_child(text)
    let button = new Button("Increment")
    button.set_onclick(||
        count += 1
        text.set_text("Count: \(count)")
    )
    stack.add_child(button)
```

For such a simple example, this is fine, but already we see the duplication between the initial construction of the UI and updates. It gets worse as the UI becomes more complex; for example, to add a slider showing the value, the onclick handler also needs to be amended to add an update for that.

My own main motivation for exploring reactive UI is primarily because the object-oriented idioms don't translate well to Rust, mostly because ownership becomes tangled – widgets are owned by their containers but references to them also exist in the callbacks. But whatever the reasons, industry is fast converging on the reactive approach. That said, while the goals are converging, and most systems allow logic to be expressed as above with only minor syntax variations, the details of implementation still seem wildly divergent. The major focus of this post is to make sense of this space.

## Tree transformations

The main theoretical construct is that reactive UI is at heart a pipeline of tree transformations. This construct makes a number of simplifying assumptions: that trees are in fact a good way to model the data being transformed, that data flows down the pipeline (this is generally true for rendering, but not so much for behavior), and that there are no "escape hatches," which are other ways to mutate the tree other than having it be a result of the transform of the previous tree in the pipeline. Of course, the exceptions to these simplifying assumptions are often a very important part to a particular reactive framework: it's definitely a way to add spice.

The core insight is that there are two fundamental ways to represent a tree. First, as a *data structure*, where you have nodes in memory somewhere, and parent nodes own their child nodes. Second, as a *trace of excecution,* which can be a number of different things, but generally is a sequence of events resulting from a (preorder) traversal of the tree. Most often, this trace reflects a call tree, with the call stack at any point reflecting the path from the root to a particular node in the tree.

<img src="/assets/reactive-ui-trees.svg" width="500" style="margin: auto; display: block;">

Obviously, it's possible to go back and forth between these two representations. If you have a data structure in memory, doing a preorder traversal of it generates the corresponding trace. Similarly, a piece of code that induces a tree in its execution trace can be annotated to generate a data structure in one of two main ways. In a more functional style, it can return a tree node as its return value. In a more imperative style, it can call mutation methods on a tree builder object. These two styles accomplish the same goal, and aren't that fundamentally different. But I think we can start to see one dimension of the diversity of reactive UI frameworks already, and I'll get into that a bit more later.

I'll sketch out a typical UI pipeline, then get into how to express transformations in the various stages. Obviously there can be variations and additional intermediate stages, but I think the archetypal reactive UI rendering pipeline is: user data, widget tree, render object (also known as "render element") tree, render objects annotated with layout, an optional but common intermediate stage, and finally a draw tree. It might sometimes make sense to consider the transformation from the draw tree to pixels as part of the pipeline, but pixels are not trees. (In some systems such as Flutter, there can be an intermediate stage between the draw tree and pixels, in this case a tree of [layers](https://api.flutter.dev/flutter/rendering/Layer-class.html), and in that case it absolutely does make sense to consider it part of the pipeline)

<img src="/assets/reactive-ui-pipeline.svg" width="600" style="margin: auto; display: block;">


So this is one of the dimensions on which to characterize the diversity of UI frameworks. At each stage in this pipeline, is the tree represented as a data structure or a trace? Both choices are generally viable, and in any pipeline when the tree is a trace, you can convert it pretty easily to a data structure by putting in a record/playback mechanism. This makes the stages more loosely coupled, but comes with costs: the memory for storing the tree nodes, and, often, some increased complexity, because trees can be messy to deal with. Later in this post, I'll characterize some important examples of UI frameworks along this dimension and others.

There are a number of reasons to analyze the problem of UI as a pipeline of tree stages. Basically, the end-to-end transformation is so complicated it would be very difficult to express directly. So it's best to break it down into smaller chunks, stitched together in a pipeline. This is especially true when the transformation is stateful, as it makes sense to attribute that state to a particular stage in the pipeline. A good example is the position of a draggable splitter pane – that's clearly a concern of the "render object" stage of the pipeline; storing it with the application logic would clutter that, and dealing with it at the draw object stage is too low level. There are other reasons to deal explicitly with separate trees, for example to make clear interface boundaries between different parts of the system, such as stitching together multiple languages (the DOM is a sharp example, as we'll see below).

Now let's get into the details of the transformations a bit. Generally you want to express this in code, because you need the flexibility. But the exact structure of the code depends on the representation of the tree, particularly data structure or trace, as well as other details.

### Push vs pull interfaces

Now is a good time to bring up "push" and "pull," I think. I was trying to make this concept apply to the entire pipeline, but found that real systems are hybrids. It makes more sense to apply it to a pipeline stage, or a sequence of pipeline stages. Since an in-memory data structure supports both push and pull, the most common structure you'll see is a sequence of stages pulling from the left and pushing to the right, with in-memory data structures at both ends.

Pulling from a tree basically means calling a function to access that part of the tree. In the simplest case, the tree is an in-memory data structure, so the function is just a getter on that node (attributes or children structure). But the more general case is very interesting as well, and if you have a pull-based protocol, it's pretty easy to, for example, add a map function for nodes of the tree.

It's very instructive at this point to look at (sequence) iterators, because sequences are a special case of trees, and there's also a very well developed ecosystem (especially in Rust) for dealing with them. The canonical (but not only) in-memory data structure is `Vec`, and a pull-based iteration protocol is [std::iter::Iterator](https://doc.rust-lang.org/std/iter/trait.Iterator.html). Going from data structure to trace is done with [iter](https://doc.rust-lang.org/beta/std/primitive.slice.html#method.iter), and the other direction with [collect](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.collect).

Rust iterators also have a rich collection of built-in transformation combinators, you're encouraged to write your own by impl'ing the `Iterator` trait, and all these are designed to compose. This is something of a model of what such an ecosystem should look like.

I'll also note that push-based iterators are also possible (called "internal iterators" in Rust lingo), and in fact [used to be standard](https://github.com/rust-lang/rust/issues/6978) in the infancy of the language. Basically, in internal iteration, the collection has a "foreach" method that takes a closure, and calls the closure for each value in the sequence. Certain things are easier to express in a push model (this is the ["streaming iterator"](https://internals.rust-lang.org/t/streaming-generators/5850) problem, also see [this blog](http://lukaskalbertodt.github.io/2018/08/03/solving-the-generalized-streaming-iterator-problem-without-gats.html)), but others, for example zip, are much easier in a pull model.

One more thing to say about the interplay between sequence iterators and trees: it's certainly possible to express a tree as its *flattening* into a sequence of events, particularly "push" and "pop" (also commonly "start" and "end" in parser lingo) to represent the tree structure. In fact, this is exactly what pulldown-cmark does, and one of the stated goals is to allow a class of tree transformations as transformations on this flattening, expressible as (composable) iterators. Again, applying a map function to nodes is a particularly easy transformation, and there are examples of that in the [pulldown-cmark README](https://github.com/raphlinus/pulldown-cmark). I'm not sure yet how generally useful this is in the context of UI, though.

### Incremental transformations

So one thing I haven't really mentioned yet is the fact that you want these tree-to-tree transformations to be *incremental.* In a UI, you're not dealing with one tree, but a time sequence of them, and most of the tree is not changing, just a small *delta.* One of the fundamental goals of a UI framework is to keep the deltas flowing down the pipeline small. (Though of course this is a tradeoff and imgui is an example of a framework that doesn't emphasize this aspect)

In general this is a pretty hard problem, because a change to some node on the input can have nontrivial effects on other nodes in the output. This is the problem of "tracking dependencies" and probably accounts for most of the variation between frameworks.

One measure of any given transformation is how tangled this graph of dependencies becomes. Some are fairly clean, we might use the word "local." But I think the user data → widget tree transformation in particular tends to have a complex dependency graph, so we need strategies to deal with it.

(Now's probably a good time to point out that this is a *very* familiar problem within the architecture of xi-editor, which concerned itself greatly with efficient propagation of deltas through the pipeline. The basic approach there was to represent these deltas very explicitly, and devise clever incremental algorithms to compute the next delta in the pipeline as a function of document state and the incoming delta. Such an approach is compelling, especially to a computer science PhD, but also accounted for a significant part of the complexity in xi, and my experience there suggests caution)

### Diffing

I think one of the most general and useful strategies is *diffing,* and there's good reason to believe this strategy applies well in UI domains. To sketch out briefly, you do a traversal (of some kind) of the output tree, and at each node you keep track of the *focus,* specifically which subtree of the input can possibly affect the result. Then you compare that input subtree to its previous state, and if it's the same you skip the output subtree, keeping the same value as before.

I use this term "focus" purposefully, as it's also a description of what a [lens] does. And indeed, one way to view lenses is that they're a way to explicitly encode dependencies. I'd say it's an "opt-out" expression of a dependency, an assertion that the stuff outside the focus is not a dependency, as opposed to the perhaps more intuitive "opt-in" expression that you see in, for example, explicit "onclick" handlers.

Diffing is also available in a less clever, more brute-force form, by actually going over the trees and comparing their contents. This is, I think, quite the common pattern in JS reactive frameworks, and, because it places the least burden on the programmer expressing the transformation logic, is often a good starting point. (More clever incremental strategies are often available as an opt-in, for when it matters to performance)

I'm not going to spend a lot of words on other incremental strategies, as it's a huge topic. But besides diffing, the other major approach is explicit deltas, most commonly (but not always) expressed as mutations. Then, you annotate the tree with "listeners," which are called when the node receives a mutation, and it does the bookkeeping to identify a subtree of the output that can change, and causes recomputation of that. There's a choice of *granularity* there, and a pattern I see a lot (cf [Recompose](https://developer.android.com/reference/kotlin/androidx/compose/package-summary#recompose) in Jetpack Compose) is to address a node in the output tree and schedule a pull-based computation of that node's contents.

#### Ways of identifying nodes

And this brings us to one of the fundamental problems in any push-based incremental approach: it becomes necessary to *identify* nodes in the output tree, so that it's possible to push content to them. I see a bunch of different approaches for this. The most basic is to use an object reference (borrowed from the underlying language). But another very common approach is some form of *key path,* (cf Swift [KeyPath](https://developer.apple.com/documentation/swift/keypath)), which is at heart a sequence of directions from the root to the identified node. There are a bunch of variations, but typically for a list the key path element is just the integer index. And a common choice for other path elements is unique call site identifiers.

The last common approach for identifying tree nodes is unique identifiers of some kind (usually just integers). This is the basic approach of [ECS](https://en.wikipedia.org/wiki/Entity_component_system) and also druid before the muggle branch.

Using object references is possible in Rust but makes ownership a lot messier, because you have to use some kind of reference counted container. I think another point to make about object references is that they nail down the fact that the tree is stored in a data structure at that point. The other approaches, particularly key paths, are more flexible about that.

## Case studies

Ok, since the above was so abstract I think a way to solidify the theory is to talk about particular cases in UI world. Not a systematic survey of every framework (that would be a major effort!) but examples that are especially illuminating.

### druid

In current druid, the pipeline looks like this: the data tree is stored, and explicitly designed to be diffable (the `Data` trait). The transformation to the widget tree is largely done in the `update` method, pulling diffs from the data tree, and results in a stored widget graph. (This might sound a little funny, as in the toy examples most of the building of the widget tree happens on app startup. But as apps become more dynamic more of the work is on update, so it's perhaps best to think of static startup as a special case where update is the constant function)

In current druid, the widget tree and render object tree are fused (one way to think of this is an identity transformation). The next stage is "render objects annotated with layout", and that's also a stored tree overlaid on the widget/render object tree (the layout annotations are stored in `WidgetPod` structs, being a major motivation for that struct to exist). This stage is done in the `layout` method, and again pulls from the input tree. Right now, it's not very incremental, it runs the whole tree from the top, but that's very much planned, and that will be mostly diff based.

The next stage, is going from the (layout annotated) render object tree to the draw tree, and this is a pretty direct traversal of the stored tree, pushing a trace to the piet [RenderContext](https://docs.rs/piet/0.0.7/piet/trait.RenderContext.html), which is a great example of an imperative trace-based protocol. In particular, the "push" and "pop" events for tree structure are clearly visible, here called [save](https://docs.rs/piet/0.0.7/piet/trait.RenderContext.html#tymethod.save) and [restore](https://docs.rs/piet/0.0.7/piet/trait.RenderContext.html#tymethod.restore), names borrowed from PostScript (and possibly from even before then). And this stage is represented in the `paint` method of the Widget trait.

I actually want to say a lot more about this, because longer term plans include moving this from trace to stored, and in particular with the stored nodes resident on the GPU. But this is, as you might imagine, a pretty big topic.

Actually talking about the render object to draw stage reminds me of some more general things I wanted to say about this stage. While this is basically the place to apply theme state, it tends to be a fairly local transformation. And I would say this transformation generally follows common patterns: basically each node type *expands* to generally more nodes in the draw tree. So one button node will transform to nodes for the button background, the outline, the button text, etc. I'll call this an "expand-style" transformation, and when the output stage is implemented by calling "emit" functions on an API, the most natural representation is just functions that call other functions.

The other general topic I left out was the question of how a schema for the trees might be represented in the language's type system (if it has one). In druid, the app state tree has a particular type, `T` with a `Data` bound. The widget tree is an interesting hybrid, each node is a `Box<dyn Widget>`, but then those widgets have an additional type parameter, which represents the focus of the corresponding data subtree. So at that stage the types include bits and pieces reflecting the transformation from app state to widgets. And the schema of the draw tree is represented as the signature of the piet RenderContext trait, a method call for each node type, and to push and pop tree structure.

An interesting observation is that there's a fairly clean mapping between the stages of the pipeline and the methods in the Widget trait - `event` is the only one I haven't identified with a tree transformation, and, indeed, that's not in the rendering pipeline but represents data flow in the reverse direction.

So that, I think, is a snapshot of druid within this theoretical framework. I'll touch on a couple others, largely to show contrasts.

### Imgui

[Imgui][Dear ImGui] is, as one can infer from the name, very much based on traces rather than stored trees. You can see basically the whole pipeline as a fused traversal of the app state, pushing draw objects to a GL context.

The "expand" pattern I identified above for the render object to draw transformation is expressed in a particularly clean way in imgui, it's just function calls. But I think the patterns here are relatively common in the different frameworks, and "expand" transformations are not especially difficult to express.

### Flutter

Touching just highlights, one of the most notable things about [Flutter](https://flutter.dev/) is that it does *not* fuse the widget and render object trees, having separate stored trees for both, and using the [createElement()](https://api.flutter.dev/flutter/widgets/Widget/createElement.html) method to do an expand-style transformation from one to the other.

### Jetpack Compose

There are a few notable things about [Jetpack Compose](https://developer.android.com/jetpack/compose). Where Flutter favors stored trees for both widgets and render objects, Jetpack Compose favors a trace style, but its compose buffer is actually something of a hybrid. The main structure expression of the output tree is trace (with our friends "push" and "pop" visible as [startGroup()](https://developer.android.com/reference/kotlin/androidx/compose/Composer.html#startGroup(kotlin.Any)) and [endGroup()](https://developer.android.com/reference/kotlin/androidx/compose/Composer.html#endGroup()), respectively, also note the key path fragment in that API), but this is not the whole story. The composer can also hold *state* associated with a particular node, with a fairly rich API for accessing the state, for example, [changed()](https://developer.android.com/reference/kotlin/androidx/compose/Composer.html#changed(androidx.compose.Composer.changed.T)) to do some diffing based on that state.

This hybrid nature, I think, gives Jetpack Compose a considerable amount of flexibility. But I think it's also (fitting a team at Google) complex, and in particular I would say that always starting from the root is a good simplification in druid, as opposed to this "recompose" business which is about restarting an incremental computation in the middle of the tree.

In both Jetpack Compose and imgui, you have a succession of stages which are basically "push" and end up emitting events to a context on the output. These compose well. Let me go into just a bit more detail because I think it's interesting. The input tree is represented by a trace of function calls into the transformer, a different function for each node type. Similarly, the output tree is represented by calling functions corresponding to node types on that tree. This pattern works especially well for expand-type transformations, as the transformation logic is especially intuitive - it's just function calls. It's also really clean when the transformation is stateless, otherwise you have the question of how the transformer gets access to state.

The intermediate trees are pretty invisible, but you certainly could visualize them if you wanted to, by putting tracing statements on the functions corresponding to node types for one of the trees.

For one, you do put startGroup and endGroup calls in (these are mostly magically inserted by a compiler plugin, doing source transformation of the original logic). These help the Composer keep track of the key path at all times, which, recall, is an identifier to a specific position in the tree.

And the Composer internally maps that to a *slot,* which can be used to store state as needed. So basically, even though there's not a tree data structure in memory, when you're in the middle of a transformation stage, you can get access to state for your node as *if* the tree was materialized.

I highly recommend Leland Richardson's [Compose From First Principles](http://intelligiblebabble.com/compose-from-first-principles/), as it goes into a fair amount of detail about general tree transformations, and the concept of "positional memoization" as a way to (in the lingo of this blog post) access state of intermediate tree stages in a transformation pipeline.

### makepad

People watching the Rust GUI space, or just GUI in general, should be aware of [makepad]. It is generally fairly similar to imgui, but with its own twists, and also serves as a reminder that the pipeline I sketched above is not set in stone. As with imgui, many of the pipeline stages are fused, so they're not apparent, but intermediate tree structures are encoded as begin/end pairs. The main fused pass is a traversal of the app state (represented as a tree of Rust structs), emitting nodes as a trace to the next stage. One unique feature is that layout doesn't happen before painting (transformation of render objects to draw objects) as in most systems, but rather is interleaved. Another major innovation of makepad is having a separate control flow for events. This is important for fixing a number of the issues with imgui, but for the most part in this post I'm ignoring event. In many systems, once the pipeline is in place for rendering, it's reasonably straightforward to reuse this mechanism to also have event flows in the opposite direction (and this is what imgui tries to do), but it's not rare to have more specialized mechanisms. Makepad also exposes styling via GPU shaders written in Rust syntax, with data flowing nearly transparently from the widget state to the GPU.


And so on for the other major reactive frameworks. I believe that you can analyze most of them and put them on this chart. They're all at heart a sequence of tree transformations, but sometimes the trees are stored, sometimes traced, and lots of details different as discussed above.

### DOM

I do want to say something now about DOM, because that has such a profound impact on JS reactive frameworks, which has been such a productive breeding ground.

Clearly DOM is a very specific tree representation. DOM is intermediate between render object and draw, which is why I named it as an optional stage above. And it's most definitely stored and has a very mutation-focused API for pushing to, with JS language object references being the main identifier. (But note opt-in (hopefully) unique identifier through the `id` attribute and the `getElementById()` API, which are heavily used and particularly optimized)

It *also* represents a major boundary in the system, with JS upstream and C++ downstream. If you're designing one of these systems for scripting, the cross-language boundary is always going to be one of the most important. So this is one reason I consider the current druid architecture poorly suited for scripting.

DOM is also known as a performance problem, to the point where it's seen as perfectly reasonable to have a "virtual DOM" as a preceding pipeline stage, with no actual logic between the two other than diffing, so that the bandwidth of expensive mutation operations to the DOM is reduced.

Obviously most JS reactive frameworks take DOM as a given, and are organized around it. They also tend to gloss over the implicit state in DOM nodes and downstream, pretending that the only state management concerns are in the transformations from app state up to the DOM. But in my thinking, it's just another tree in the pipeline. And of course, when building systems not based on web technology, all those decisions should be questioned.

### Drawing to pixels

So I do want to touch on draw-to-pixels, especially GPU and the prospect of retaining draw subtrees there. The boundary between CPU and GPU is just as sharp as DOM, but I think the same incremental tree transformation paradigm applies, as well as the same basic design decisions. Should updates to the GPU resident draw tree be done using mutations (similar to DOM), or through composition of immutable subtrees, more of a value-type approach? I favor the latter, but these decisions are best made with empirical data on both performance and ease-of-programming.

In addition, if there's a compositor in the mix, it's possible to partially digest the draw tree into subtrees of rendered-to-texture content, and use the same general techniques to manage those trees. From what I've seen, SwiftUI does this extensively (the ultimate render target of a view is a CALayer), in part to interop more nicely with Cocoa. Flutter also has explicit layers which can be rendered into textures and composited at the end of the pipeline. But, though I acknowledge that leveraging the compositor has some advantages (in some common scenarios, it's optimum for power usage), long term I don't see it as a good approach. (A blog post in my backlog is "the compositor is evil")

## Final thoughts

My goal in posting this is to start a conversation. I'm not claiming to have all the answers, but want to learn as much as I can from others working in this space. I've already learned a lot from discussions with Adam Perry, Colin Rofls, and Tristan Hume in particulars (though they are of course not responsible for errors). To a large extent, this exploration is a continuation of the "we don't yet know what to build" theme from my [Rust 2020 blog post]. I think it's possible to be systematic, rather than just trying out a bunch of random stuff.

I'm also curious if there's good academic literature I'm missing. I'm familiar with some, particularly from the [FRP][Breaking down FRP] community, but it's rare to come across something that gives a good picture of exactly what happens from mouse clicks to pixels.

In this post I went into a bit of detail on druid's framework. I plan to [give a talk](https://www.meetup.com/Rust-Bay-Area/events/266571982/) soon, which hopefully will convey the ideas behind that (particularly the approach to lensing), and of course not be so theoretical. In general, I'm looking forward to refining the patterns in druid to make them general and powerful (including building in good escape hatches), and also explaining the design as I go along.

[piet]: https://github.com/linebender/piet
[druid]: https://github.com/xi-editor/druid
[xi-editor]: https://github.com/xi-editor/xi-editor
[makepad]: https://github.com/makepad/makepad
[Model-View-Catharsis]: https://acko.net/blog/model-view-catharsis/
[Breaking down FRP]: https://blog.janestreet.com/breaking-down-frp/
[React as a UI Runtime]: https://overreacted.io/react-as-a-ui-runtime/
[Dear ImGui]: https://github.com/ocornut/imgui
[The Elm Architecture]: https://guide.elm-lang.org/architecture/
[no longer based on FRP]: https://elm-lang.org/news/farewell-to-frp
[property wrappers]: https://mecid.github.io/2019/06/12/understanding-property-wrappers-in-swiftui/
[Lager]: https://sinusoid.es/lager/
[the xi Zulip]: https://xi.zulipchat.com/
[Runebender]: https://github.com/linebender/runebender
[areweguiyet]: https://areweguiyet.com/
[Rust 2020 blog post]: https://raphlinus.github.io/rust/druid/2019/10/31/rust-2020.html
[lens]: https://en.wikibooks.org/wiki/Haskell/Lenses_and_functional_references
