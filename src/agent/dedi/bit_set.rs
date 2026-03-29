use std::fmt;
use std::ops::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Bitset192 {
    pub a: u64, // bits   0..63
    pub b: u64, // bits  64..127
    pub c: u64, // bits 128..191
}

impl Bitset192 {
    pub fn new(a: u64, b: u64, c: u64) -> Self {
        Self { a, b, c }
    }

    #[inline]
    pub fn get_bit(&self, idx: usize) -> bool {
        match idx {
            0..=63 => (self.a >> idx) & 1 != 0,
            64..=127 => (self.b >> (idx - 64)) & 1 != 0,
            128..=191 => (self.c >> (idx - 128)) & 1 != 0,
            _ => panic!("bit index {} out of range (0..191)", idx),
        }
    }

    #[inline]
    pub fn set_bit(&mut self, idx: usize) {
        match idx {
            0..=63 => self.a |= 1u64 << idx,
            64..=127 => self.b |= 1u64 << (idx - 64),
            128..=191 => self.c |= 1u64 << (idx - 128),
            _ => panic!("bit index {} out of range (0..191)", idx),
        }
    }

    #[inline]
    pub fn clear_bit(&mut self, idx: usize) {
        match idx {
            0..=63 => self.a &= !(1u64 << idx),
            64..=127 => self.b &= !(1u64 << (idx - 64)),
            128..=191 => self.c &= !(1u64 << (idx - 128)),
            _ => panic!("bit index {} out of range (0..191)", idx),
        }
    }

    #[inline]
    pub fn any(&self) -> bool {
        self.a != 0 || self.b != 0 || self.c != 0
    }
}

impl BitAnd for Bitset192 {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self {
            a: self.a & rhs.a,
            b: self.b & rhs.b,
            c: self.c & rhs.c,
        }
    }
}

impl BitAndAssign for Bitset192 {
    fn bitand_assign(&mut self, rhs: Self) {
        self.a &= rhs.a;
        self.b &= rhs.b;
        self.c &= rhs.c;
    }
}

impl BitOr for Bitset192 {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self {
            a: self.a | rhs.a,
            b: self.b | rhs.b,
            c: self.c | rhs.c,
        }
    }
}

impl BitOrAssign for Bitset192 {
    fn bitor_assign(&mut self, rhs: Self) {
        self.a |= rhs.a;
        self.b |= rhs.b;
        self.c |= rhs.c;
    }
}

impl BitXor for Bitset192 {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        Self {
            a: self.a ^ rhs.a,
            b: self.b ^ rhs.b,
            c: self.c ^ rhs.c,
        }
    }
}

impl BitXorAssign for Bitset192 {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.a ^= rhs.a;
        self.b ^= rhs.b;
        self.c ^= rhs.c;
    }
}

impl Not for Bitset192 {
    type Output = Self;
    fn not(self) -> Self {
        Self {
            a: !self.a,
            b: !self.b,
            c: !self.c,
        }
    }
}

impl Bitset192 {
    pub fn iter_ones(&self) -> impl Iterator<Item = usize> {
        let mut a = self.a;
        let mut b = self.b;
        let mut c = self.c;

        std::iter::from_fn(move || {
            if a != 0 {
                let tz = a.trailing_zeros() as usize;
                a &= a - 1;
                return Some(tz);
            }

            if b != 0 {
                let tz = b.trailing_zeros() as usize;
                b &= b - 1;
                return Some(64 + tz);
            }

            if c != 0 {
                let tz = c.trailing_zeros() as usize;
                c &= c - 1;
                return Some(128 + tz);
            }

            None
        })
    }
}

impl fmt::Debug for Bitset192 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format c (highest bits), then b, then a (lowest bits)
        let mut out = String::with_capacity(192);

        for i in (0..64).rev() {
            out.push(if (self.c >> i) & 1 == 1 { '1' } else { '0' });
        }
        out.push('_');
        for i in (0..64).rev() {
            out.push(if (self.b >> i) & 1 == 1 { '1' } else { '0' });
        }
        out.push('_');
        for i in (0..64).rev() {
            out.push(if (self.a >> i) & 1 == 1 { '1' } else { '0' });
        }

        write!(f, "Bitset192({})", out)
    }
}
