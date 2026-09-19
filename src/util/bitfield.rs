use std::ops::Div;
use serde::{Deserialize, Serialize};

static BIT_COUNT: usize = 8;

#[derive(Serialize, Deserialize)]
pub(crate) struct BitField {
    bits: Vec<u8>,
}

impl BitField {
    pub fn new(bit_count: usize) -> Self {
        let mut bits = Vec::new();
        bits.resize(bit_count.div_ceil(BIT_COUNT), 0);
        BitField {
            bits,
        }
    }

    pub fn get(&self, bit: usize) -> bool {
        self.bits[bit / BIT_COUNT] & (1 << (bit % BIT_COUNT)) as u8 != 0
    }

    pub fn set(&mut self, bit: usize, val: bool) {
        if val {
            self.bits[bit / BIT_COUNT] |= 1 << (bit % BIT_COUNT);
        } else {
            self.bits[bit / BIT_COUNT] &= !(1 << (bit % BIT_COUNT));
        }
    }

    pub fn up(&mut self, bit: usize) {
        self.set(bit, true);
    }

    pub fn down(&mut self, bit: usize) {
        self.set(bit, false);
    }

    pub fn bit_count(&self) -> usize {
        self.bits.len() * BIT_COUNT
    }
}

#[cfg(test)]
mod tests {
    use crate::util::bitfield::BitField;

    #[test]
    fn test_correct_size() {
        assert_eq!(BitField::new(9).bits.len(), 2);
        assert_eq!(BitField::new(8).bits.len(), 1);
        assert_eq!(BitField::new( 7).bits.len(), 1);
        assert_eq!(BitField::new(16).bits.len(), 2);
        assert_eq!(BitField::new(17).bits.len(), 3);
    }

    #[test]
    fn test_get_set() {
        let mut bf = BitField::new(16);
        bf.set(0, true);
        bf.set(3, true);
        bf.set(4, true);
        bf.set(10, true);
        bf.set(15, true);

        assert_eq!(bf.get(0), true);
        assert_eq!(bf.get(1), false);
        assert_eq!(bf.get(2), false);
        assert_eq!(bf.get(3), true);
        assert_eq!(bf.get(4), true);
        assert_eq!(bf.get(5), false);
        assert_eq!(bf.get(6), false);
        assert_eq!(bf.get(7), false);
        assert_eq!(bf.get(8), false);
        assert_eq!(bf.get(9), false);
        assert_eq!(bf.get(10), true);
        assert_eq!(bf.get(11), false);
        assert_eq!(bf.get(12), false);
        assert_eq!(bf.get(13), false);
        assert_eq!(bf.get(14), false);
        assert_eq!(bf.get(15), true);
    }
}
