use std::{mem::transmute, simd::*};


use super::{
    u1000::{u64x16_shift_elements_left, u64x16_shift_elements_right},
    u500::*,
};
use num::SimdUint;

#[rustfmt::skip]
const MASK5000: u64x16 = {
    let mut a = [!0;16];
    a[14] = (!0) << 56;
    a[15] = 0 ;
    u64x16::from_array(a)
};

#[rustfmt::skip]
const ONE: [u64x16;5] = {
    let mut a = [0;16];
    a[0] = 1<<63;
    [
        u64x16::from_array(a),
        u64x16::from_array([0;16]),
        u64x16::from_array([0;16]),
        u64x16::from_array([0;16]),
        u64x16::from_array([0;16]),
    ]
};

#[derive(Debug, Default, Clone)]
pub struct U5000([u64x16; 5]); // 64*80 = 5120

impl Bits for U5000 {
    /// from_slice reads 5000 bits from start_pos of b.
    /// b must have multiple of 5000.
    fn from_slice(b: &[u8], start_pos: usize) -> Self {
        let b = &b[start_pos/8..];
        let mut x = U5000([
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 0..64 * 1].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 1..64 * 2].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 2..64 * 3].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 3..64 * 4].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 4..64 * 5].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 5..64 * 6].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 6..64 * 7].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 7..64 * 8].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 8..64 * 9].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_be_bytes(b[64 * 9..64 * 10].try_into().unwrap())).swap_bytes(),
                ])
            },
        ]);

        // 置零最右120bit
        x.0[4] &= MASK5000;
        x
    }

    /// from_slice reads 500 bits from start_pos of b, where b may has
    /// exactly start_pos + 500 bits.
    fn from_slice_exact(b: &[u8], start_pos: usize) -> Self {
        let b = &b[(start_pos - 120) / 8..];

        let x = U5000([
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 0..64 * 1].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 1..64 * 2].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 2..64 * 3].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 3..64 * 4].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 4..64 * 5].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 5..64 * 6].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 6..64 * 7].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 7..64 * 8].try_into().unwrap())).swap_bytes(),
                ])
            },
            unsafe {
                transmute([
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 8..64 * 9].try_into().unwrap())).swap_bytes(),
                    transmute::<u8x64, u64x8>(u8x64::from_le_bytes(b[64 * 9..64 * 10].try_into().unwrap())).swap_bytes(),
                ])
            },
        ]);

        // 对齐
        x.shift_left(120)
    }

    #[inline(always)]
    fn zero() -> Self {
        U5000::default()
    }

    #[inline(always)]
    fn one() -> Self {
        U5000(ONE)
    }

    // just returns the parity of population
    #[inline(always)]
    fn population_parity(&self) -> u8 {
        ((self.0[0] ^ self.0[1] ^ self.0[2] ^ self.0[3] ^ self.0[4]).reduce_xor().count_ones() & 1) as u8
    }

    #[inline(always)]
    fn shift_right(&self, n: usize) -> Self {
        let x = self.shift_elements_right(n / 64);
        let n = (n % 64) as u64;
        if n > 0 {
            let xx = x.shift_elements_right(1);
            U5000([
                (x.0[0] >> n) ^ (xx.0[0] << (64 - n)),
                (x.0[1] >> n) ^ (xx.0[1] << (64 - n)),
                (x.0[2] >> n) ^ (xx.0[2] << (64 - n)),
                (x.0[3] >> n) ^ (xx.0[3] << (64 - n)),
                (x.0[4] >> n) ^ (xx.0[4] << (64 - n)),
            ])
        } else {
            x
        }
    }

    #[inline(always)]
    fn shift_left(&self, n: usize) -> Self {
        let x = self.shift_elements_left(n / 64);
        let n = (n % 64) as u64;
        if n > 0 {
            let xx = x.shift_elements_left(1);
            U5000([
                (x.0[0] << n) ^ (xx.0[0] >> (64 - n)),
                (x.0[1] << n) ^ (xx.0[1] >> (64 - n)),
                (x.0[2] << n) ^ (xx.0[2] >> (64 - n)),
                (x.0[3] << n) ^ (xx.0[3] >> (64 - n)),
                (x.0[4] << n) ^ (xx.0[4] >> (64 - n)),
            ])
        } else {
            x
        }
    }

    #[inline(always)]
    fn xor(&mut self, rhs: &Self) {
        self.0[0] ^= rhs.0[0];
        self.0[1] ^= rhs.0[1];
        self.0[2] ^= rhs.0[2];
        self.0[3] ^= rhs.0[3];
        self.0[4] ^= rhs.0[4];
    }

    #[inline(always)]
    fn or(&mut self, rhs: &Self) {
        self.0[0] |= rhs.0[0];
        self.0[1] |= rhs.0[1];
        self.0[2] |= rhs.0[2];
        self.0[3] |= rhs.0[3];
        self.0[4] |= rhs.0[4];
    }

    #[inline(always)]
    fn and(&mut self, rhs: &Self) {
        self.0[0] &= rhs.0[0];
        self.0[1] &= rhs.0[1];
        self.0[2] &= rhs.0[2];
        self.0[3] &= rhs.0[3];
        self.0[4] &= rhs.0[4];
    }

    #[inline(always)]
    fn set_bit(&mut self, n: usize) {
        self.or(&U5000::one().shift_right(n))
    }
}

impl U5000 {
    // 0 <= n < 64
    #[inline(always)]
    fn shift_elements_left(&self, n: usize) -> Self {
        if n >= 16 * 5 {
            Self::default()
        } else if n >= 16 * 4 {
            U5000([
                u64x16_shift_elements_left(&self.0[4], n - 64),
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
            ])
        } else if n >= 16 * 3 {
            let n = n - 48;
            U5000([
                u64x16_shift_elements_left(&self.0[3], n) | u64x16_shift_elements_right(&self.0[4], 16 - n),
                u64x16_shift_elements_left(&self.0[4], n),
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
            ])
        } else if n >= 16 * 2 {
            let n = n - 32;
            U5000([
                u64x16_shift_elements_left(&self.0[2], n) | u64x16_shift_elements_right(&self.0[3], 16 - n),
                u64x16_shift_elements_left(&self.0[3], n) | u64x16_shift_elements_right(&self.0[4], 16 - n),
                u64x16_shift_elements_left(&self.0[4], n),
                u64x16::default(),
                u64x16::default(),
            ])
        } else if n >= 16 * 1 {
            let n = n - 16;
            U5000([
                u64x16_shift_elements_left(&self.0[1], n) | u64x16_shift_elements_right(&self.0[2], 16 - n),
                u64x16_shift_elements_left(&self.0[2], n) | u64x16_shift_elements_right(&self.0[3], 16 - n),
                u64x16_shift_elements_left(&self.0[3], n) | u64x16_shift_elements_right(&self.0[4], 16 - n),
                u64x16_shift_elements_left(&self.0[4], n),
                u64x16::default(),
            ])
        } else {
            U5000([
                u64x16_shift_elements_left(&self.0[0], n) | u64x16_shift_elements_right(&self.0[1], 16 - n),
                u64x16_shift_elements_left(&self.0[1], n) | u64x16_shift_elements_right(&self.0[2], 16 - n),
                u64x16_shift_elements_left(&self.0[2], n) | u64x16_shift_elements_right(&self.0[3], 16 - n),
                u64x16_shift_elements_left(&self.0[3], n) | u64x16_shift_elements_right(&self.0[4], 16 - n),
                u64x16_shift_elements_left(&self.0[4], n),
            ])
        }
    }

    // 0 <= n < 64
    #[inline(always)]
    fn shift_elements_right(&self, n: usize) -> Self {
        if n >= 16 * 5 {
            Self::default()
        } else if n >= 16 * 4 {
            U5000([
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
                u64x16_shift_elements_right(&self.0[0], n - 64),
            ])
        } else if n >= 16 * 3 {
            let n = n - 48;
            U5000([
                u64x16::default(),
                u64x16::default(),
                u64x16::default(),
                u64x16_shift_elements_right(&self.0[0], n),
                u64x16_shift_elements_right(&self.0[1], n) | u64x16_shift_elements_left(&self.0[0], 16 - n),
            ])
        } else if n >= 16 * 2 {
            let n = n - 32;
            U5000([
                u64x16::default(),
                u64x16::default(),
                u64x16_shift_elements_right(&self.0[0], n),
                u64x16_shift_elements_right(&self.0[1], n) | u64x16_shift_elements_left(&self.0[0], 16 - n),
                u64x16_shift_elements_right(&self.0[2], n) | u64x16_shift_elements_left(&self.0[1], 16 - n),
            ])
        } else if n >= 16 * 1 {
            let n = n - 16;
            U5000([
                u64x16::default(),
                u64x16_shift_elements_right(&self.0[0], n),
                u64x16_shift_elements_right(&self.0[1], n) | u64x16_shift_elements_left(&self.0[0], 16 - n),
                u64x16_shift_elements_right(&self.0[2], n) | u64x16_shift_elements_left(&self.0[1], 16 - n),
                u64x16_shift_elements_right(&self.0[3], n) | u64x16_shift_elements_left(&self.0[2], 16 - n),
            ])
        } else {
            U5000([
                u64x16_shift_elements_right(&self.0[0], n),
                u64x16_shift_elements_right(&self.0[1], n) | u64x16_shift_elements_left(&self.0[0], 16 - n),
                u64x16_shift_elements_right(&self.0[2], n) | u64x16_shift_elements_left(&self.0[1], 16 - n),
                u64x16_shift_elements_right(&self.0[3], n) | u64x16_shift_elements_left(&self.0[2], 16 - n),
                u64x16_shift_elements_right(&self.0[4], n) | u64x16_shift_elements_left(&self.0[3], 16 - n),
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask() {
        for i in 0..64 {
            // println!("{:?}", SHIFT_ELEMENTS_MASK[i]);
            println!("{} => self.0.rotate_elements_left::<{}>(),", i, i);
        }
    }

    #[test]
    fn test_simd() {
        #[rustfmt::skip]
        let mut b = Vec::new();
        for i in 1..1000{
            b.push (i as u8);
        }
        println!("{:02x?}", b);
        println!();

        let v = U5000::from_slice(&b, 4);
        println!("{:02x?}\n", v);

        let v = v.shift_elements_right(4);
        println!("{:02x?}\n", v);

        let v = v.shift_elements_left(4);
        println!("{:02x?}\n", v);

        let v = v.shift_right(12);
        println!("{:02x?}\n", v);

        let v = v.shift_left(12);
        println!("{:02x?}\n", v);
    }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use super::*;
    use test::Bencher;

    #[bench]
    fn bench_popcount(b: &mut Bencher) {
        let buf = [0xab; 512];
        let u = U500::from_slice(&buf, 0);
        b.iter(|| {
            for _ in 0..1000000 {
                test::black_box(u.population_parity());
            }
        });
    }

    #[bench]
    fn bench_shift(b: &mut Bencher) {
        let buf = [0xab; 512];
        let u = U500::from_slice(&buf, 0);

        // 629,663.28 ns/iter
        b.iter(|| {
            for _ in 0..1000000 {
                test::black_box(u.shift_left(100));
            }
        });
    }
}
