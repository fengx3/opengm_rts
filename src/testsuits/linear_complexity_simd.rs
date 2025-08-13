use crate::{
    igamc, powi,
    u500::Bits,
    Sample, TestResult,
};

/// 线性复杂度检测
/// m = 500
#[cfg_attr(target_arch="aarch64", target_feature(enable = "neon"))]
#[cfg_attr(target_arch="x86_64", target_feature(enable = "avx2"))]
pub(crate) fn linear_complexity_simd<T: Bits, const M: usize>(
    sample: &Sample,
) -> TestResult {
    let mut nu = [0; 7];
    let pi = [
        0.010417, 0.03125, 0.12500, 0.50000, 0.25000, 0.06250, 0.020833,
    ];

    #[allow(non_upper_case_globals)]
    let n = sample.bit_length;

    let N = n / M;
    let b = &sample.b;
    let sign = if M % 2 == 0 { 1.0 } else { -1.0 }; // -1^m
    let mean = M as f64 / 2.0 + (9.0 - sign) / 36.0
        - 1.0 / powi(2.0, M as i32) * (M as f64 / 3.0 + 2.0 / 9.0);

    for i in 0..N {
        // 取出 500bit
        let S = if i < N - 1 {
            T::from_slice(b, i * M)
        } else {
            // FIXME: only works for m = 500
            T::from_slice_exact(b, i * M)
        };

        let L = berlekamp_massey_simd::<T, M>(S) as f64;
        let t = sign * (L - mean) + 2.0 / 9.0;

        if t <= -2.5 {
            nu[0] += 1;
        } else if t > -2.5 && t <= -1.5 {
            nu[1] += 1;
        } else if t > -1.5 && t <= -0.5 {
            nu[2] += 1;
        } else if t > -0.5 && t <= 0.5 {
            nu[3] += 1;
        } else if t > 0.5 && t <= 1.5 {
            nu[4] += 1;
        } else if t > 1.5 && t <= 2.5 {
            nu[5] += 1;
        } else {
            nu[6] += 1;
        }
    }
    let mut chi2 = 0.00;
    for (v, p) in core::iter::zip(nu, pi) {
        let np = N as f64 * p;
        chi2 += powi(v as f64 - np, 2) / np;
    }
    let pv = igamc(3.0, chi2 / 2.0);
    TestResult { pv, qv: pv }
}

//
// set c = c + b*D^e
#[inline(always)]
fn add_shift<T: Bits>(c: &mut T, b: &T, e: usize) {
    c.xor(&b.shift_left(e))
}

// // Find L(S) where S start from pos leadingzeros, with nbits bits.
// // For m = 500
#[inline]
fn berlekamp_massey_simd<T: Bits, const M: usize>(s: T) -> usize {
    // let length = 500;
    let mut C = T::default(); // C is in reverse order, right aligned.
    let mut B = T::default();
    let mut T ;

    let mut L = 0;
    let mut m: i32 = -1;

    // C[D] = 1
    C.set_bit(M - 1);
    B.set_bit(M - 1);

    let mut N = 0;
    while N < M {
        // let d = discrepancy(&S, leadingzeros, &C, N, L);
        let d = discrepancy::<T, M>(&s, &C, N as usize);
        if d == 1 {
            // T(D) = C(D)
            T = C.clone();

            // C(D) = C(D) + B(D)*D^{N-m}
            let e = (N as i32 - m) as usize;
            add_shift(&mut C, &B, e);

            if L <= N / 2 {
                L = N + 1 - L;
                m = N as i32;
                B = T.clone();
            }
        }
        N += 1;
    }
    L
}

// d = C_0S_N + C_1S_{N-1} + ... + C_LS_{N-L}
#[inline(always)]
fn discrepancy<T: Bits, const M: usize>(S: &T, C: &T, N: usize) -> u8 {
    let mut x = S.shift_right(M - 1 - N);
    x.and(C);
    x.population_parity()
}

#[cfg(test)]
mod tests {

    use crate::{
        test_data::E, testsuits::linear_complexity_epsilon, u1000::U1000, u500::U500, u5000::U5000
    };

    use super::*;

    #[test]
    fn test_add_shift() {
        // let c: Vec<i32> = vec![0, 0, 1];
        // let b = vec![0, 0, 3];
        // let e = 127;
        // u8x16_add_shift_long(&mut c, &b, e);
        // println!("{:#?}", c);
    }

    #[test]
    fn test_e() {
        let sample: Sample = E.into();
        let l2 = linear_complexity_epsilon(&sample, 500);
        let l1 = unsafe { linear_complexity_simd::<U500, 500>(&sample) };
        assert_eq!(l1, l2);

        let l1 = unsafe { linear_complexity_simd::<U1000, 1000>(&sample) };
        let l2 = linear_complexity_epsilon(&sample, 1000);
        assert_eq!(l1, l2);

        let l1 = unsafe { linear_complexity_simd::<U5000, 5000>(&sample) };
        let l2 = linear_complexity_epsilon(&sample, 5000);
        assert_eq!(l1, l2);
    }

    // #[test]
    // fn test_equal() {
    //     let sample: Sample = E.into();
    //     for param in [500, 1000, 100, 200, 301] {
    //         let res = linear_complexity(&sample, param);
    //         assert_eq!(res, linear_complexity_epsilon(&sample, param));
    //     }
    // }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use super::*;
    use crate::{popcount_u64, test_data::E, u1000::U1000, u500::U500, u5000::U5000, Sample};
    use test::Bencher;

    #[bench]
    fn bench_linear_complexity_simd500(b: &mut Bencher) {
        let sample: Sample = E.into();

        // m=500: 12,160,983.30 ns/iter
        b.iter(|| {
            test::black_box(unsafe { linear_complexity_simd::<U500, 500>(&sample) });
        });
    }

    #[bench]
    fn bench_linear_complexity_simd1000(b: &mut Bencher) {
        let sample: Sample = E.into();

        // m=1000: 16,527,279.10
        b.iter(|| {
            test::black_box(unsafe { linear_complexity_simd::<U1000, 1000>(&sample) });
        });
    }

    #[bench]
    fn bench_linear_complexity_simd5000(b: &mut Bencher) {
        let sample: Sample = E.into();

        // m=5000: 77,473,004.20
        b.iter(|| {
            test::black_box(unsafe { linear_complexity_simd::<U5000, 5000>(&sample) });
        });
    }


    #[bench]
    fn bench_popcount(b: &mut Bencher) {
        let sample: Sample = E.into();

        // m=500: 15,323,787.50
        // m=1000:20,563,904.20
        b.iter(|| {
            test::black_box(popcount_u64(&sample.b64));
        });
    }
}
