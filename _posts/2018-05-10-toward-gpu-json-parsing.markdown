---
layout: post
title:  "Towards GPGPU JSON parsing"
date:   2018-05-10 08:37:03 -0700
categories: personal
---
The amount of computing resources available on general purpose GPU hardware is vastly greater than in scalar CPUs. A continuing trend is to move computation from CPU to GPGPU. Some computations (most 3D graphics operations, many machine learning tasks) can be expressed efficiently in terms of primitives that GPUs offer. However, tasks such as JSON parsing are traditionally considered as serial algorithms and are not often implemented on GPU.

I've been thinking about how to apply rope science techniques to this problem, and now believe I have a practical solution. This post sketches my idea at a high level - I haven't implemented it yet and have little hands-on experience with GPU, so who knows what could go wrong?

## The problem

We'll choose a juicy fragment of general JSON: parsing [Dyck languages](https://en.wikipedia.org/wiki/Dyck_language). In this post, we'll concentrate entirely on extracting the tree structure from the input.

The output will be an array representation suitable for both random access and batch processing. Each node in the tree will be stored as a contiguous block in the output array. The first word in the block is the number of children, and the successive words will be the indices of the children (with the root at index 0). This representation is similar to Cap'n Proto, Flat buffers, and FIDL. For reasons that will become clear later, we'll generate this result in [BFS](https://en.wikipedia.org/wiki/Breadth-first_search) order.

As a running example, we'll use the input `[[][[][][[]]][][]]`. The result is:

```
index 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
value 4  5  6 10 11  0  3 12 13 14  0  0  0  0  1 16  0
```

## The primitives

I'm building my parser from three basic primitives: scan, scatter/gather, and sort. All three can be implemented reasonably efficiently on GPGPU.

### Scan

[Scan](https://en.wikipedia.org/wiki/Prefix_sum) is a generalization of prefix sum, potentially based on any binary associative operator. We'll restrict ourselves to operators over small, fixed-size data structures

### Scatter/gather

The "gather" operation is basically texture read. For parsing, we'll rely more heavily on "scatter," which I'll define as follows. There are three inputs (which can be arrays, or can be simple computations over arrays): condition, index, and value. The result is a buffer b, so that for any triple (true, index, value) in the input, b[index] = value. The operation is only well defined when the value for any index is unique, when we use the primitive we'll make sure that there are no duplicate indexes.

Scatter is not necessarily an efficient primitive on GPU, but good techniques exist, see [1] [2].

### Sort

Sorting is one of the most fundamental algorithms. There is extensive literature on efficient GPU implementation.

## The algorithm

We'll present it in as a sequence of passes, using our running example.

### Count nodes, compute nesting depth

The first pass counts the number of nodes and the nesting depth of each node. To count nodes, map `[` to 1 and `]` to 0, and compute (exclusive) prefix sum. Nesting depth is the same, but map `]` to -1.


```
input:  [  [  ]  [  [  ]  [  ]  [  [  ]  ]  ]  [  ]  [  ]  ]
 node:  0  1  2  2  3  4  4  5  5  6  7  7  7  7  8  8  9  9
depth:  0  1  2  1  2  3  2  3  2  3  4  3  2  1  2  1  2  1
```

At this point, we know the number of nodes (9). We can also validate that the brackets are balanced, the result of the depth sum should be 0. Note that for the node and depth calculations we're using _exclusive_ prefix sum; we'll be using both inclusive and exclusive variants depending on the exact needs.

### Reduce to nodes

We only care about nodes, not symbols. Our first scatter will generate the depth of each node. The condition is that the input is `[`, the index is the node count, and the value is the depth. The result:

```
index:  0  1  2  3  4  5  6  7  8
depth:  0  1  1  2  2  2  3  1  1
```

This array fully captures the structure of our tree.

### Sort by depth

Now sort the tree into BFS order. This is basically a stable sort using depth as the primary key. However, instead of actually sorting the array, we record for each node its order in a BFS traversal. (This is equivalent to retaining the index of each element while sorting, then doing the inverse permutation, which can be represented as a scatter, but it's likely that an actual implementation can be more direct).

```
index:  0  1  2  3  4  5  6  7  8
depth:  0  1  1  2  2  2  3  1  1
  bfs:  0  1  2  5  6  7  8  3  4
```

### Determine parents of first children

Now we do a scatter operation, with the goal of associating first children with their parent nodes. The condition is: depth[i + 1] = depth[i] + 1 (this means that node i + 1 is the first child of node i). The key is bfs[i + 1], and the value is bfs[i]:

```
index:  0  1  2  3  4  5  6  7  8
 1par:     0           2        7
```

After this pass, each element of 1par represents a node in BFS order, and holds the index of the parent node if it's a first child.

### Propagate parent links

The next pass is a scan that propagates the parent link to the right. In the same pass, count the number of children.

```
 index:  0  1  2  3  4  5  6  7  8
parent:     0  0  0  0  2  2  2  7
 count:     1  2  3  4  1  2  3  1
```

After this pass, each element of parent represents a node in BFS order, and holds the index of the parent node. This works because, in BFS order, all the siblings (ie all nodes sharing the same parent) are in a contiguous block.

### Scatter child counts

Another scatter. The condition is that parent[i] != parent[i + 1], the index is parent[i], and the value is count[i]. The default is 0 (the scatter only writes a count for nonempty nodes).

```
 index:  0  1  2  3  4  5  6  7  8
parent:     0  0  0  0  2  2  2  7
 count:     1  2  3  4  1  2  3  1
nchild:  4  0  3  0  0  0  0  1  0
```

### Allocate

Now we can assign each node an index in the final output. The number of cells of each node is 1 + the number of children. Do an (exclusive) prefix sum of 1 + nchild[i].

```
 index:  0  1  2  3  4  5  6  7  8
parent:     0  0  0  0  2  2  2  7
nchild:  4  0  3  0  0  0  0  1  0
 alloc:  0  5  6 10 11 12 13 14 16
```

### Generate the result

The final result is two scatters into the same array. For each node i, store nchild[i] into alloc[i], which establishes the size fields. Also store alloc[i] into alloc[parent[i] + count[i]], which establishes the references to the children.

```
index 0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16
value 4  5  6 10 11  0  3 12 13 14  0  0  0  0  1 16  0
```

## Discussion

Traditional parsing of bracket-balanced languages uses a stack to keep track of parent-child relationships. A stack is inherently a sequential data structure that does not lend itself well to implementation on a GPU. We obviate the need for a stack by taking advantage of the fact that the parent/child relationships have a simple structure when the parse tree is sorted into BFS order.

The overall algorithm is 3 scans, 4 scatters, and a sort. Each of these operations should be efficiently implementable on a GPGPU. The most expensive operation is likely the sort; the scans in particular should be quite efficient because work-efficient algorithms are known.

The scan/scatter technique can quite easily be extended to support, for example, backslash unescaping for the strings in the JSON document. It's likely that dictionaries don't present any serious difficulties beyond arrays; the keys can be hashed entirely in parallel, and constructing hash tables in the output can be done with the same kind of scan/scatter.

Is this technique already known? There is literature on parallel parsing for more general grammars [4], but it's not clear to me that these approaches are at all efficient for simple grammars such as those needed to parse JSON (here abstracted to Dyck languages).

Is it really practical? I've been thinking about this at an abstract level, considering the kinds of operations that could be parallelized, but don't know how efficient the scatter and sort operations would be in practice.

Thanks to Rif A. Sauros for comments on an earlier draft.

## References

[0] [nvParse](https://github.com/antonmks/nvParse): Parsing CSV files with GPU

[1] [Implementing Scatter](https://developer.nvidia.com/gpugems/GPUGems2/gpugems2_chapter32.html)

[2] [Efficient Gather and Scatter Operations on Graphics Processors](http://www.cse.ust.hk/catalac/papers/scatter_sc07.pdf)

[3] [AMA: Explaining my 750 line compiler+runtime designed to GPU self-host APL](https://news.ycombinator.com/item?id=13797797)

[4] [Parsing in Parallel on Multiple Cores and GPUs](http://www.aclweb.org/anthology/U11-1006)

[5] [Mison: A Fast JSON Parser for Data Analytics](https://www.microsoft.com/en-us/research/publication/mison-fast-json-parser-data-analytics/)

