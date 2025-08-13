use std::sync::Arc;

use super::{super::USE_U8, util::*};
use crate::{Sample, TestResult};

use realfft::{RealFftPlanner, RealToComplex};
use rustfft::num_complex::{Complex32, ComplexFloat};
use std::sync::{LazyLock, Mutex};

// a Vec<T> memory pool
// 内存最多会占用 O(线程数 * 测试数据).
struct VecPool(Vec<(Vec<f32>, Vec<Complex32>)>);
impl VecPool {
    #[inline]
    const fn new() -> Self {
        VecPool(Vec::new())
        // VecPool(Vec::with_capacity(16))
    }
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
    #[inline]
    fn pop(&mut self) -> Option<(Vec<f32>, Vec<Complex32>)> {
        self.0.pop()
    }
    #[inline]
    fn push(&mut self, v: (Vec<f32>, Vec<Complex32>)) {
        self.0.push(v)
    }
}

static VEC_POOL: Mutex<VecPool> = Mutex::new(VecPool::new());

// static VEC_POOL: LazyLock<Mutex<VecPool>> =
//     LazyLock::new(|| Mutex::new(VecPool::new()));

static FFT_PLANNER: LazyLock<Mutex<RealFftPlanner<f32>>> = LazyLock::new(|| Mutex::new(RealFftPlanner::new()));

// get vec form pool, the return vec's length is not guaranteed.
#[inline]
fn get_vec(n: usize) -> (Vec<f32>, Vec<Complex32>) {
    if let Ok(mut p) = VEC_POOL.try_lock() {
        if p.len() > 0 {
            return p.pop().unwrap();
        }
    }
    (Vec::with_capacity(n), Vec::with_capacity(n / 2 + 1))
}

#[inline]
fn put_vec(v: (Vec<f32>, Vec<Complex32>)) {
    if let Ok(mut p) = VEC_POOL.lock() {
        p.push(v);
    }
}

#[inline]
fn get_fft(n: usize) -> Arc<dyn RealToComplex<f32>> {
    FFT_PLANNER.lock().unwrap().plan_fft_forward(n)
}

/// 离散傅里叶检测
pub(crate) fn discrete_fourier(sample: &Sample) -> TestResult {
    if USE_U8 {
        discrete_fourier_u8(sample)
    } else {
        discrete_fourier_u64(sample)
    }
}

#[cfg(test)]
pub(crate) fn discrete_fourier_epsilon(sample: &Sample) -> TestResult {
    use rustfft::num_complex::Complex64;

    let e = &sample.e;
    let n = e.len();
    let mut f = Vec::with_capacity(n);

    // use complex to complex fft.
    for i in 0..n {
        f.push(Complex64 {
            re: (2 * e[i] as i8 - 1) as f64,
            im: 0.0,
        });
    }

    rustfft::FftPlanner::new().plan_fft_forward(f.len()).process(&mut f);
    let t = sqrt(2.995732274 * n as f64);
    let n0 = 0.95 * n as f64 / 2.0;
    let n1 = f[0..=(n / 2 - 1)].iter().map(|x| if x.abs() < t { 1 } else { 0 }).sum::<usize>();

    let d = (n1 as f64 - n0) / sqrt(n as f64 * 0.95 * 0.05 / 3.8);
    let pv = erfc(abs(d) / sqrt(2.0));
    let qv = erfc(d / sqrt(2.0)) / 2.0;

    TestResult { pv, qv }
}

const BITS_PARTTEN: [[f32; 8]; 256] = {
    let mut b = [[0.0; 8]; 256];
    let mut i = 0;
    while i < 256 {
        let mut j = 0;
        while j < 8 {
            b[i][j] = ((i >> (7 - j)) & 1) as f32 * 2.0 - 1.0;
            j += 1;
        }
        i += 1;
    }
    b
};

/// 离散傅里叶检测
/// Note: we perform the FFT for e_i, not x_i = 2*e_i - 1,
/// Then the result will be f'_j = \sum_k e_k exp(-2*pi*i*j*k) = \sum_k (x_i+1)/2 exp(-2*pi*i*j*k)
/// = (\sum_k x_i exp(-2*pi*i*j*k))/2 = f_j/2, except for j = 0, where f_0 = \sum(e_i) * 2 - n
pub(crate) fn discrete_fourier_u8(sample: &Sample) -> TestResult {
    let n = sample.len();

    let (mut e, mut f) = get_vec(n);
    e.clear();

    for b in &sample.b {
        e.extend_from_slice(&BITS_PARTTEN[*b as usize]);
    }

    f.resize(n / 2 + 1, Complex32::default());

    // SAFTY: process returns error only if e and f have wrong length.
    get_fft(e.len()).process(&mut e, &mut f).unwrap();

    let t = sqrt(2.995732274 * n as f64);
    let n0 = 0.95 * n as f64 / 2.0;
    let n1 = f[0..=(n / 2 - 1)]
        .iter()
        // .map(|x| if ((x.abs() as f64)) < t { 1 } else { 0 })
        .map(|x| if ((x.re * x.re + x.im * x.im) as f64) < t * t { 1 } else { 0 })
        .sum::<i64>();

    let d = (n1 as f64 - n0) / sqrt(n as f64 * 0.95 * 0.05 / 3.8);
    let pv = erfc(abs(d) / sqrt(2.0));
    let qv = erfc(d / sqrt(2.0)) / 2.0;

    put_vec((e, f));
    TestResult { pv, qv }
}

/// 离散傅里叶检测
pub(crate) fn discrete_fourier_u64(sample: &Sample) -> TestResult {
    let n = sample.len();
    let mut e = Vec::with_capacity(n);
    for b in sample.b64[..sample.b64.len() - 1].iter() {
        let x = *b;
        for i in (0..64).rev() {
            e.push(((x >> i) & 1) as f32);
        }
    }
    // 0 < tail_length <= 64
    let tail_length = sample.tail_length();
    let x = sample.b64[sample.b64.len() - 1] >> (64 - tail_length);

    for i in (0..tail_length).rev() {
        e.push(((x >> i) & 1) as f32);
    }

    // get_fft(f.len()).process(&mut f);
    let mut f = vec![Complex32::default(); n / 2 + 1];
    // SAFTY: process returns error only if e and f have wrong length.
    get_fft(e.len()).process(&mut e, &mut f).unwrap();

    let t = sqrt(2.995732274 * n as f64);
    let n0 = 0.95 * n as f64 / 2.0;
    // f0 = sum(x_k) = 2*pop - n
    let mut n1 = if ((2 * sample.pop as i64 - n as i64).abs() as f64) < t {
        1
    } else {
        0
    };
    n1 += f[1..=(n / 2 - 1)]
        .iter()
        .map(|x| if (x.abs() as f64) < t / 2.0 { 1 } else { 0 })
        .sum::<i64>();

    let d = (n1 as f64 - n0) / sqrt(n as f64 * 0.95 * 0.05 / 3.8);
    let pv = erfc(abs(d) / sqrt(2.0));
    let qv = erfc(d / sqrt(2.0)) / 2.0;

    TestResult { pv, qv }
}

#[cfg(test)]
mod tests {
    use super::{super::tests::*, *};
    use crate::{test_data::E, Sample};

    #[test]
    fn test_basic() {
        let tv = get_test_vec(crate::TestFuncs::DiscreteFourier);
        let sample: Sample = tv.0.into();
        assert_eq!(discrete_fourier_epsilon(&sample), tv.2);
        assert_eq!(discrete_fourier_u64(&sample), tv.2);
    }

    #[test]
    fn test_e() {
        let tv = get_test_vec_e(crate::TestFuncs::DiscreteFourier);
        let sample: Sample = E.into();
        assert_eq!(tv.1, discrete_fourier_epsilon(&sample));
        assert_eq!(tv.1, discrete_fourier_u64(&sample));
        assert_eq!(tv.1, discrete_fourier_u8(&sample));
    }

    #[test]
    fn test_equal() {
        for nbits in 18..1000 {
            let sample: Sample = E[..nbits * 8].into();
            assert_eq!(discrete_fourier_epsilon(&sample), discrete_fourier_u64(&sample));
            assert_eq!(discrete_fourier_u8(&sample), discrete_fourier_u64(&sample));
        }
    }
}

#[cfg(test)]
mod bench {
    extern crate test;
    use super::*;
    use crate::{test_data::E, Sample};
    use test::Bencher;

    const K: i32 = 3;

    #[bench]
    fn bench_test_epsilon(b: &mut Bencher) {
        let sample: Sample = E.into();

        // 23,200,620.80
        b.iter(|| {
            test::black_box(discrete_fourier_epsilon(&sample));
        });
    }

    #[bench]
    fn bench_test_u8(b: &mut Bencher) {
        let sample: Sample = E.into();

        // 6,864,074.95 ns/iter
        b.iter(|| {
            test::black_box(discrete_fourier_u8(&sample));
        });
    }

    #[bench]
    fn bench_test_u64(b: &mut Bencher) {
        let sample: Sample = E.into();

        // 22,630,787.50
        // 12,930,812.50
        b.iter(|| {
            test::black_box(discrete_fourier_u64(&sample));
        });
    }
    // #[bench]
    // fn bench_test_fft(b: &mut Bencher) {
    //     let sample: Sample = E.into();
    //     let e = &sample.e;
    //     let mut f = Vec::with_capacity(e.len());
    //     for i in 0..e.len() {
    //         f.push(Complex {
    //             re: e[i] as f64,
    //             im: 0.0,
    //         });
    //     }

    //     // 25,085,366.60 ns/iter
    //     b.iter(|| {
    //         let mut ff = f.clone();
    //         test::black_box(fft(&mut ff));
    //     });
    // }
}
