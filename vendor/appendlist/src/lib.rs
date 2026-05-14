//! # AppendList: An append-only list that preserves references to its elements
//!
//! [![Build Status](https://travis-ci.com/danieldulaney/appendlist.svg?branch=master)](https://travis-ci.com/danieldulaney/appendlist)
//!
//! This list lets you add new things to the end, even if you're holding a reference
//! to something already inside it. It also avoids reallocations.
//!
//! When should you use it?
//!
//! - You need to insert into a list while its other elements are borrowed
//! - You have so much data that `Vec` reallocations are significant
//!
//! When shouldn't you use it?
//!
//! - You are storing indices into a list, rather than actual references (just use
//!   `Vec<T>`)
//! - `Vec` reallocations don't matter very much (normally the case!)
//!
//! What are some features?
//!
//! - A `push(&self, item: T)` method (you'd expect `push(&mut self, item: T)`)
//! - Non-amortized constant-time insertions and indexes (normally insertions are
//!   *amortized* constant-time, like `Vec`, or indexing is linear-time, like `LinkedList`)
//! - You can hold onto references in the list
//! - Only 2 lines of unsafe code
//!
//! ## What does this let me do?
//!
//! This example fails to compile because `second_element` has a reference into
//! `list` when `list.push()` is called.
//!
//! ```rust compile_fail
//! let list: Vec<u32> = (1..=10).collect();
//!
//! let second_element = &list[1];
//!
//! list.push(11); // Push needs &mut self, but list is already borrowed
//!
//! assert_eq!(*second_element, 2); // Fails to compile
//! ```
//!
//! But if you just swap in `AppendList` for `Vec`, everything works!
//!
//! ```rust
//! use appendlist::AppendList;
//!
//! let list: AppendList<u32> = (1..=10).collect();
//!
//! let second_element = &list[1];
//!
//! list.push(11); // Push only needs &self, so this works fine
//!
//! assert_eq!(*second_element, 2); // All OK!
//! ```
//!
//! ## What's all this about reallocations?
//!
//! In general, `Vec`s are pretty cool, and you should use them by default. But they
//! have a weakness: when you create
//! one, it gets created with a finite amount of space. When it runs out of space,
//! it needs to reallocate: grab a new hunk of memory (usually twice as big as
//! the current one) and copy everything over, then release the old memory.
//!
//! Reallocations take O(n) time to do: you need to copy all n elements in the list.
//! But you don't have to do them very often: only O(log n) reallocations are needed
//! to do n insertions (for the current `Vec` implementation). In fact, the
//! reallocations are rare enough that if you spread
//! them out across all the insertions, they just add a constant extra time.
//! This is why the Rust docs say that `Vec` has "O(1) *amortized*
//! push" -- it generally takes constant time to push and it occasionally takes linear
//! time, but if you spread out those expensive pushes over all the cheap pushes,
//! it's still constant.
//!
//! Reallocations have another issue: any reference to an element inside the `Vec`
//! is invalidated. If you have a reference to one of the elements when it gets
//! copied over, your reference has no way of knowing its new location and will
//! still point to the old location, which is now invalid memory. Using that reference
//! would be a use-after-free bug, so Rust forces you to have no references into a
//! `Vec` before you push another element on, just in case that push would reallocate.
//!
//! The `AppendList` solves both issues by keeping a `Vec` of chunks of data. When
//! you push a new element on, it goes to the end of the current chunk. If the chunk
//! is full, rather than reallocate it, `AppendList` creates a whole new chunk that
//! starts off empty. Each
//! chunk is double the size of the last chunk, so only O(log n) allocations are
//! needed, and each one takes constant time (for most allocators) rather than linear
//! time. By eliminating reallocations, an `AppendList` gives you a couple of benefits
//! compared to `Vec`:
//!
//! - You can keep your references while pushing
//! - You don't have to pay the occasional linear reallocate-and-copy cost
//!
//! However, it also comes with some drawbacks:
//!
//! - Indexing takes longer (you have to index into a chunk, then into your item)
//! - CPU cache behavior might be somewhat worse near a chunk boundary (because the
//!   next chunk isn't generally contiguous)
//!
//! ## So should I use this crate?
//!
//! Probably not.
//!
//! In general, you should just use a `Vec` and keep track of indices rather than
//! references. But if keeping references is very important, then this is your solution.

use std::cell::{Cell, UnsafeCell};
use std::fmt::{self, Debug};
use std::iter::FromIterator;
use std::ops::Index;

// Must be a power of 2
const FIRST_CHUNK_SIZE: usize = 16;

/// A list that can be appended to while elements are borrowed
///
/// This looks like a fairly bare-bones list API, except that it has a `push`
/// method that works on non-`mut` lists. It is safe to hold references to
/// values inside this list and push a new value onto the end.
///
/// Additionally, the list has O(1) index and O(1) push (not amortized!).
///
/// For example, this would be illegal with a `Vec`:
///
/// ```
/// use appendlist::AppendList;
///
/// let list = AppendList::new();
///
/// list.push(1);
/// let first_item = &list[0];
/// list.push(2);
/// let second_item = &list[1];
///
/// assert_eq!(*first_item, list[0]);
/// assert_eq!(*second_item, list[1]);
/// ```
///
/// # Implementation details
///
/// This section is not necessary to use the API, it just describes the underlying
/// allocation and indexing strategies.
///
/// The list is a `Vec` of *chunks*. Each chunk is itself a `Vec<T>`. The list
/// will fill up a chunk, then allocate a new chunk with its full capacity.
/// Because the capacity of a given chunk never changes, the underlying `Vec<T>`
/// never reallocates, so references to that chunk are never invalidated. Each
/// chunk is twice the size of the previous chunk, so there will never be more
/// than O(log(n)) chunks.
///
/// Constant-time indexing is achieved because the chunk ID of a particular index
/// can be quickly calculated: if the first chunk has size c, index i will be
/// located in chunk floor(log2(i + c) - log2(c)). If c is a power of 2, this
/// is equivalent to floor(log2(i + c)) - floor(log2(c)), and a very fast floor
/// log2 algorithm can be derived from `usize::leading_zeros()`.
pub struct AppendList<T> {
    chunks: UnsafeCell<Vec<Vec<T>>>,
    len: Cell<usize>,
}

impl<T> AppendList<T> {
    /// Wrapper to get the list of chunks immutably
    fn chunks(&self) -> &[Vec<T>] {
        unsafe { &*self.chunks.get() }
    }

    /// In test builds, check all of the unsafe invariants
    ///
    /// In release builds, no-op
    fn check_invariants(&self) {
        #[cfg(test)]
        {
            if self.len.get() > 0 {
                // Correct number of chunks
                assert_eq!(index_chunk(self.len.get() - 1), self.chunks().len() - 1);

                // Every chunk holds enough items
                for chunk_id in 0..self.chunks().len() {
                    assert!(chunk_size(chunk_id) <= self.chunks()[chunk_id].capacity());
                }

                // Intermediate chunks are full
                for chunk_id in 0..self.chunks().len() - 1 {
                    assert_eq!(chunk_size(chunk_id), self.chunks()[chunk_id].len());
                }

                // Last chunk is correct length
                assert_eq!(
                    self.chunks().last().unwrap().len(),
                    self.len.get() - chunk_start(self.chunks().len() - 1)
                );
            } else {
                // No chunks
                assert_eq!(0, self.chunks().len());
            }
        }
    }

    /// Create a new `AppendList`
    pub fn new() -> Self {
        Self {
            chunks: UnsafeCell::new(Vec::new()),
            len: Cell::new(0),
        }
    }

    /// Append an item to the end
    ///
    /// Note that this does not require `mut`.
    pub fn push(&self, item: T) {
        self.check_invariants();

        // Unsafe code alert!
        //
        // Preserve the following invariants:
        // - Only the last chunk may be modified
        // - A chunk cannot ever be reallocated
        // - len must reflect the length
        //
        // Invariants are checked in the check_invariants method
        let mut_chunks = unsafe { &mut *self.chunks.get() };

        let new_index = self.len.get();
        let chunk_id = index_chunk(new_index);

        if chunk_id < mut_chunks.len() {
            // We should always be inserting into the last chunk
            debug_assert_eq!(chunk_id, mut_chunks.len() - 1);

            // Insert into the appropriate chunk
            let chunk = &mut mut_chunks[chunk_id];

            // The chunk must not be reallocated! Save the pre-insertion capacity
            // so we can check it later (debug builds only)
            #[cfg(test)]
            let prev_capacity = chunk.capacity();

            // Do the insertion
            chunk.push(item);

            // Check that the capacity didn't change (debug builds only)
            #[cfg(test)]
            assert_eq!(prev_capacity, chunk.capacity());
        } else {
            // Need to allocate a new chunk

            // New chunk should be the immediate next chunk
            debug_assert_eq!(chunk_id, mut_chunks.len());

            // New chunk must be big enough
            let mut new_chunk = Vec::with_capacity(chunk_size(chunk_id));
            debug_assert!(new_chunk.capacity() >= chunk_size(chunk_id));

            new_chunk.push(item);

            mut_chunks.push(new_chunk);
        }

        // Increment the length
        self.len.set(self.len.get() + 1);

        self.check_invariants();
    }

    /// Get the length of the list
    pub fn len(&self) -> usize {
        self.check_invariants();

        self.len.get()
    }

    /// Get an item from the list, if it is in bounds
    ///
    /// Returns `None` if the `index` is out-of-bounds. Note that you can also
    /// index with `[]`, which will panic on out-of-bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.check_invariants();

        if index >= self.len() {
            return None;
        }

        let chunk_id = index_chunk(index);
        let chunk_start = chunk_start(chunk_id);

        return Some(&self.chunks()[chunk_id][index - chunk_start]);
    }

    /// Get an iterator over the list
    pub fn iter(&self) -> Iter<T> {
        self.check_invariants();

        Iter {
            list: &self,
            index: 0,
        }
    }
}

const fn chunk_size(chunk_id: usize) -> usize {
    // First chunk is FIRST_CHUNK_SIZE, subsequent chunks double each time
    FIRST_CHUNK_SIZE << chunk_id
}

const fn chunk_start(chunk_id: usize) -> usize {
    // This looks like magic, but I promise it works
    // Essentially, each chunk is the size of the sum of all chunks before
    // it. Except that the first chunk is different: it "should" be preceded
    // by a whole list of chunks that sum to its size, but it's not. Therefore,
    // there's a "missing" set of chunks the size of the first chunk, so
    // later chunks need to be updated.
    chunk_size(chunk_id) - FIRST_CHUNK_SIZE
}

const fn index_chunk(index: usize) -> usize {
    // This *is* magic
    floor_log2(index + FIRST_CHUNK_SIZE) - floor_log2(FIRST_CHUNK_SIZE)
}

#[inline]
const fn floor_log2(x: usize) -> usize {
    const BITS_PER_BYTE: usize = 8;

    BITS_PER_BYTE * std::mem::size_of::<usize>() - (x.leading_zeros() as usize) - 1
}

impl<T> Default for AppendList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<usize> for AppendList<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index)
            .expect("AppendList indexed beyond its length")
    }
}

impl<T> FromIterator<T> for AppendList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let list = Self::new();

        for item in iter {
            list.push(item);
        }

        list
    }
}

impl<'l, T> IntoIterator for &'l AppendList<T> {
    type Item = &'l T;
    type IntoIter = Iter<'l, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: PartialEq> PartialEq for AppendList<T> {
    fn eq(&self, other: &AppendList<T>) -> bool {
        let mut s = self.iter();
        let mut o = other.iter();

        loop {
            match (s.next(), o.next()) {
                (Some(a), Some(b)) if a == b => {},
                (None, None) => return true,
                _ => return false,
            }
        }
    }
}

impl<T: Debug> Debug for AppendList<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_list().entries(self.iter()).finish()
    }
}

pub struct Iter<'l, T> {
    list: &'l AppendList<T>,
    index: usize,
}

impl<'l, T> Iterator for Iter<'l, T> {
    type Item = &'l T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.list.get(self.index);

        self.index += 1;

        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.list.len() - self.index;

        (remaining, Some(remaining))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_iterator() {
        let l: AppendList<i32> = (0..100).collect();

        for i in 0..100 {
            assert_eq!(l[i], i as i32);
        }
    }

    #[test]
    fn iterator() {
        let l: AppendList<i32> = (0..100).collect();
        let mut i1 = l.iter();
        let mut i2 = l.into_iter();

        for item in 0..100 {
            assert_eq!(i1.next(), Some(&item));
            assert_eq!(i2.next(), Some(&item));
        }

        assert_eq!(i1.next(), None);
        assert_eq!(i2.next(), None);
    }

    #[test]
    fn equality() {
        let a = AppendList::new();
        let b = AppendList::new();

        assert_eq!(a, b);

        a.push("foo");

        assert_ne!(a, b);

        b.push("foo");

        assert_eq!(a, b);

        a.push("bar");
        a.push("baz");

        assert_ne!(a, b);
    }

    #[test]
    fn iterator_size_hint() {
        let l: AppendList<i32> = AppendList::new();
        let mut i = l.iter();
        assert_eq!(i.size_hint(), (0, Some(0)));

        l.push(1);
        assert_eq!(i.size_hint(), (1, Some(1)));

        l.push(2);
        assert_eq!(i.size_hint(), (2, Some(2)));

        i.next();
        assert_eq!(i.size_hint(), (1, Some(1)));

        l.push(3);
        assert_eq!(i.size_hint(), (2, Some(2)));

        i.next();
        assert_eq!(i.size_hint(), (1, Some(1)));

        i.next();
        assert_eq!(i.size_hint(), (0, Some(0)));
    }

    #[test]
    fn chunk_sizes_make_sense() {
        assert_eq!(chunk_size(0), FIRST_CHUNK_SIZE);

        let mut index = 0;

        for chunk in 0..20 {
            // Each chunk starts just after the previous one ends
            assert_eq!(chunk_start(chunk), index);
            index += chunk_size(chunk);
        }
    }

    #[test]
    fn index_chunk_matches_up() {
        for index in 0..1_000_000 {
            let chunk_id = index_chunk(index);

            // Each index happens after its chunk start and before its chunk end
            assert!(index >= chunk_start(chunk_id));
            assert!(index < chunk_start(chunk_id) + chunk_size(chunk_id));
        }
    }

    #[test]
    fn empty_list() {
        let n: AppendList<usize> = AppendList::new();

        assert_eq!(n.len(), 0);
        assert_eq!(n.get(0), None);

        let d: AppendList<usize> = AppendList::default();

        assert_eq!(d.len(), 0);
        assert_eq!(d.get(0), None);
    }

    #[test]
    fn thousand_item_list() {
        test_big_list(1_000);
    }

    #[test]
    #[ignore]
    fn million_item_list() {
        test_big_list(1_000_000);
    }

    fn test_big_list(size: usize) {
        let l = AppendList::new();
        let mut refs = Vec::new();

        for i in 0..size {
            assert_eq!(l.len(), i);

            l.push(i);
            refs.push(l[i]);

            assert_eq!(l.len(), i + 1);
        }

        for i in 0..size {
            assert_eq!(Some(&refs[i]), l.get(i));
        }
    }
}
