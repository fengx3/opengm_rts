#![allow(dead_code)]
#![allow(non_snake_case)]
#![feature(test)]
#![feature(portable_simd)]
// #![allow(unused_imports)]


mod tester;
mod testsuits;

use std::sync::Mutex;
use std::sync::Arc;
pub use tester::*;
use testsuits::util::*;


#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TestFuncs {
    Frequency,              // 1. 频度检测
    BlockFrequency,         // 2. 块内频度检测
    Poker,                  // 3. 扑克检测
    Serial1,                // 4. 重叠子序列p_value1检测
    Serial2,                // 4. 重叠子序列p_value2检测
    Runs,                   // 5. 游程分布检测
    RunsDistribution,       // 6. 游程分布检测
    LongestRun0,            // 7. 块内最大0游程检测
    LongestRun1,            // 7. 块内最大1游程检测
    BinaryDerivative,       // 8. 二元推导检测
    Autocorrelation,        // 9. 自相关检测
    Rank,                   // 10. 矩阵秩检测
    CumulativeSumsForward,  // 11. 前向累加和检测
    CumulativeSumsBackward, // 11. 后向累加和检测
    ApproximateEntropy,     // 12. 近似熵检测
    LinearComplexity,       // 13. 线性复杂度检测
    Universal,              // 14. Maurer通用统计检测
    DiscreteFourier,        // 15. 离散傅立叶检测
}

#[derive(Default, Debug)]
pub struct TestResult {
    pub pv: f64,
    pub qv: f64,
}

impl PartialEq for TestResult {
    fn eq(&self, other: &Self) -> bool {
        (self.pv - other.pv).abs() < 0.0001 && (self.qv - other.qv).abs() < 0.0001
    }
}

impl TestResult {
    pub fn pass(&self, alpha: f64) -> bool {
        self.pv >= alpha
    }
}

const MAX_OVERLAPPING_PATTERN: usize = 8;
const USE_U8: bool = true;


pub struct Sample {
    // Note: for a byte x, bit 0 is (x>>7) & 1 and bit 7 is x & 1.
    b: Vec<u8>,

    // Note: for an u64 x, bit 0 is (x>>63) & 1 and bit 63 is x & 1.
    // use Vec<u8> or Vec<u64> decided by set USE_U8 = true or false.
    // The performance is approximately equal.
    b64: Vec<u64>,

    bit_length: usize,

    // The epsilon, use one u8 to represent a bit.
    #[cfg(test)]
    e: Vec<u8>,

    // popcount
    pop: u64,

    // 重叠子序列和近似熵
    patterns: Mutex<Vec<Option<Arc<Vec<u64>>>>>,

    // 线性复杂度
    complexities: Mutex<Vec<Option<Vec<u16>>>>,

    // 最大游程
    longest_run: Mutex<Option<[f64;2]>>,
}

fn u64_from(b: &[u8]) -> Vec<u64> {
    let mut b64 = Vec::with_capacity((b.len() + 7) / 8);
    let full_chunks = b.len() & (!7);
    for chunk in b[..full_chunks].chunks_exact(8) {
        b64.push(u64::from_be_bytes(chunk.try_into().unwrap()));
    }

    if full_chunks < b.len() {
        b64.push(u64_from_be_slice(&b[full_chunks..]));
    }
    b64
}

// from bit string("10100...")
impl From<&str> for Sample {
    fn from(bit_string: &str) -> Self {
        let bit_length = bit_string.len();
        let e = {
            let mut e = Vec::with_capacity(bit_string.len());
            for c in bit_string.chars() {
                e.push(c as u8 - '0' as u8);
            }
            e
        };

        let b = if USE_U8 {
            let mut b = Vec::with_capacity((bit_string.len() + 7) / 8);
            let full_chunks = bit_length & (!7);
            for chunk in e[..full_chunks].chunks_exact(8) {
                let mut x = 0;
                for i in chunk {
                    x = (x << 1) | (*i);
                }
                b.push(x);
            }
            if full_chunks < bit_length {
                let mut c = 0;
                for (i, x) in e[full_chunks..].iter().enumerate() {
                    c |= (*x) << (7 - i);
                }
                b.push(c);
            }
            b
        } else {
            Vec::new()
        };

        #[cfg(test)]
        let b64 = u64_from(&b);
        #[cfg(not(test))]
        let b64 = if USE_U8 { Vec::new() } else { u64_from(&b) };

        let pop = popcount_u8(&b);
        Sample {
            b,
            bit_length,
            b64,
            #[cfg(test)]
            e,
            pop,
            patterns: Mutex::new(vec![None; MAX_OVERLAPPING_PATTERN + 1]),
            complexities: Mutex::new(vec![None;2]),
            longest_run: Mutex::new(None),
        }
    }
}

impl From<&[u8]> for Sample {
    fn from(byte_slice: &[u8]) -> Self {
        let bit_length = byte_slice.len() * 8;
        #[cfg(test)]
        let e = {
            let mut e = Vec::with_capacity(byte_slice.len() * 8);
            for c in byte_slice {
                for j in 0..8 {
                    e.push((*c >> (7 - j)) & 1)
                }
            }
            e
        };

        #[cfg(test)]
        let b64 = u64_from(&byte_slice);
        #[cfg(not(test))]
        let b64 = if USE_U8 { Vec::new() } else { u64_from(&byte_slice) };

        let b = if USE_U8 { byte_slice.to_vec() } else { Vec::new() };
        Sample {
            b,
            bit_length,
            b64,
            #[cfg(test)]
            e,
            pop: popcount_u8(&byte_slice),
            patterns: Mutex::new(vec![None; MAX_OVERLAPPING_PATTERN + 1]),
            complexities: Mutex::new(vec![None; 2]),
            longest_run: Mutex::new(None),
        }
    }
}


// move vec into sample
impl From<Vec<u8>> for Sample {
    fn from(v: Vec<u8>) -> Self {
        let bit_length = v.len() * 8;
        #[cfg(test)]
        let e = {
            let mut e = Vec::with_capacity(v.len() * 8);
            for c in &v {
                for j in 0..8 {
                    e.push((*c >> (7 - j)) & 1)
                }
            }
            e
        };

        #[cfg(test)]
        let b64 = u64_from(&v);
        #[cfg(not(test))]
        let b64 = if USE_U8 { Vec::new() } else { u64_from(&v) };

        let pop = popcount_u8(&v);
        let b = if USE_U8 { v } else { Vec::new() };

        Sample {
            b,
            bit_length,
            b64,
            #[cfg(test)]
            e,
            pop,
            patterns: Mutex::new(vec![None; MAX_OVERLAPPING_PATTERN + 1]),
            complexities: Mutex::new(vec![None; 2]),
            longest_run: Mutex::new(None),
        }
    }
}


#[inline(always)]
fn get_bit_unchecked_u64(b64: &[u64], i: usize) -> u8 {
    unsafe { (b64.get_unchecked(i / 64) >> (63 - (i & 63))) as u8 & 1 }
}

#[inline(always)]
fn get_bit_unchecked_u8(b8: &[u8], i: usize) -> u8 {
    unsafe { (b8.get_unchecked(i / 8) >> (8 - (i & 7))) & 1 }
}

impl Sample {
    pub fn new(bytes: &[u8]) -> Sample {
        bytes.into()
    }

    #[inline(always)]
    pub fn get_bit_unchecked(&self, i: usize) -> u8 {
        get_bit_unchecked_u64(&self.b64, i)
    }

    /// returns the number of bits of sample.
    pub fn len(&self) -> usize {
        self.bit_length
    }

    // return the last valid bits in self.b64[self.b64.len()-1];
    pub(crate) fn tail_length(&self) -> usize {
        64 - (64 - self.len() % 64) % 64
    }

    // return self.b64[self.b64.len()-1] aligned right.
    // i.e., the last bit is the return u64 & 1.
    pub(crate) fn tail_aligned_right(&self) -> u64 {
        self.b64[self.b64.len() - 1] >> (64 - self.tail_length())
    }

    pub fn frequency(&self) -> TestResult {
        testsuits::frequency(self)
    }

    pub fn block_frequency(&self, m: i32) -> TestResult {
        testsuits::block_frequency(self, m)
    }

    pub fn poker(&self, m: i32) -> TestResult {
        testsuits::poker(self, m)
    }

    pub fn serial1(&self, m: i32) -> TestResult {
        testsuits::serial1(self, m)
    }
    pub fn serial2(&self, m: i32) -> TestResult {
        testsuits::serial2(self, m)
    }

    pub fn runs(&self) -> TestResult {
        testsuits::runs(self)
    }

    pub fn runs_distribution(&self) -> TestResult {
        testsuits::runs_distribution(self)
    }

    pub fn longest_run0(&self) -> TestResult {
        testsuits::longest_run0(self)
    }

    pub fn longest_run1(&self) -> TestResult {
        testsuits::longest_run1(self)
    }

    pub fn binary_derivative(&self, k: i32) -> TestResult {
        testsuits::binary_derivative(self, k)
    }

    pub fn autocorrelation(&self, d: i32) -> TestResult {
        testsuits::autocorrelation(self, d)
    }

    pub fn rank(&self) -> TestResult {
        testsuits::rank(self)
    }

    pub fn cumulative_sums_forward(&self) -> TestResult {
        testsuits::cumulative_sums_forward(self)
    }
    pub fn cumulative_sums_backward(&self) -> TestResult {
        testsuits::cumulative_sums_backward(self)
    }

    pub fn approximate_entropy(&self, m: i32) -> TestResult {
        testsuits::approximate_entropy(self, m)
    }

    pub fn linear_complexity(&self, m: i32) -> TestResult {
        // testsuits::linear_complexity_u64(self, m)
        testsuits::linear_complexity(self, m)
    }

    pub fn universal(&self) -> TestResult {
        testsuits::universal(self)
    }

    pub fn discrete_fourier(&self) -> TestResult {
        testsuits::discrete_fourier(self)
    }

    fn iter_u64(&self) -> U8Iterator{
        U8Iterator::new(&self.b)
    }

    fn tail_u64(&self) -> (u64, usize){
        tail_u64(&self.b)
    }
}


#[inline(always)]
pub(crate) fn tail_u64(b:&[u8]) -> (u64, usize){
    let tail_bits_length = (b.len() % 8) * 8;
    if tail_bits_length == 0{
        return (0, 0);
    }
    let length = b.len();
    let tail = u64_from_be_slice(&b[length & (!7)..]);
    (tail, tail_bits_length)

}

// U8Iterator iterates over a &[u8] with returned item u64
pub(crate) struct U8Iterator<'a>{
    curr: usize,
    b: &'a [u8]
}
impl<'a> U8Iterator<'a>{
    fn new(b: &'a [u8])-> Self{
        U8Iterator{
            curr: 0,
            b
        }
    }
    fn tail(&self) -> (u64, usize){
        let tail_bits_length = (self.b.len() % 8) * 8;
        if tail_bits_length == 0{
            return (0, 0);
        }
        let length = self.b.len();
        let tail = u64_from_be_slice(&self.b[length & (!7)..]);
        (tail, tail_bits_length)
    }
}

impl<'a> Iterator for U8Iterator<'a>{
    type Item = u64;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr + 8 <= self.b.len(){
            let r = u64::from_be_bytes(self.b[self.curr..self.curr+8].try_into().unwrap());
            self.curr += 8;
            return Some(r);
        }
        None
    }
}


/// GM/T 0005-2021, 6.3 样本分布均匀性判定
/// The value of qv[i] should distribute formally in [0,1].
pub fn qvalue_distribution(qv: &[f64], k: usize) -> f64 {
    if qv.len() <= 1 {
        return 1.0;
    }

    let s = qv.len();
    let mut F = vec![0.0; k];

    // i/k <= v < (i+1)/k => i <= kv < i+1
    for v in qv {
        let mut idx = (*v * k as f64).floor() as usize;
        if idx >= k{
            idx = k-1;
        }
        F[idx] += 1.0;
    }
    let sk = s as f64 / k as f64;
    let V: f64 = F.iter().map(|f| (*f - sk) * (*f - sk) / sk).sum();
    igamc((k - 1) as f64 / 2.0, V / 2.0)
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LICENSE: &str = "Copyright (c) 2024 The OpenGM Group <opengm@yeah.net>";



#[cfg(test)]
mod test_data;

#[cfg(test)]
pub mod tests {
    use super::test_data::E;
    use super::*;
    use rand::RngCore;
    use std::collections::HashMap;
    use std::fs;
    use std::time::Instant;

    // Test for all 15 test functions.
    // cargo test --release --package opengm_rts --lib -- tests::test_all --exact --show-output
    #[test]
    fn test_all() {
        let sample: Sample = E.into();
        let mut pv = HashMap::new();

        let start = Instant::now();
        let last = start;

        pv.insert(TestFuncs::Frequency, sample.frequency());
        let now = Instant::now();
        println!("frequency: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::BlockFrequency, sample.block_frequency(10000));
        let now = Instant::now();
        println!("block_frequency: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Poker, sample.poker(8));
        let now = Instant::now();
        println!("poker: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Serial1, sample.serial1(5));
        let now = Instant::now();
        println!("serial: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Serial2, sample.serial2(5));
        let now = Instant::now();
        println!("serial: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Runs, sample.runs());
        let now = Instant::now();
        println!("runs: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::RunsDistribution, sample.runs_distribution());
        let now = Instant::now();
        println!("runs_distribution: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::LongestRun0, sample.longest_run0());
        let now = Instant::now();
        println!("longest_run0: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::LongestRun1, sample.longest_run1());
        let now = Instant::now();
        println!("longest_run0: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::BinaryDerivative, sample.binary_derivative(7));
        let now = Instant::now();
        println!("binary_derivative: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Autocorrelation, sample.autocorrelation(16));
        let now = Instant::now();
        println!("autocorrelation: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Rank, sample.rank());
        let now = Instant::now();
        println!("rank: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::CumulativeSumsForward, sample.cumulative_sums_forward());
        let now = Instant::now();
        println!("cumulative_sums_forward: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::CumulativeSumsBackward, sample.cumulative_sums_backward());
        let now = Instant::now();
        println!("cumulative_sums_backward: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::ApproximateEntropy, sample.approximate_entropy(5));
        let now = Instant::now();
        println!("approximate_entropy: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::LinearComplexity, sample.linear_complexity(1000));
        let now = Instant::now();
        println!("linear_complexity: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::Universal, sample.universal());
        let now = Instant::now();
        println!("universal: {:.6} s", (now - last).as_secs_f64());
        let last = now;

        pv.insert(TestFuncs::DiscreteFourier, sample.discrete_fourier());
        let now = Instant::now();
        println!("discrete_fourier: {:.6} s", (now - last).as_secs_f64());

        println!("total: {:.6} s", (Instant::now() - start).as_secs_f64());
    }

    // Test for all 15 test functions.
    // cargo test --release --package opengm_rts --lib -- tests::test_all_time --exact --show-output
    #[test]
    fn test_all_time() {
        let mut samples: Vec<Sample> = Vec::new();
        let mut data = vec![0u8; 1000000 / 8];
        let mut rng = rand::rng();
        for _ in 0..1000 {
            rng.fill_bytes(&mut data);
            samples.push(Sample::from(data.as_slice()));
        }
        let mut pv = HashMap::new();

        // total: 35.769053
        let start = Instant::now();
        for sample in samples {
            // // 0s
            // pv.insert(TestFuncs::Frequency, sample.frequency());

            // // 0.01
            // pv.insert(TestFuncs::BlockFrequency, sample.block_frequency(10000));

            // // 0.09
            // pv.insert(TestFuncs::Poker, sample.poker(8));

            // // 1.10 s
            // pv.insert(TestFuncs::Serial1, sample.serial1(7));
            // pv.insert(TestFuncs::Serial2, sample.serial2(7));

            // // 0.02 s
            // pv.insert(TestFuncs::Runs, sample.runs());

            // // 0.98 s
            // pv.insert(TestFuncs::RunsDistribution, sample.runs_distribution());

            // // 0.96s - TODO
            // pv.insert(TestFuncs::LongestRun0, sample.longest_run0());
            // pv.insert(TestFuncs::LongestRun1, sample.longest_run1());

            // // 0.83s, 0.04 s
            // pv.insert(TestFuncs::BinaryDerivative, sample.binary_derivative(7));

            // // 0.65s, 0.02s
            // pv.insert(TestFuncs::Autocorrelation, sample.autocorrelation(8));

            // // 6.67s -> 1.89 s
            // pv.insert(TestFuncs::Rank, sample.rank());

            // // 1s
            // pv.insert(
            //     TestFuncs::CumulativeSumsForward,
            //     sample.cumulative_sums_forward(),
            // );

            // // 1s
            // pv.insert(
            //     TestFuncs::CumulativeSumsBackward,
            //     sample.cumulative_sums_backward(),
            // );

            // // 0.94s
            // pv.insert(
            //     TestFuncs::ApproximateEntropy,
            //     sample.approximate_entropy(5),
            // );

            // // m=1000:16.975644 s
            // // m=500:12.231s
            // pv.insert(
            //     TestFuncs::LinearComplexity,
            //     sample.linear_complexity(500),
            // );

            // pv.insert(
            //     TestFuncs::LinearComplexity,
            //     sample.linear_complexity(1000),
            // );

            // // 0.58s - TODO
            pv.insert(TestFuncs::Universal, sample.universal());

            // // 25.199296 s(rustFFT)
            // // 12.091892 s(realFFT)
            // pv.insert(TestFuncs::DiscreteFourier, sample.discrete_fourier());
        }
        println!("total: {:.6} s", (Instant::now() - start).as_secs_f64());
    }

    #[test]
    fn test_iter() {
        let sample: Sample = E[..1000].into();
        for (i, x) in sample.iter_u64().enumerate(){
            println!("{}: {:016x} ", i, x);
        }
        println!("tail: {:016x}, {}", sample.tail_u64().0, sample.tail_u64().1);

    }

    #[test]
    fn test_1ybits(){
        let data = fs::read("data1y.bin").unwrap();
        let bits = data.len()*8;
        let sample = Sample::from(data);
        let result = sample_test(&sample, &get_testers(ALL_TESTS_FUNCS, bits));
        println!("{:#?}", result)

    }
}
