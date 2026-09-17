use crate::util::bitfield::BitField;

pub(crate) struct BloomField {
    bits: BitField,
}

impl BloomField {
    pub fn init(bit_count: usize) -> Self {
        BloomField {
            bits: BitField::new(bit_count)
        }
    }
    
    // TODO: This
}
