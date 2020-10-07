use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Display statistics on line lengths
#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    path: PathBuf,
    #[structopt(short, long)]
    verbose: bool,
}

#[cfg(debug_assertions)]
const LOG_LEVEL: &str = concat!(env!("CARGO_PKG_NAME"), "=debug");
#[cfg(not(debug_assertions))]
const LOG_LEVEL: &str = "error";

fn main() {
    env_logger::from_env(env_logger::Env::default().default_filter_or(LOG_LEVEL)).init();
    let args = Args::from_args();
    let entries = walk_dir(&args.path);
    if args.verbose {
        for (filename, stats) in entries.iter() {
            println!("{}", filename.display());
            println!("{:^}", stats);
            println!();
        }
    }
    let lines: Vec<usize> = entries.into_iter().flat_map(|(_, f)| f.lines).collect();
    println!("{}", Stats::from(lines));
}

fn walk_dir<P: AsRef<Path>>(path: &P) -> HashMap<PathBuf, Stats> {
    ignore::WalkBuilder::new(path)
        .build_parallel()
        .map(analyze_entry)
        .filter_map(Result::ok)
        .collect()
}

fn analyze_entry(entry: ignore::DirEntry) -> anyhow::Result<(PathBuf, Stats)> {
    let path = entry.into_path();
    if !path.is_file() {
        return Err(anyhow::anyhow!("not a file"));
    }
    log::debug!("processing {}", path.display());
    let file = BufReader::new(File::open(&path)?);
    let line_lengths: Vec<usize> = file
        .lines()
        .filter_map(Result::ok)
        .map(|l| l.chars().count())
        .filter(|&l| l > 0)
        .collect();
    if line_lengths.len() < 2 {
        log::warn!("{} only has one line", path.display());
        return Err(anyhow::anyhow!("not enough lines to be significant"));
    }
    Ok((path, Stats::from(line_lengths)))
}

#[derive(Debug, Default)]
struct Stats {
    lines: Vec<usize>,
    max: usize,
    min: usize,
    mean: f64,
    median: f64,
    standard_deviation: f64,
}

impl From<Vec<usize>> for Stats {
    fn from(list: Vec<usize>) -> Self {
        let flist: Vec<_> = list.iter().copied().map(|i| i as _).collect();
        Stats {
            max: list.iter().copied().max().unwrap_or(0),
            min: list
                .iter()
                .copied()
                .filter(|i| ![0, 1].contains(i))
                .min()
                .unwrap_or(0),
            mean: statistical::mean(&flist),
            median: statistical::median(&flist),
            standard_deviation: statistical::standard_deviation(&flist, None),
            lines: list,
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match f.align() {
            Some(fmt::Alignment::Center) => {
                writeln!(
                    f,
                    "  Length (mean ± σ):  {:>4.1} ±  {:>4.1}",
                    self.mean, self.standard_deviation
                )?;
                write!(
                    f,
                    "  Range (min … max):  {:>4} …  {:>4} (excluding lengths < 2)",
                    self.min, self.max
                )
            }
            _ => {
                writeln!(
                    f,
                    "Length (mean ± σ):  {:>4.1} ±  {:>4.1}",
                    self.mean, self.standard_deviation
                )?;
                write!(
                    f,
                    "Range (min … max):  {:>4} …  {:>4} (excluding lengths < 2)",
                    self.min, self.max
                )
            }
        }
    }
}

trait WalkParallelMap {
    fn map<F, I>(self, fnmap: F) -> std::sync::mpsc::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send;
}

impl WalkParallelMap for ignore::WalkParallel {
    fn map<F, I>(self, fnmap: F) -> std::sync::mpsc::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send,
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.run(move || {
            let sender = sender.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {
                    sender.send(fnmap(entry)).unwrap();
                }
                ignore::WalkState::Continue
            })
        });
        receiver.into_iter()
    }
}
