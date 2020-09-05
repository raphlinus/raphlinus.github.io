---
layout: post
title:  "The stack monoid"
date:   2020-09-05 08:14:42 -0700
categories: [gpu]
---
This is a bit of a followup to [Towards GPGPU JSON parsing]. That proposed a rather roundabout way to parallelize a simple parsing task. Having had more GPU programming experience under my belt, I don't expect that particular approach to work well, but it did suggest that parallelism exists in the problem.

This post is a writeup of a new idea, but with a caution, no implementation. It probably contains some mistakes, and maybe the idea is flawed. But if it holds up, I think it's an exciting line of research on how to port sequential algorithms to GPU.

For this post, I'm going to pose an even more simplified version of the problem: for each open bracket, record the index of the parent in the parse tree, and for each close bracket, the index of the corresponding open bracket.

This is a simple sequential program:

```python
    stack = [None]
    for i, token in input:
        result[i] = stack[len(stack) - 1]
        if token == '[':
            stack.push(i)
        elif token == ']':
            stack.pop()
```

To follow the running example from the JSON post, assume the input `[[][[][][[]]][][]]`. The result is:

```
index 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17
input [  [  ]  [  [  ]  [  ]  [  [  ]  ]  ]  [  ]  [  ]  ]
value -  0  1  0  3  4  3  6  3  8  9  8  3  0 13  0 15  0
```

Can it be parallelized? It's challenging to see how, as there are dependencies on previous state. Further, the state itself has unbounded size. It can be O(n); a pathological case is a million open brackets followed by a million close. In practice, depending on the workloads, we might expect more modest nesting depth, but ideally we'd like an algorithm that can handle all cases at least reasonably well.

Maybe we can do something. To longtime readers of my blog (and the "rope science" series before that), it will be no surprise I'll try to use monoids.

So let's turn this sequential program into a monoid. If I had a time portal, I'd lens forward in time and use the handy automated tool that some enterprising PhD student will has done by then. But, failing that, I'll do it in my head.

The monoid is basically a sequence of pops followed by a sequence of pushes. Since the pop operations (in this case) don't have a payload, they can be represented simply as a count. So the monoid is the pair `(n_pops, elements)`. The primitive for push is `(0, [element])`, and for pop it's `(1, [])`. The monoid operation is:

```python
def combine(m1, m2):
    n1, s1 = m1
    n2, s2 = m2
    if n2 >= len(s1):
        return (n1 + n2 - len(s1), s2)
    else:
        return (n1, s1[:len(s1) - n2] + s2)
```

Running scan, or generalized [prefix sum], on this monoid, will result in the desired result at the top of the stack, i.e. the last element of the sequence in the monoid.

**Exercise:** Show that this monoid is associative.

I do believe that an automated tool for translating these kinds of simple sequential programs is possible, and that there's a theory behind it that illuminates ways in which monoids do and do not compose. Some composition is possible, for example you could fairly easily extend this idea so that each child knows its sequence number relative to its siblings. To do that, blend in the "counting monoid," which is just integer addition.

**Exercise:** Extend the monoid to handle sequence numbers.

If we had a bound on stack depth, we'd be pretty well set. The problem is the unbounded case. My original thinking was to have a window of (let's say size k) elements, and each pass would handle a slice of the stack of size k. I think this can be made to work, but at heart it requires a number of passes proportional to stack depth, so in the worst case O(n^2).

My new idea is to retain the single pass scan, but use a purpose-built data structure to represent the stack. At the top is a window of k elements, then after that is a linked list (or perhaps a linked list of chunks; honestly I haven't worked through all the implications, much less actually implemented and done performance measurement).

The combine operation tries to fit the new combined sequence in the window (up to a shared link, which it can just retain). But if it overflows, then it must allocate, most likely using an atomic bump allocator into global memory, as is often done in piet-gpu.

## Performance analysis

To do a *real* performance analysis would require an implementation, but it should be possible to reason about it a bit.

Clearly performance will be excellent if stack depth is shallow. The rate at which the linked list will be invoked will depend on the workload, and also the parallelism patterns. It's worth noting that if the scan is run sequentially, then the linked list operations will always be cheap. This is because the concatenation at the heart of the monoid operation is *asymmetrical* when using a linked list - a large sequence on the left is cheap when the sequence on the right is small, but not the other way around. When running the scan purely sequentially, the monoid on the right is always of size 1 at most.

And the decoupled lookback implementation of scan, which is state of the art, runs essentially sequentially at large granularity; it just tries to exploit as much parallelism as the hardware provides at smaller granularity. Even with extremly deep nesting, it should build and retain a linked list of the stack as it scans, with a relatively smaller window for the part of the problem currently in flight.

There is another reason to be optimistic about performance. While the bandwidth of communication between partitions scales with the window size, that's not necessarily true for processing *within* a partition. Putting aside for a moment the relatively challenging problem of implementing an efficient GPU kernel (which by necessity would exploit SIMD parallelism), consider a decoupled look-back implementation running on a more traditional multicore CPU, where each thread is sequential.

Basically, the first pass runs the traditional, sequential stack algorithm, with the modification that some of the input stack might not be available. At the end is the monoid, with "n_pops" being a count of the number of times that happened, and the sequence containing all the elements pushed inside that partition.

In the decoupled lookback phase, this monoid is published for consumption of partitions to the right, and the partition also computes the aggregate from partitions to the left. The cost of the monoid operations in this phase *will* be proportional to the size of the window, but keep in mind that the number of aggregates is orders of magnitude less than the number of elements.

Finally, in the second pass over the elements, the stack algorithm is run again, and this time elements that were missing in the first pass are available, from the aggregate obtained during the decoupled lookback phase. As another potential optimization, the second pass might "jump over" subtrees that were computed entirely locally within the partition in the first pass, only doing work on subtrees that cross partition boundaries.

Thus, my analysis is that the work factor is *extremely* good for this algorithm, meaning that on a 64 core processor, it might come within striking distance of 64 times faster than a single core sequential implementation (obviously modulo all the usual other factors that make this challenging).

As I mentioned, an efficient GPU kernel is likely to be challenging, probably requiring tricky techniques to exploit SIMD (warp) parallelism without spending too much time computing the monoid composition, but the fact that it seems to work so well in the multicore case is an encouraging sign that an efficient, fully GPU-tuned implementation is possible.

## Conclusions

I am even more convinced than before that efficient parsing is possible on GPU. The "stack monoid" shows promise to be a fundamental building block to represent the parse stack, and in general to manipulate tree structured data. I am unaware of any presentation of this, though it's likely it exists somewhere in the literature.

Automatically generating monoids such as the stack monoid from their corresponding simple sequential programs seems like an extremely promising area for research. If I were a professor (I'm not) and a PhD student brought it to me as a proposal, I would be quite excited.

Part of the motivation for this exploration is to represent the "clip stack" in piet-gpu, which has turned out to be a bit of a tricky problem; I'm likely to compute at least some of that on CPU as a preprocessing step. But it would be appealing, and very much in the spirit of piet-gpu, to do all of that computation on the GPU, and even more so if there are no artificial limits on nesting depth. If I do implement that, it will no doubt be another blog post (including quantitative measurements).

But in the meantime, I'm interested to see if other people take up these ideas. My JSON post has gotten a steady trickle of interest since it was published. I'd also like to learn if there's literature that's been published but I just haven't seen yet. If not, it seems like a very rich vein of ore to mine.

(As a personal note, I'm taking a break from Twitter, possibly a very long one, as I'm finding that social media has been really sapping my energy. The best way to get in touch with me for followup is email. I do love hearing from people though!)

[Towards GPGPU JSON parsing]: https://raphlinus.github.io/personal/2018/05/10/toward-gpu-json-parsing.html
[prefix sum]: https://raphlinus.github.io/gpu/2020/04/30/prefix-sum.html
