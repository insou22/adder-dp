// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


#![feature(iter_advance_by)]
#![forbid(unsafe_code)]
#![warn(missing_docs, broken_intra_doc_links)]

//! This library provides a bitvec struct which uses atomic integers as its backing representation.
//!
//! This allows the bitvec to be used without external synchronization, though the perils
//! of improper use of atomics can come into play.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::num::NonZero;

/// AtomicBitVec is build atop a standard [`Vec`], and uses [`AtomicU64`] for its backing store.
/// The ordering for atomic operations is left to the user to decide.
///
/// The term "blocks" is used throughout this documentation to refer to the number of atomic
/// integers are stored in the backing storage. All resizing and allocation is done in block-sized
/// units; this means that the bit-length of these bitvecs will *always* be a multiple of 64.
pub struct AtomicBitVec {
    data: Vec<AtomicU64>
}

const fn next_mul_64(v: usize) -> usize {
    (v + 64) & !63
}

impl AtomicBitVec {
    /// Creates an empty [`AtomicBitVec`].
    ///
    /// This does not allocate; you'll need to call one of [`with_bit_capacity`], [`with_capacity`],
    /// [`resize_blocks_with`], or [`resize_bits_with`] to actually allocate memory and initialize
    /// the backing store.
    ///
    /// [`with_bit_capacity`]: #method.with_bit_capacity
    /// [`with_capacity`]: #method.with_capacity
    /// [`resize_blocks_with`]: #method.resize_blocks_with
    /// [`resize_bits_with`]: #method.resize_bits_with
    ///
    /// # Examples
    /// Basic usage:
    /// ```
    /// use atomic_bitvec::AtomicBitVec;
    /// let s = AtomicBitVec::new();
    /// ```
    pub const fn new() -> Self {
        Self {
            data: Vec::new()
        }
    }

    /// Returns the size of this bitvec in memory in bytes.
    ///
    /// This value is calculated from the size of the allocated backing store and the size of the
    /// vector itself. This does not take into account potential reserve overhead; it is based
    /// purely on the current length of the bitvec.
    pub fn size_in_mem(&self) -> usize {
        std::mem::size_of::<Vec<AtomicU64>>() + self.data.len() * std::mem::size_of::<AtomicU64>()
    }

    /// Creates a new bitvec with capacity to hold at least `bit_cap` many bits.
    ///
    /// This implementation will allocate as many bits as is necessary to hold a multiple of 64 bits.
    pub fn with_bit_capacity(bit_cap: usize) -> Self {
        let blocks = next_mul_64(bit_cap) / 64;
        Self::with_capacity(blocks)
    }

    /// Creates a new bitvec with capacity to hold at least `blocks` many blocks.
    ///
    /// Each block holds 64 bits.
    pub fn with_capacity(blocks: usize) -> Self {
        Self {
            data: Vec::with_capacity(blocks)
        }
    }

    /// Resizes a bitvec to contain `new_blocks` many blocks, using `f` to generate new elements if
    /// extending the bitvec. If `new_blocks` is less than [`block_cnt`], this truncates instead.
    ///
    /// [`block_cnt`]: #method.block_cnt
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::AtomicU64;
    /// let mut s = AtomicBitVec::with_capacity(2);
    /// assert_eq!(s.block_cnt(), 0);
    /// s.resize_blocks_with(4, AtomicU64::default);
    /// assert_eq!(s.block_cnt(), 4);
    /// ```
    pub fn resize_blocks_with(&mut self, new_blocks: usize, f: impl FnMut() -> AtomicU64) {
        self.data.resize_with(new_blocks, f)
    }

    /// Resizes a bitvec to contain at least `new_bits` many bits, using `f` to generate new blocks if
    /// extending the bitvec. If `new_bits` is less than [`len`], this truncates instead.
    ///
    /// This will extend the bitvec to the next multiple of 64 bits if `new_bits` is not a multiple of 64.
    ///
    /// [`len`]: #method.len
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::AtomicU64;
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// assert_eq!(s.len(), 0);
    /// s.resize_bits_with(200, AtomicU64::default);
    /// // Note that the next multiple of 64 bits was allocated.
    /// assert_eq!(s.block_cnt(), 4);
    /// assert_eq!(s.len(), 256);
    /// ```
    pub fn resize_bits_with(&mut self, new_bits: usize, f: impl FnMut() -> AtomicU64) {
        let blocks = next_mul_64(new_bits) / 64;
        self.data.resize_with(blocks, f)
    }

    /// Returns the current block count of the bitvec. This is equivalent to the bit-length
    /// of the bitvec divided by 64.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::AtomicU64;
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(200, AtomicU64::default);
    /// assert_eq!(s.block_cnt(), 4);
    /// ```
    pub fn block_cnt(&self) -> usize {
        self.data.len()
    }

    /// Returns the current bit-length of the bitvec. This is equivalent to the current block count
    /// times 64.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::AtomicU64;
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(200, AtomicU64::default);
    /// // Note that the next multiple of 64 bits was allocated.
    /// assert_eq!(s.len(), 256);
    /// ```
    pub fn len(&self) -> usize {
        self.block_cnt() * 64
    }

    /// Sets the bit at `idx` to `value`, using the atomic ordering provided by `ordering`.
    /// Returns the previous value at the specified bit.
    ///
    /// The bit will be set atomically, allowing this bitvec to be used from multiple threads.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(256, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// assert!(s.get(3, Ordering::Acquire));
    /// ```
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds.
    pub fn set(&self, idx: usize, value: bool, ordering: Ordering) -> bool {
        let (loc, mask) = Self::loc_and_mask(idx);
        let dest: &AtomicU64 = &self.data[loc];
        if value {
            let prev = dest.fetch_or(mask, ordering);
            prev & mask != 0
        } else {
            let unset_mask = !mask;
            let prev = dest.fetch_and(unset_mask, ordering);
            prev & mask != 0
        }
    }

    /// Returns the bit at the specified index according to the given atomic ordering.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(256, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// assert!(s.get(3, Ordering::Acquire));
    /// ```
    ///
    /// # Panics
    /// Panics if `idx` is out of bounds or if `ordering` is not valid for [`AtomicU64::load`]
    pub fn get(&self, idx: usize, ordering: Ordering) -> bool {
        let (loc, mask) = Self::loc_and_mask(idx);
        let dest: &AtomicU64 = &self.data[loc];
        dest.load(ordering) & mask != 0
    }

    /// Returns an iterator over the bits of this bitvec.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(64, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// let i = s.iter(Ordering::Acquire);
    /// let v: Vec<bool> = i.take(5).collect();
    /// assert_eq!(v, [false, false, false, true, false]);
    /// ```
    /// # Panics
    /// Panics if `ordering` is not valid for [`AtomicU64::load`]
    /// # Warning
    /// Because this struct can be updated atomically, if this function is called while other threads
    /// are updating this bitvec, the result may not be equivalent to if this function had been called
    /// when this thread had unique ownership.
    /// ```no_run
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// # use std::sync::Arc;
    ///
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(64, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// let a = Arc::new(s);
    /// let ta = a.clone();
    /// # let h =
    /// std::thread::spawn(move || ta.set(4, true, Ordering::AcqRel));
    /// let i = a.iter(Ordering::Acquire);
    /// let v: Vec<bool> = i.take(5).collect();
    /// assert_eq!(v, [false, false, false, true, false]); // May or may not panic!
    /// # h.join().unwrap();
    /// ```
    pub fn iter<'a>(&'a self, ordering: Ordering) -> impl Iterator<Item=bool> + 'a {
        Iter::new(self, ordering)
    }

    const fn loc_and_mask(idx: usize) -> (usize, u64) {
        let mask = 1u64 << (idx & (64 - 1));
        let block = idx >> (64u64.trailing_zeros());
        (block, mask)
    }

    /// Counts all of the set bits in this bitvec.
    ///
    /// # Examples
    /// ```
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(64, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// s.set(5, true, Ordering::AcqRel);
    /// assert_eq!(s.count_ones(Ordering::Acquire), 2);
    /// ```
    /// # Panics
    /// Panics if `ordering` is not valid for [`AtomicU64::load`]
    ///
    /// # Warning
    /// Because this struct can be updated atomically, if this function is called while other threads
    /// are updating this bitvec, the result may not be equivalent to if this function had been called
    /// when this thread had unique ownership.
    /// ```no_run
    /// # use atomic_bitvec::AtomicBitVec;
    /// # use std::sync::atomic::{AtomicU64, Ordering};
    /// # use std::sync::Arc;
    /// let mut s = AtomicBitVec::with_bit_capacity(128);
    /// s.resize_bits_with(64, AtomicU64::default);
    /// s.set(3, true, Ordering::AcqRel);
    /// s.set(5, true, Ordering::AcqRel);
    /// let a = Arc::new(s);
    /// let ta = a.clone();
    /// # let h =
    /// std::thread::spawn(move || ta.set(5, false, Ordering::AcqRel));
    /// assert_eq!(a.count_ones(Ordering::Acquire), 2); // May or may not panic!
    /// # h.join().unwrap();
    /// ```
    pub fn count_ones(&self, ordering: Ordering) -> u64 {
        self.data.iter()
            .map(|n| n.load(ordering).count_ones() as u64)
            .sum()
    }
}

/// The iterator for an [`AtomicBitVec`]. This implementation pulls double duty as the struct
/// used for [`Iterator`] and [`IntoIterator`].
pub struct Iter<'a, Inner> where Inner: Borrow<AtomicBitVec> + 'a {
    src: Inner,
    order: Ordering,
    idx: usize,
    back_idx: usize,
    phony: PhantomData<&'a AtomicBitVec>,
}

impl<'a, Inner> Iter<'a, Inner> where Inner: Borrow<AtomicBitVec> + 'a {
    pub(crate) fn src(&self) -> &AtomicBitVec {
        self.src.borrow()
    }
}

impl<'a> Iter<'a, &'a AtomicBitVec> {
    pub(crate) fn new(orig: &'a AtomicBitVec, order: Ordering) -> Self {
        let bit_size = orig.len();
        Self {
            src: orig,
            order,
            idx: 0,
            back_idx: bit_size,
            phony: PhantomData::default(),
        }
    }
}

impl IntoIterator for AtomicBitVec {
    type Item = bool;
    type IntoIter = Iter<'static, AtomicBitVec>;

    fn into_iter(self) -> Self::IntoIter {
        let bs = self.len();
        Iter {
            src: self,
            order: Ordering::Acquire,
            idx: 0,
            back_idx: bs,
            phony: Default::default(),
        }
    }
}

impl<'a, Inner> Iterator for Iter<'a, Inner> where Inner: Borrow<AtomicBitVec> + 'a {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.back_idx {
            let o = self.src().get(self.idx, self.order);
            self.idx += 1;
            Some(o)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let hint = self.back_idx - self.idx;
        (hint, Some(hint))
    }

    fn advance_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        if self.idx + n <= self.back_idx {
            self.idx += n;
            Ok(())
        } else {
            let e = NonZero::new(self.back_idx - self.idx).unwrap();
            self.idx += n;
            Err(e)
        }
    }
}

impl<'a, Inner> ExactSizeIterator for Iter<'a, Inner> where Inner: Borrow<AtomicBitVec> + 'a {}

impl<'a, Inner> DoubleEndedIterator for Iter<'a, Inner> where Inner: Borrow<AtomicBitVec> + 'a {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.idx < self.back_idx {
            let o = self.src().get(self.back_idx - 1, self.order);
            self.back_idx = self.back_idx.saturating_sub(1);
            Some(o)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static_assertions::assert_impl_all!(AtomicBitVec: Sync);
}
