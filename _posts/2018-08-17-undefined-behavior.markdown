---
layout: post
title:  "With Undefined Behavior, Anything is Possible"
date:   2018-08-17 11:51:03 -0700
categories: [programming, rust]
---
![anything is possible](/assets/Anything_is_Possible_scaled.jpg)

Undefined behavior contributes to many serious problems, including security vulnerabilities. It’s also, I believe, poorly understood, and discussions of it tend to become contentious. How did we get here? What are the best ways to deal with it? Is it a good thing or a bad thing, and if the latter, is it even possible to get rid of it? To address these questions will require digging a bit into history.

## Unportable, Semi-portable, and Standard C

C programmers are not a homogeneous community. To understand undefined behavior, I divide them into three camps. The principles and practices in one camp can seem alien, even threatening to another.

### Unportable C: a step up from assembler

In the “unportable C” camp, the ultimate goal is to ship a binary, usually for a single piece of hardware. The C compiler is a tool to help you get to that binary faster. It makes sense to exploit any extensions offered by the compiler. In the event of any confusion about the semantics of the language, the binary (the output of the compiler) is the ultimate source of truth.

The computation model is very much the same as assembly language, with details matching those of the target hardware. Pointers are just integers (of a known word size) used in a particular way, integer arithmetic wraps reliably, etc.

Unportable use of C is increasingly rare, but used to be common practice. An extreme example of unportable C is the book _Mastering C Pointers: Tools for Programming Power,_ which was [castigated](https://wozniak.ca/blog/2018/06/25/Massacring-C-Pointers/index.html) recently. To be fair, that book has other flaws rather than being in a different camp, but I think that fuels some of the intensity of passion against it.

### Semi-portable C: the triumph of the #ifdef

In many cases, you don’t want to target a single machine, but maybe a range of them, and a range of operating systems, compilers, and other factors. These machines may be diverse; pointer size can be 16, 32, or 64 bits, and both little and big endian are possible. There’s going to be some form of configuration mechanism (often [autoconf](https://www.gnu.org/software/autoconf/autoconf.html) in the Unix-y world) that populates preprocessor symbols so that #ifdef directives can choose the right alternative code for that target.

Use of compiler-specific extensions is to be looked at with suspicion (though in many cases it might make sense to turn them on with #ifdef if it’ll help with performance, debugging, etc.). It makes sense to write code with at least an eye out for portability, rather than making assumptions about the specific target machine.

The computation model is pretty much the same as the unportable case, just that the details might vary. In particular, [type punning](https://blog.regehr.org/archives/959) is fine, as long as care is taken to make sure sizes line up. Sometimes you can get bitten, for example x86 is much more forgiving of unaligned accesses than RISC; sometimes they can cause a serious performance problem, other times they might just crash. Similarly, shifting past bitwidth might do different things on different machines (maybe shifting in all zeros, maybe shifting an amount modulo the bitwidth), but always something reasonable.

Semi-portable C used to be the mainstream, but is now giving way to standard C in many contexts. But, as we’ll see, there are still (at least partial) hold-outs. When people describe C as “[a portable assembly language](https://stackoverflow.com/questions/3040276/when-did-people-first-start-thinking-c-is-portable-assembler)” it’s pretty much the semi-portable camp they’re referring to.

### Standard C: a compromise

Faced with the above situation, the standards committee had a daunting task: come up with a version of C that could reasonably be implemented by all compilers. It had to be close enough to existing C that it wouldn’t be too difficult or expensive to migrate existing code, but at the same time there was an opportunity to improve the language, in particular to [clean up](http://ee.hawaii.edu/~tep/EE160/Book/chapapx/node7.html) some of the lack of discipline about function parameter types.

In so doing, the committee had to converge on a computational model that would somehow encompass all targets. This turned out to be quite difficult, because there were a lot of targets out there that would be considered strange and exotic; arithmetic is not even guaranteed to be twos complement (the alternative is [ones complement](https://superuser.com/questions/1137182/is-there-any-existing-cpu-implementation-which-uses-ones-complement)), [word sizes might not be a power of 2[(https://retrocomputing.stackexchange.com/questions/4419/how-was-the-c-language-ported-to-architectures-with-non-power-of-2-word-sizes), and other things.

C requires rigorous attention to correct use of pointers, to avoid [use-after-free](https://www.purehacking.com/blog/lloyd-simon/an-introduction-to-use-after-free-vulnerabilities), [double-free](https://www.owasp.org/index.php/Double_Free), [out-of-bounds access](https://cwe.mitre.org/data/definitions/125.html), and other similar memory errors. Any of those can cause symptoms ranging from crashes to subtle memory corruption to silently incorrect results, and very likely different results on different machines. The standards committee invented the concept of “undefined behavior” to capture this range of behavior. Essentially, it’s a license for the implementation to do anything it wants. And that’s reasonable; it’s hard to imagine nailing down the behavior any further without compromising performance or the fundamental nature of the problem.

But given this hammer, the committee applied it far more broadly. For example, shift-past-bitwidth is also considered undefined behavior. Many have argued persuasively that it would have been better to treat this particular case as “implementation defined,” so a programmer would be able to count on, for example, always getting the same result for the same inputs on the same chip (though it might be different on a different chip, like endianness). However, that’s not what they decided. Instead, computing `x << 64` is allowed to crash, subtly corrupt memory, or connect to a server to transfer money out of your account. That last is not a joke (along the lines of [nasal demons](http://www.catb.org/jargon/html/N/nasal-demons.html)); undefined behavior is the source of many serious security vulnerabilities, and arithmetic issues (including shifting but especially integer overflow) a respectable subset of those.

![anything is possible](/assets/unicorndraft1.jpg)

Indeed, there is a very large catalog of potential undefined behaviors: signed integer overflow, reading uninitialized memory, computing (not just dereferencing!) an out-of-bounds pointer, type punning through pointers, etc. I won’t try to give an exhaustive catalog here (John Regehr’s [guide](https://blog.regehr.org/archives/213) is an excellent introduction), but the point is that it cast such a wide net that essentially all extant programs ran into one or another form of it.

In other words, the standard basically broke all existing programs, in the sense that don’t work in the new category of “strictly conforming”. Perhaps the committee felt that programmers could fairly easily clean up the flaws in their programs, much the way they had to change argument syntax, but if so they massively underestimated the task.

Most of the remainder of this post is dedicated to the implications and consequences of such an expansive definition of undefined behavior

## Pointers are complicated

Undefined behavior is not just for capturing the variation between implementations; another major motivation is to enable optimizations that would otherwise be difficult. One of the trickiest areas is [strict aliasing](https://blog.regehr.org/archives/1307).

Here’s a quick sketch of the motivation. Many optimizations depend on “aliasing analysis,” essentially guarantees that pointers don’t alias, or more generally that ranges of memory don’t overlap. In general, alias analysis is intractable. However, in a program that strongly respects typing rules, two values of different types _can’t_ overlap. Standard C basically makes the assumption that programs do respect types, therefore two pointers of two different types can’t possibly overlap. Back in the day, lots of people had trouble coming to grips with that, hence slogans such as “Not all of the world is a VAX.”

And it “enforces” this by declaring that any such usage of pointers is undefined behavior. This can’t be done with a simple model where pointers are just numbers. The best way to understand the actual C standard is that programs run on an exotic, complex virtual machine in which pointers are numbers, yes, but annotated with types and valid ranges. Any usage of a pointer that doesn’t strictly follow the rules is immediately undefined behavior.

This is the true computational model of standard C. What makes the situation so deceptive is that this complex virtual machine can be easily run on standard hardware, just by stripping out the extra stuff. What’s easy to forget sometimes is that the compiler is allowed to do much more complicated things, and often does so in service of optimization.

Understanding the actual rules is not easy. For an excellent recent discussion digging into more detail, see [Pointers Are Complicated, or: What's in a Byte?](https://www.ralfj.de/blog/2018/07/24/pointers-and-bytes.html). And for a research paper going into great detail (including evidence that both LLVM and GCC miscompile some programs that technically follow the standard), see [Reconciling High-level Optimizations and Low-level Code in LLVM](http://sf.snu.ac.kr/llvmtwin/).

## A radical change, in slow motion

C has a reputation for being a stable, mature language. However, the transition to standard C was anything but stable; it broke almost all programs, and in deep, fundamental ways that are difficult to identify. If compilers had actually generated crashing code for all undefined behavior, as allowed in the standard, there would have been open revolt.

But compilers didn’t change much at all in early days. Undefined behavior was a convenient way for existing implementations to claim compliance with the standard. Programs routinely violated the letter of the standard, but when you compiled them, they _worked._ In practice, everybody was in the semi-portable camp. To use the language of the standard, compilers were “conforming.”

However, compiler authors got bolder over time, feeling that everything allowed in the standard was fair game, at the same time getting more sophisticated in what optimizations could be done with stronger assumptions. This, of course, had the effect of taking all those programs that were broken in theory and making them broken in practice. Understandably, programmers in the semi-portable camp blamed compiler authors in the standard camp for being overly aggressive, optimizing too much. A passionate and recent example is the [rant from Linus Torvalds](https://lkml.org/lkml/2018/6/5/769) critiquing a patch to remove union-based aliasing.

Similarly, people in the standard camp (likely, the authors of that patch) view any code that introduces the potential for undefined behavior as dangerous, frequently invoking language of contamination and uncleanliness. From this perspective, it’s not surprising to see [discussions](https://news.ycombinator.com/item?id=17262643) get contentious.

## Threads, memory models, and expressivity

Threads are increasingly important, not least because multi-core CPU’s are now ubiquitous. However, they interact in complex ways with languages (like C) that are fundamentally sequential. The prevailing approach at the time of C89 is that thread primitives (such as [pthreads](https://en.wikipedia.org/wiki/POSIX_Threads)) would be provided as a library, and that their behavior would be described entirely in semi-portable concepts.

A corollary is that the strict “standard C” language was much less expressive than the semi-portable dialog, not being able to represent threaded programs at all. Again, this wasn’t too much of a problem in practice

The situation improved considerably with C11, building off the many years of work to arrive at the C++11 memory model (which in turn was quite inspired by the [Java memory model](https://en.wikipedia.org/wiki/Java_memory_model), arguably the first successful example of such a thing). Actually understanding these memory models is [complicated](https://arxiv.org/pdf/1803.04432.pdf), but at least now we can say that the standard dialect has regained much of the expressivity it lost.

It’s worth asking: what is the gap in expressivity between semi-portable and standard C today? Much of it is implementation of concepts from much higher level programming languages, such as the [Boehm-Demers-Weiser garbage collector](http://www.hboehm.info/gc/), which relies heavily on semi-portable constructs. Similarly, tail recursion and coroutines are popular programming language features that cannot be expressed readily in standard C. Thus, many systems (including many that embed a scripting language) must rely on semi-portable C. The standards committee dream of all programs strictly conforming to the standard is likely a long ways off, if ever.

## What is to be done?

Undefined behavior is a mess. There have been [proposals for a friendly dialect](https://blog.regehr.org/archives/1180) removing some of the more egregious examples, but ultimately they [failed](https://blog.regehr.org/archives/1287), just not being able to get consensus from all the people involved. Compiler writers really like the freedom that aggressive undefined behavior gives them to optimize, and are reluctant to cede any ground that might impact performance. It’s entirely possible that future editions of C will revert some of the most egregious choices, but I suspect that the situation won’t change much.

One of the reasons why undefined behavior is so insidious is that it’s so difficult to tell if a program exhibits it or not. It’s very common for it to lie dormant in a codebase for years, until a compiler upgrade triggers it. Fortunately, tools are emerging, for example the [undefined behavior sanitizer](https://clang.llvm.org/docs/UndefinedBehaviorSanitizer.html) from LLVM. Of course, such tools can only detect when a particular run of a program triggers undefined behavior; since so much of the problem is hard-to-trigger behavior from certain (malicious) inputs, these tools also work well in conjunction with fuzzers. Learning and setting up these tools is not easy, but it’s a necessary cost of writing software in C or C++ and having any hope of escaping the problems caused by undefined behavior.

A word of caution, however. As pointed out by [Undefined Behavior in 2017](https://blog.regehr.org/archives/1520), these sanitizers only go so far. Follow that link to read lots more detail about how to mitigate undefined behavior in real systems.

Commenting on an earlier draft, Thomas Lord writes:
 
> I think probably many or most working programmers using C are probably doing it wrong. Not that there is only one true way — but.
> 
> If one's interest is in safe, portable code – C can be a very fine choice. One must use it well, though. C should be regarded as a target language. I mean this more broadly than people usually do.
> 
> C’s sharp edges can be managed safely two ways, at least:
> 
> One is through careful use of well-designed coding standards. Large program authors should make key architectural decisions very early on, define a safe, constrained, style – and have the team stick to that style.
> 
> In some situations, additionally: code generation tools are appropriate. Those can be hybrids that mix C fragments with other stuff (like lex and yacc) – or higher level languages entirely.

## A problem with C alone?

I’ve been focusing on C, but a good question is the degree to which undefined behavior affects other languages. Java, for example, almost completely avoids it (with the exception of JNI, about which more below). Rust has one of my favorite approaches: the safe dialect is free of undefined behavior, but using unsafe makes the language both potentially unsafe (and more expressive), at the cost of potential undefined behavior. There is a [project underway](https://www.ralfj.de/blog/2017/07/14/undefined-behavior.html) to document the rules carefully so the careful programmer can have both.

Most other languages fall into what I call the “safe-ish” category. The most common compromise is that sequential code has little or no undefined behavior, but that data races can trigger it. It’s very hard to protect against those. Among other things, it used to be thought that data races could be classified into “benign” and dangerous categories, but [research](http://hboehm.info/boehm-hotpar11.pdf) strongly suggests that the former category doesn’t exist.

Perhaps the biggest persistent problem is that C underlies almost all runtimes (it’s possible to build a system based on some language other than C at the lowest levels, but it would be considered quite exotic), and that [FFI](https://en.wikipedia.org/wiki/Foreign_function_interface) is almost always needed to stitch these lower levels with higher level languages, even in cases where the higher level language is designed to be rigorously safe.

Also note that “modern C++” when properly applied avoids many of the memory corruption issues common to C, but is still subject to integer overflow, as well as more subtle forms such as [iterator invalidation](https://stackoverflow.com/questions/6438086/iterator-invalidation-rules). My personal feeling is that careful use of modern C++, and other efforts such as the [core guidelines](https://github.com/isocpp/CppCoreGuidelines), can reduce undefined behavior but can’t provide anywhere nearly the same guarantees as a truly safe language.

## Been there, done that, got the t-shirt

If you’ve read this far, now I can reveal that the real point of this post is to flog my [undefined behavior t-shirt](https://teespring.com/undefined-behavior-shirt#pid=369&cid=6521&sid=front), with the rainbow and unicorn artwork from the top of the page. It’s a good way to indicate to your friends and colleagues that you appreciate the finer points of undefined behavior, plus I’d like to think it’s colorful and fun. All profits go to Amnesty International.

## Further reading:

* [Pointers Are Complicated, or: What's in a Byte?](https://www.ralfj.de/blog/2018/07/24/pointers-and-bytes.html)

* [A Guide to Undefined Behavior in C and C++, Part 1](https://blog.regehr.org/archives/213)

* [Proposal for a Friendly Dialect of C](https://blog.regehr.org/archives/1180)

* [The Problem with Friendly C](https://blog.regehr.org/archives/1287)

* [Reconciling High-level Optimizations and Low-level Code in LLVM](http://sf.snu.ac.kr/llvmtwin/)

## Artwork credits

Top image by [dbeast32](https://www.fiverr.com/dbeast32). Space unicorn by [chrislove95](https://www.fiverr.com/chrislove95).
