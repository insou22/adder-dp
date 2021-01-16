#![feature(iter_advance_by)]
#![forbid(unsafe_code)]

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub struct AtomicBitVec {
    data: Vec<AtomicU64>
}

const fn next_mul_64(v: usize) -> usize {
    (v + 64) & !63
}

impl AtomicBitVec {

    pub fn new() -> Self {
        Self {
            data: Vec::new()
        }
    }

    pub fn with_bit_capacity(bit_cap: usize) -> Self {
        let blocks = next_mul_64(bit_cap) / 64;
        Self::with_capacity(blocks)
    }

    pub fn with_capacity(blocks: usize) -> Self {
        Self {
            data: Vec::with_capacity(blocks)
        }
    }

    pub fn resize_with(&mut self, new_blocks: usize, f: impl FnMut() -> AtomicU64) {
        self.data.resize_with(new_blocks, f)
    }

    pub fn resize_bits_with(&mut self, new_bits: usize, f: impl FnMut() -> AtomicU64) {
        let blocks = next_mul_64(new_bits) / 64;
        self.data.resize_with(blocks, f)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn bit_size(&self) -> usize {
        self.len() * 64
    }

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

    pub fn get(&self, idx: usize, ordering: Ordering) -> bool {
        let (loc, mask) = Self::loc_and_mask(idx);
        let dest: &AtomicU64 = &self.data[loc];
        dest.load(ordering) & mask != 0
    }

    pub fn iter<'a>(&'a self, ordering: Ordering) -> impl Iterator<Item=bool> + 'a {
        BitArrayIter::new(self, ordering)
    }

    const fn loc_and_mask(idx: usize) -> (usize, u64) {
        let mask = 1u64 << (idx & (64 - 1));
        let block = idx >> (64u64.trailing_zeros());
        (block, mask)
    }

    pub fn count_ones(&self, ordering: Ordering) -> usize {
        self.data.iter()
            .map(|n| n.load(ordering).count_ones() as usize)
            .sum()
    }
}

pub struct BitArrayIter<'a> {
    src: &'a AtomicBitVec,
    order: Ordering,
    idx: usize
}

impl <'a> BitArrayIter<'a> {

    pub(crate) fn new(orig: &'a AtomicBitVec, order: Ordering) -> Self {
        Self {
            src: orig,
            order,
            idx: 0
        }
    }
}

impl <'a> Iterator for BitArrayIter<'a> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.src.len() {
            let o = self.src.get(self.idx, self.order);
            self.idx += 1;
            Some(o)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let hint = self.src.len() - self.idx;
        (hint, Some(hint))
    }

    fn advance_by(&mut self, n: usize) -> Result<(), usize> {
        if self.idx + n <= self.src.len() {
            self.idx += n;
            Ok(())
        } else {
            let e = self.src.len() - self.idx;
            self.idx += n;
            Err(e)
        }
    }
}