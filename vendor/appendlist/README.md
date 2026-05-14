# AppendList: An append-only list that preserves references to its elements

[![Build Status](https://travis-ci.com/danieldulaney/appendlist.svg?branch=master)](https://travis-ci.com/danieldulaney/appendlist)

This list lets you add new things to the end, even if you're holding a reference
to something already inside it. It also avoids reallocations.

When should you use it?

- You need to insert into a list while its other elements are borrowed
- You have so much data that `Vec` reallocations are significant

When shouldn't you use it?

- You are storing indices into a list, rather than actual references (just use
  `Vec<T>`)
- `Vec` reallocations don't matter very much (normally the case!)

What are some features?

- A `push(&self, item: T)` method (you'd expect `push(&mut self, item: T)`)
- Non-amortized constant-time insertions and indexes (normally insertions are
  *amortized* constant-time, like `Vec`, or indexing is linear-time, like `LinkedList`)
- You can hold onto references in the list
- Only 2 lines of unsafe code

## What does this let me do?

This example fails to compile because `second_element` has a reference into
`list` when `list.push()` is called.

```rust compile_fail
let list: Vec<u32> = (1..=10).collect();

let second_element = &list[1];

list.push(11); // Push needs &mut self, but list is already borrowed

assert_eq!(*second_element, 2); // Fails to compile
```

But if you just swap in `AppendList` for `Vec`, everything works!

```rust
use appendlist::AppendList;

let list: AppendList<u32> = (1..=10).collect();

let second_element = &list[1];

list.push(11); // Push only needs &self, so this works fine

assert_eq!(*second_element, 2); // All OK!
```

## What's all this about reallocations?

In general, `Vec`s are pretty cool, and you should use them by default. But they
have a weakness: when you create
one, it gets created with a finite amount of space. When it runs out of space,
it needs to reallocate: grab a new hunk of memory (usually twice as big as
the current one) and copy everything over, then release the old memory.

Reallocations take O(n) time to do: you need to copy all n elements in the list.
But you don't have to do them very often: only O(log n) reallocations are needed
to do n insertions (for the current `Vec` implementation). In fact, the
reallocations are rare enough that if you spread
them out across all the insertions, they just add a constant extra time.
This is why the Rust docs say that `Vec` has "O(1) *amortized*
push" -- it generally takes constant time to push and it occasionally takes linear
time, but if you spread out those expensive pushes over all the cheap pushes,
it's still constant.

Reallocations have another issue: any reference to an element inside the `Vec`
is invalidated. If you have a reference to one of the elements when it gets
copied over, your reference has no way of knowing its new location and will
still point to the old location, which is now invalid memory. Using that reference
would be a use-after-free bug, so Rust forces you to have no references into a
`Vec` before you push another element on, just in case that push would reallocate.

The `AppendList` solves both issues by keeping a `Vec` of chunks of data. When
you push a new element on, it goes to the end of the current chunk. If the chunk
is full, rather than reallocate it, `AppendList` creates a whole new chunk that
starts off empty. Each
chunk is double the size of the last chunk, so only O(log n) allocations are
needed, and each one takes constant time (for most allocators) rather than linear
time. By eliminating reallocations, an `AppendList` gives you a couple of benefits
compared to `Vec`:

- You can keep your references while pushing
- You don't have to pay the occasional linear reallocate-and-copy cost

However, it also comes with some drawbacks:

- Indexing takes longer (you have to index into a chunk, then into your item)
- CPU cache behavior might be somewhat worse near a chunk boundary (because the
  next chunk isn't generally contiguous)

## So should I use this crate?

Probably not.

In general, you should just use a `Vec` and keep track of indices rather than
references. But if keeping references is very important, then this is your solution.
