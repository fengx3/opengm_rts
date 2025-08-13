use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::Display;

use crate::Sample;
use crate::*;

pub const ALPHA: f64 = 0.01;
// GM/T 0005-2021 6.3
pub const SAMPLE_DISTRIBUTION_K: usize = 10;
pub const SAMPLE_DISTRIBUTION_ALPHA_T: f64 = 0.0001;

impl Display for TestFuncs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TestFuncs::Frequency => {
                write!(f, "Frequency")
            }
            TestFuncs::BlockFrequency => {
                write!(f, "BlockFrequency")
            }
            TestFuncs::Poker => {
                write!(f, "Poker")
            }
            TestFuncs::Serial1 => {
                write!(f, "Serial1")
            }
            TestFuncs::Serial2 => {
                write!(f, "Serial2")
            }
            TestFuncs::Runs => {
                write!(f, "Runs")
            }
            TestFuncs::RunsDistribution => {
                write!(f, "RunsDistribution")
            }
            TestFuncs::LongestRun0 => {
                write!(f, "LongestRun0")
            }
            TestFuncs::LongestRun1 => {
                write!(f, "LongestRun1")
            }
            TestFuncs::BinaryDerivative => {
                write!(f, "BinaryDerivative")
            }
            TestFuncs::Autocorrelation => {
                write!(f, "Autocorrelation")
            }
            TestFuncs::Rank => {
                write!(f, "Rank")
            }
            TestFuncs::CumulativeSumsForward => {
                write!(f, "CumulativeSumsForward")
            }
            TestFuncs::CumulativeSumsBackward => {
                write!(f, "CumulativeSumsBackward")
            }
            TestFuncs::ApproximateEntropy => {
                write!(f, "ApproximateEntropy")
            }
            TestFuncs::LinearComplexity => {
                write!(f, "LinearComplexity")
            }
            TestFuncs::Universal => {
                write!(f, "Universal")
            }
            TestFuncs::DiscreteFourier => {
                write!(f, "DiscreteFourier")
            }
        }
    }
}

pub const ALL_TESTS_FUNCS: &[TestFuncs] = &[
    TestFuncs::Frequency,
    TestFuncs::BlockFrequency,
    TestFuncs::Poker,
    TestFuncs::Serial1,
    TestFuncs::Serial2,
    TestFuncs::Runs,
    TestFuncs::RunsDistribution,
    TestFuncs::LongestRun0,
    TestFuncs::LongestRun1,
    TestFuncs::BinaryDerivative,
    TestFuncs::Autocorrelation,
    TestFuncs::Rank,                  // 0.2
    TestFuncs::CumulativeSumsForward, // 0.1
    TestFuncs::CumulativeSumsBackward,
    TestFuncs::ApproximateEntropy, //0.2
    TestFuncs::LinearComplexity,   // 2.5
    TestFuncs::Universal,          // 0.1
    TestFuncs::DiscreteFourier,    //1.1
];

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Tester {
    pub f: TestFuncs,
    pub param: Option<i32>,
}

impl Display for Tester {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(param) = self.param {
            write!(f, "{}({})", self.f, param)
        } else {
            write!(f, "{}", self.f)
        }
    }
}

impl Tester {
    fn new(f: TestFuncs, param: Option<i32>) -> Tester {
        Tester { f, param }
    }
    fn test(&self, s: &Sample) -> TestResult {
        let mut m = 0;
        if self.param.is_some() {
            m = self.param.unwrap();
        }

        match self.f {
            TestFuncs::Frequency => s.frequency(),
            TestFuncs::BlockFrequency => s.block_frequency(m),
            TestFuncs::Poker => s.poker(m),
            TestFuncs::Serial1 => s.serial1(m),
            TestFuncs::Serial2 => s.serial2(m),
            TestFuncs::Runs => s.runs(),
            TestFuncs::RunsDistribution => s.runs_distribution(),
            TestFuncs::LongestRun0 => s.longest_run0(),
            TestFuncs::LongestRun1 => s.longest_run1(),
            TestFuncs::BinaryDerivative => s.binary_derivative(m),
            TestFuncs::Autocorrelation => s.autocorrelation(m),
            TestFuncs::Rank => s.rank(),
            TestFuncs::CumulativeSumsForward => s.cumulative_sums_forward(),
            TestFuncs::CumulativeSumsBackward => s.cumulative_sums_backward(),
            TestFuncs::ApproximateEntropy => s.approximate_entropy(m),
            TestFuncs::LinearComplexity => s.linear_complexity(m),
            TestFuncs::Universal => s.universal(),
            TestFuncs::DiscreteFourier => s.discrete_fourier(),
        }
    }
}

fn filt_funcs(funcs: &[TestFuncs], bits: usize) -> Vec<TestFuncs> {
    let mut res = Vec::new();
    if bits <= 20000 {
        for f in funcs {
            match *f {
                TestFuncs::Rank | TestFuncs::LinearComplexity | TestFuncs::Universal => {}
                _ => {
                    res.push(*f);
                }
            }
        }
    }else{
        res = Vec::from(funcs);
    }
    res
}

fn get_param(f: TestFuncs, bits: usize) -> Option<Vec<i32>> {
    if bits <= 20000 {
        match f {
            TestFuncs::BlockFrequency => Some(vec![1000]),
            TestFuncs::Poker => Some(vec![4, 8]),
            TestFuncs::Serial1 | TestFuncs::Serial2 => Some(vec![3, 5]),
            TestFuncs::LongestRun0 | TestFuncs::LongestRun1 => Some(vec![128]),
            TestFuncs::BinaryDerivative => Some(vec![3, 7]),
            TestFuncs::Autocorrelation => Some(vec![2, 8, 16]),
            TestFuncs::ApproximateEntropy => Some(vec![2, 5]),
            _ => None,
        }
    } else if bits <= 1000000 {
        // Universal has fixed parameters L,Q
        match f {
            TestFuncs::BlockFrequency => Some(vec![10000]),
            TestFuncs::Poker => Some(vec![4, 8]),
            TestFuncs::Serial1 | TestFuncs::Serial2 => Some(vec![3, 5]),
            TestFuncs::LongestRun0 | TestFuncs::LongestRun1 => Some(vec![10000]),
            TestFuncs::BinaryDerivative => Some(vec![3, 7]),
            TestFuncs::Autocorrelation => Some(vec![1, 2, 8, 16]),
            TestFuncs::ApproximateEntropy => Some(vec![2, 5]),
            TestFuncs::LinearComplexity => Some(vec![500, 1000]),
            _ => None,
        }
    } else {
        match f {
            TestFuncs::BlockFrequency => Some(vec![100000]),
            TestFuncs::Poker => Some(vec![4, 8]),
            TestFuncs::Serial1 | TestFuncs::Serial2 => Some(vec![3, 5, 7]),
            TestFuncs::LongestRun0 | TestFuncs::LongestRun1 => Some(vec![10000]),
            TestFuncs::BinaryDerivative => Some(vec![3, 7, 15]),
            TestFuncs::Autocorrelation => Some(vec![1, 2, 8, 16, 32]),
            TestFuncs::ApproximateEntropy => Some(vec![5, 7]),
            TestFuncs::LinearComplexity => Some(vec![5000]),
            _ => None,
        }
    }
}

// Note: if bits <= 20000, there are no Rank, LinearComplexity and Universal tests.
pub fn get_testers(funcs: &[TestFuncs], bits: usize) -> Vec<Tester> {
    let mut res: Vec<_> = Vec::new();
    for f in filt_funcs(funcs, bits) {
        if let Some(params) = get_param(f, bits) {
            for param in params {
                res.push(Tester { f: f, param: Some(param) });
            }
        } else {
            res.push(Tester { f: f, param: None });
        }
    }
    res
}

pub fn waterline(alpha: f64, s: usize) -> usize {
    let s = s as f64;
    (s * (1.0 - alpha - 3.0 * (alpha * (1.0 - alpha) / s).sqrt())).ceil() as usize
}

pub fn sample_test(s: &Sample, testers: &[Tester]) -> HashMap<Tester, TestResult> {
    let mut result = HashMap::<Tester, TestResult>::new();

    let reverse = if USE_U8 { (s.b[0] & 1) == 0 } else { (s.b64[0] & 1) == 0 };
    if reverse {
        for t in testers.iter().rev() {
            result.insert(*t, t.test(s));
        }
    } else {
        for t in testers {
            result.insert(*t, t.test(s));
        }
    }

    result
}

// let line = waterline(ALPHA, samples.len());
pub fn count_pvalue_pass(res: &Vec<HashMap<Tester, TestResult>>, f: Tester, alpha: f64) -> i32 {
    res.iter().map(|t| if t[&f].pass(alpha) { 1 } else { 0 }).sum()
}

pub fn compute_qvalue_distribution(res: &Vec<HashMap<Tester, TestResult>>, f: Tester) -> f64 {
    let mut qvalues = Vec::with_capacity(res.len() * 2);

    for r in res {
        let result = &r.get(&f).unwrap();
        qvalues.push(result.qv);
    }
    qvalue_distribution(&qvalues, SAMPLE_DISTRIBUTION_K)
}

// return p_value result and q_value result.
// p_value result returns [samples[0].Result, samples[1].Result,...]
// samples[i].Result: Tester: TestResult.
// q_value result returns hashmap: Tester: q_value distribution.
pub fn randomness_test(samples: &Vec<Sample>, testers: &[Tester]) -> (Vec<HashMap<Tester, TestResult>>, HashMap<Tester, f64>) {
    if samples.len() == 0 || testers.len() == 0 {
        return (Vec::<HashMap<Tester, TestResult>>::new(), HashMap::new());
    }
    let presult = samples
        .par_iter() // multiple threads by rayon
        // .iter() // sigle thread
        .map(|s| sample_test(s, &testers))
        .collect::<Vec<HashMap<Tester, TestResult>>>();

    let mut qresult = HashMap::with_capacity(testers.len());
    for f in testers {
        qresult.insert(*f, compute_qvalue_distribution(&presult, *f));
    }
    (presult, qresult)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use std::time::Instant;

    // cargo test --release --package opengm_rts --lib -- tester::tests::test_rts --exact --show-output
    #[test]
    fn test_rts() {
        let mut samples: Vec<Sample> = Vec::new();
        let bits = 1000000;
        let mut data = vec![0u8; bits / 8];
        let mut rng = rand::rng();
        let testers = get_testers(&ALL_TESTS_FUNCS, bits);

        for _ in 0..1000 {
            rng.fill_bytes(&mut data);
            samples.push(Sample::from(data.as_slice()));
        }
        let start = Instant::now();
        randomness_test(&samples, &testers);
        let elapsed = Instant::now() - start;
        println!("Used time: {} s", elapsed.as_secs_f64())
    }

    #[test]
    fn test_waterline() {
        assert_eq!(waterline(ALPHA, 1000), 981);
    }
}
