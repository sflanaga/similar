use threadpool;
use std::path::PathBuf;
use structopt::StructOpt;
use strsim::{damerau_levenshtein, osa_distance, levenshtein};
use crate::io::{lines_from_file, lines_from_file_alphanum_only};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::fmt::Debug;
use std::process::exit;

///
/// Compare to list of strings and find top N "best" matches in file2 for each string in file1
///
///
///

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

type StrCmpFn = fn(&str, &str) -> usize;

#[derive(StructOpt, Clone)]
#[structopt()]
/// Search for strings within a reference list for most similar strings
pub struct CliCfg {
    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbosity - use more than one v for greater detail
    pub verbose: usize,

    #[structopt(short = "f", name = "search_list_path", group = "ref_source")]
    /// file containing a list of strings to search for closest matches against the references list
    search_vec_path: Option<PathBuf>,

    #[structopt(short = "s", name = "search_string", group = "ref_source")]
    /// list of strings to try searches for closest matches against the references list
    search_vec: Option<Vec<String>>,

    #[structopt(short = "r", name = "reference_list_path", parse(from_os_str))]
    /// larger list to search in for closest matches - the reference list
    reference_list_path: PathBuf,

    #[structopt(short = "d", default_value = ",")]
    /// output delimiter used for results
    delimiter: String,

    #[structopt(short = "t", long = "top_n_matches", default_value = "3")]
    /// limit the number of match written to the top X most
    top_n_matches: usize,

    #[structopt(short = "a", long = "str_simularity_algorithm", default_value = "damerau_levenshtein", parse(try_from_str = str_to_sim_alg))]
    // algorithm can be either damerau_levenshtein, osa_distance, or levenshtein
    alg: StrCmpFn,

    #[structopt(short = "c", long = "alpha_num_chars_only")]
    // algorithm can be either damerau_levenshtein, osa_distance, or levenshtein
    alpha_num_chars_only: bool,


}

mod io;

#[derive(Eq, Debug)]
struct TrackedString {
    size: usize,
    string: String,
}

fn str_to_sim_alg(s: &str) -> Result<StrCmpFn> {
    match s {
        "damerau_levenshtein" => Ok(damerau_levenshtein),
        "osa_distance" => Ok(osa_distance),
        "levenshtein" => Ok(levenshtein),
        _ => Err(format!("Error: string similarity algorithm \"{}\" unknown.  Must be one of damerau_levenshtein, osa_distance, or levenshtein", s))?,
    }
}

fn alg_to_str<'r, 's>(f: fn(&'r str, &'s str) -> usize) -> &'static str {
    match f {
        f if f == damerau_levenshtein => "damerau_levenshtein",
        f if f == osa_distance => "osa_distance",
        f if f == levenshtein => "levenshtein",
        _ => "uh",
    }
}

// only emmits a warning about unreachable but creates a bug
// fn alg_to_str<'r, 's>(f: fn(&'r str, &'s str) -> usize) -> &'static str {
//     match f {
//         damerau_levenshtein => "damerau_levenshtein",
//         osa_distance => "osa_distance",
//         levenshtein => "levenshtein",
//         _ => "uh",
//     }
// }

impl Ord for TrackedString {
    fn cmp(&self, other: &TrackedString) -> Ordering {
        self.size.cmp(&other.size).reverse()
    }
}

impl PartialOrd for TrackedString {
    fn partial_cmp(&self, other: &TrackedString) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TrackedString {
    fn eq(&self, other: &TrackedString) -> bool {
        self.size == other.size
    }
}

fn search(alg: StrCmpFn, result_idx: usize, results: &mut Arc<Mutex<Vec<String>>>, search_str: &str, ref_vec: &Vec<String>, limit: usize, del: &str) -> () {
    let mut heap = BinaryHeap::new();
    for ref_str in ref_vec.iter() {
        let val = alg(search_str, ref_str);
        heap.push(TrackedString {
            size: val,
            string: ref_str.to_string(),
        });
    }
    let mut line = String::with_capacity(100);
    line.push_str(&search_str);
    line.push_str(del);
    line.push_str(&format!("{}{}", heap.len(), del));
    let mut count_zero = 0usize;
    for p in heap.iter() {
        if p.size == 0 {
            count_zero += 1;
        }
    }
    for _i in 0..limit {
        match heap.pop() {
            Some(p) => {
                line.push_str(&format!("{},{}", p.size, &p.string))
            }
            None => break,
        }
        line.push_str(del);
    }
    if count_zero > 0 {
        line.push_str(&format!("{}, {} COUNT OF ZERO results", del, count_zero));
    }
    let mut res = results.lock().unwrap();
    res[result_idx] = line;
    ()
}

// TODO:
// - add special character strip levels before matching
// - pretty string diff of top N matches


// fn __main() -> Result<()> {
//     println!("{}", damerau_levenshtein("count", "5G capable RRC Connection Number-EnDcConnNoCnt(count)"));
//     exit(1);
//
//     Ok(())
// }

fn main() -> Result<()> {
    let cfg = CliCfg::from_args();
    // let str_sim_fn = match cfg.alg {
    //     "damerau_levenshtein" => damerau_levenshtein()| "osa_distance" | "osa_distance" =>
    // }
    // [algname] file1 file2
    let ref_vec = if !cfg.alpha_num_chars_only {
        Arc::new(lines_from_file(&cfg.reference_list_path))
    } else {
        Arc::new(lines_from_file_alphanum_only(&cfg.reference_list_path))
    };
    let search_vec = match cfg.search_vec_path {
        Some(search_file_path) => {
            let search_vec = if !cfg.alpha_num_chars_only {
                lines_from_file(&search_file_path)
            } else {
                lines_from_file_alphanum_only(&search_file_path)
            };
            println!("Each of {} strings in file: \"{}\" find top matches from reference list in file \"{}\" using algorithm \"{}\"",
                     search_vec.len(), search_file_path.to_str().unwrap(), search_file_path.to_str().unwrap(), alg_to_str(cfg.alg));
            Arc::new(search_vec)
        }
        None => {
            match cfg.search_vec {
                Some(mut search_vec) => {
                    //let mut search_vec = search_vec.clone();
                    println!("Search strings: ");
                    for s in search_vec.iter_mut() {
                        if cfg.alpha_num_chars_only {
                            s.retain(|c| c.is_alphanumeric());
                        }
                        println!("\t{}", &s);
                    }
                    println!("Find top matches from reference list in file \"{}\" using algorithm \"{}\"",
                             cfg.reference_list_path.to_str().unwrap(), alg_to_str(cfg.alg));
                    Arc::new(search_vec)
                }
                None => Err("neither search strings or search file list were given")?,
            }
        }
    };


    let empty = String::new();
    let results: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![empty; search_vec.len()]));

    let pool = threadpool::Builder::new().build();
    let start_f = std::time::Instant::now();
    for (idx, search_str) in search_vec.clone().iter().enumerate() {
        let ref_vec_clone = ref_vec.clone();
        let search_str_clone = search_str.clone();
        let del = cfg.delimiter.clone();
        let top_n_matches = cfg.top_n_matches;
        let mut results_clone = results.clone();
        let alg = cfg.alg;
        pool.execute(move || { search(alg, idx, &mut results_clone, &search_str_clone, &ref_vec_clone, top_n_matches, &del) });
        // println!("submit: {}", &ref_str);
    }

    pool.join();

    let res = results.lock().unwrap();
    for val in res.iter() {
        println!("{}", &val);
    }
    println!("done in {:.3} secs", start_f.elapsed().as_secs_f32());
    Ok(())
}
