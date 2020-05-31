#![ allow( dead_code, unused_imports ) ]
use threadpool::ThreadPool;
use std::path::PathBuf;
use structopt::StructOpt;
use strsim::{damerau_levenshtein,osa_distance, levenshtein, };
use crate::io::lines_from_file;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::thread;
use std::thread::{spawn, JoinHandle};
use std::borrow::Borrow;
use std::sync::{Arc, Mutex};
use std::fmt::Debug;

///
/// Compare to list of strings and find top N "best" matches in file2 for each string in file1
///
///
///

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

type StrCmpFn = fn(&str,&str) -> usize;

#[derive(StructOpt, Clone)]
#[structopt(
global_settings(&[structopt::clap::AppSettings::ColoredHelp, structopt::clap::AppSettings::VersionlessSubcommands, structopt::clap::AppSettings::DeriveDisplayOrder]),
//raw(setting = "structopt::clap::AppSettings::DeriveDisplayOrder"),
author, about
)]
pub struct CliCfg {
    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbosity - use more than one v for greater detail
    pub verbose: usize,

    #[structopt(parse(from_os_str))]
    ref_path: PathBuf,

    #[structopt(parse(from_os_str))]
    search_path: PathBuf,

    #[structopt(short = "d", default_value=",")]
    delimiter: String,

    #[structopt(short = "t", long="top_n_matches", default_value="3")]
    top_n_matches: usize,

    #[structopt(short = "a", long="str_simularity_algorithm", default_value="damerau_levenshtein", parse(try_from_str = str_to_sim_alg))]
    alg: StrCmpFn,
}

mod io;

#[derive(Eq, Debug)]
struct TrackedString {
    size: usize,
    string: String
}

fn str_to_sim_alg(s: &str) -> Result<StrCmpFn> {
    match s {
        "damerau_levenshtein" => Ok(damerau_levenshtein),
        "osa_distance" => Ok(osa_distance),
        "levenshtein" => Ok(levenshtein),
        _ => Err(format!("Error: string similarity algorithm \"{}\" unknown.  Must be one of damerau_levenshtein, osa_distance, or levenshtein", s))?,
    }
}

//??? BUG in rust?
fn alg_to_str(f: &fn(&str, &str) -> usize) -> &'static str {
    match f {
        &damerau_levenshtein => "damerau_levenshtein",
        &osa_distance => "osa_distance",
        &levenshtein => "levenshtein",
        _ => "uh",
    }
}

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

fn build_line(alg: StrCmpFn, result_idx: usize, results: &mut Arc<Mutex<Vec<String>>>, ref_str: &str, search_vec: &Vec<String>, limit: usize, del: &str) -> () {
    let mut heap = BinaryHeap::new();
    for search_str in search_vec.iter() {
        let val = alg(ref_str, search_str);
        heap.push(TrackedString{
            size: val,
            string: search_str.to_string(),
        });
    }
    let mut line = String::with_capacity(100);
    line.push_str(&ref_str);
    line.push_str(del);
    line.push_str(&format!("{}{}", heap.len(), del));
    for _i in 0 .. limit {
        match heap.pop() {
            Some(p) =>
                line.push_str(&format!("{},{}", p.size, &p.string)),
            None => break,
        }
        line.push_str(del);
    }
    let mut res = results.lock().unwrap();
    res[result_idx] = line;
    ()
}


fn main() {

    let cfg =CliCfg::from_args();
    // let str_sim_fn = match cfg.alg {
    //     "damerau_levenshtein" => damerau_levenshtein()| "osa_distance" | "osa_distance" =>
    // }
    // [algname] file1 file2
    let ref_vec = Arc::new(lines_from_file(&cfg.ref_path));
    let search_vec = Arc::new(lines_from_file(&cfg.search_path));

    println!("Each of {} strings in file: \"{}\" find top matches from the {} strings found in file: \"{}\" using algorithm \"{}\"",
    ref_vec.len(), &cfg.ref_path.to_str().unwrap(), search_vec.len(), &cfg.search_path.to_str().unwrap(), alg_to_str(&cfg.alg));
    let empty = String::new();
    let results : Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![empty;ref_vec.len()]));

    let pool = threadpool::Builder::new().build();
    let start_f = std::time::Instant::now();
    for (idx, ref_str) in ref_vec.clone().iter().enumerate() {
        let vec_search_clone = search_vec.clone();
        let ref_str_clone = ref_str.clone();
        let del = cfg.delimiter.clone();
        let top_n_matches = cfg.top_n_matches;
        let mut results_clone = results.clone();
        let alg = cfg.alg;
        pool.execute(move|| {build_line(alg, idx, &mut results_clone, &ref_str_clone, &vec_search_clone, top_n_matches, &del)});
        // println!("submit: {}", &ref_str);
    }

    pool.join();

    let res = results.lock().unwrap();
    for val in res.iter() {
        println!("{}", &val);
    }
    println!("done in {:.3} secs", start_f.elapsed().as_secs_f32());

}
