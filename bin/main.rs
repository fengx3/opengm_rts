mod alloc;
mod thread_pool;

use opengm_rts::*;
use std::{
    collections::HashMap,
    env, fs,
    io::{self, Write},
    path::PathBuf,
    thread,
    time::Instant,
};
use thread_pool::ThreadPool;

use crate::alloc::max_mem_used;

macro_rules! pprint {
    ($v: expr, $n: expr) => {{
        let s = format!("{}", $v);
        assert!(s.len() <= $n);
        print!("{}", s);
        (0..($n - s.len())).for_each(|_| print!(" "));
    }};
}

macro_rules! pprint_center {
    ($v: expr, $n: expr) => {{
        let s = format!("{}", $v);
        assert!(s.len() <= $n);
        (0..($n - s.len()) / 2).for_each(|_| print!(" "));
        print!("{}", s);
        (0..($n - s.len() - ($n - s.len()) / 2)).for_each(|_| print!(" "));
    }};
}

const COL1: usize = 24;
const COL2: usize = 12;
const COL3: usize = 6;
const COL4: usize = 12;
const COL5: usize = 6;
const COL: usize = COL1 + COL2 + COL3 + COL4 + COL5 + 4;

const PASS: &str = "PASS";
const FAIL: &str = "FAIL";

fn print_line(delimiter: &str) {
    print!("+");
    (0..COL1).for_each(|_| print!("-"));
    print!("{}", delimiter);
    (0..COL2).for_each(|_| print!("-"));
    print!("{}", delimiter);
    (0..COL3).for_each(|_| print!("-"));
    print!("{}", delimiter);
    (0..COL4).for_each(|_| print!("-"));
    print!("{}", delimiter);
    (0..COL5).for_each(|_| print!("-"));
    println!("+");
}

pub fn read_dir(data_dir: &str) -> Result<Vec<(PathBuf, u64)>, io::Error> {
    let mut result = Vec::new();
    for entry in fs::read_dir(data_dir)? {
        let path = entry?.path();
        let size = fs::metadata(&path).unwrap().len();
        if fs::metadata(&path)?.is_file() {
            result.push((path, size));
        }
    }

    Ok(result)
}

const USAGE: &str = r#"Usage: ./opengm_rts <path/to/data/dir>

Example: 
$ ls data
000.bin 001.bin 002.bin 003.bin 004.bin ... 999.bin
$ ./opengm_rts ./data
"#;

fn main() {
    println!("opengm_rts {}\n{}", VERSION, LICENSE);

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("{}", USAGE);
        return;
    }
    let data_dir = args[1].as_str();
    let verbose = false;

    let start_time = Instant::now();

    let num_cpus = thread::available_parallelism().unwrap().get();

    let paths = match read_dir(data_dir) {
        Ok(path) => path,
        Err(err) => {
            println!("read file error: {:?}", err);
            return;
        }
    };
    if paths.len() < 1 {
        println!("no files in {:?}.", data_dir);
        return;
    }
    let n = paths.len();
    let bits = (paths[0].1 * 8) as usize;
    let w = waterline(ALPHA, n);

    // The statistics threads pool.
    let (pool, result_receiver) = ThreadPool::new(num_cpus);

    // The reading thread
    thread::spawn(move || {
        for path in paths {
            let content = match fs::read(&path.0) {
                Ok(content) => content,
                Err(err) => {
                    println!("read file {:?} error: {:?}", &path.0, err);
                    std::process::exit(-1);
                }
            };
            let testers = get_testers(&ALL_TESTS_FUNCS, bits);
            let sample = Sample::from(content);
            pool.execute(move || sample_test(&sample, &testers));
        }
    });

    let testers = get_testers(&ALL_TESTS_FUNCS, bits);
    // In main thread, read results from pool.
    let mut presult = Vec::with_capacity(n);
    let mut percent = 0;
    for i in 0..n {
        if verbose {
            print!("\r{}% completed", percent);
            if i % 10 == 0 {
                percent = 100 * i / n;
            }
            io::stdout().flush().unwrap();
        }
        presult.push(result_receiver.recv().unwrap());
    }
    if verbose {
        print!("\r");
    }

    let mut qresult = HashMap::with_capacity(testers.len());
    for f in &testers {
        qresult.insert(*f, compute_qvalue_distribution(&presult, *f));
    }

    let mut pfailed_tests = Vec::new();
    let mut qfailed_tests = Vec::new();

    print_line("-");
    print!("|");
    pprint!(format!("Number of samples: {}", n), COL);
    println!("|");
    print!("|");
    pprint!(format!("Bits per sample:   {}", bits), COL);
    println!("|");
    print!("|");
    pprint!(format!("P_value threshold: {}", w), COL);
    println!("|");
    print!("|");
    pprint!(format!("Q_value threshold: {}", SAMPLE_DISTRIBUTION_ALPHA_T), COL);
    println!("|");

    print_line("-");

    print!("|");
    pprint!(" ", COL1);
    print!(" ");
    pprint_center!("p_value", COL2);
    print!(" ");
    pprint!(" ", COL3);
    print!(" ");
    pprint_center!("q_value", COL4);
    print!(" ");
    pprint!(" ", COL5);
    println!("|");

    print_line("+");

    for tester in testers {
        print!("|");
        pprint!(tester, COL1);
        print!("|");

        let passed = count_pvalue_pass(&presult, tester, ALPHA);
        pprint_center!(format!("{}/{}", passed, n), COL2);
        print!("|");

        let pv_result = passed >= w as i32;
        pprint_center!(if pv_result { PASS } else { FAIL }, COL3);
        print!("|");

        pprint_center!(format!("{:.4}", qresult.get(&tester).unwrap()), COL4);
        print!("|");
        let qv_result = *qresult.get(&tester).unwrap() >= SAMPLE_DISTRIBUTION_ALPHA_T;
        pprint_center!(if qv_result { PASS } else { FAIL }, COL5);
        println!("|");

        if !pv_result {
            pfailed_tests.push(tester);
        }
        if !qv_result {
            qfailed_tests.push(tester);
        }
    }

    print_line("+");

    if pfailed_tests.len() == 0 && pfailed_tests.len() == 0 {
        print!("|");
        pprint!("Randomness test PASS.", COL);
        println!("|");
    } else {
        print!("|");
        println!("Randomness test FAIL.");
        println!("|");
    }

    print!("|");
    pprint!(
        format!("Time used: {:.01} seconds", (Instant::now() - start_time).as_secs_f64()),
        COL
    );
    println!("|");

    print!("|");
    pprint!(format!("Memory used: {:.01} MB", max_mem_used() / 1024 / 1024), COL);
    println!("|");
    print_line("-");
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Instant};

    use rand::RngCore;

    use super::*;

    // cargo test --release --package opengm_rts --bin opengm_rts --features build-binary -- tests::test_rts --exact --show-output
    // 1亿比特
    // other( without FFT and LinearComplexity): 125s, 12G
    // all: 716s
    #[test]
    fn test_rts() {
        // const NBITS :usize = 100*100*10000 / 8;
        const NBITS: usize = 100 * 10000 / 8;
        // const NBITS: usize = 20000 / 8; // 2w bits
        let testers = get_testers(&ALL_TESTS_FUNCS, NBITS);
        // println!("{:?}", testers);
        let mut samples: Vec<Sample> = Vec::new();
        let mut data = vec![0u8; NBITS];
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            rng.fill_bytes(&mut data);
            samples.push(Sample::from(data.as_slice()));
        }
        let start = Instant::now();

        let _result = randomness_test(&samples, &testers);

        let elapsed = Instant::now() - start;
        println!("Used time: {} s", elapsed.as_secs_f64())
    }

    #[test]
    fn test_gen_data() {
        const NBITS: usize = 10000 * 10000 / 8;
        let mut data = vec![0u8; NBITS];
        let mut rng = rand::thread_rng();
        for i in 0..1000 {
            rng.fill_bytes(&mut data);
            let filename = format!("./data1y/{:03}.bin", i);
            fs::write(filename, &data).unwrap();
        }
    }

    #[test]
    fn test_waterline() {
        assert_eq!(waterline(ALPHA, 1000), 981);
    }
}
