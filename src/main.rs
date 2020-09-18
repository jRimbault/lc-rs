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
    let entries: HashMap<PathBuf, Stats> = walker(&args.path, anaylize_entry);
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

type IgnoreResult = Result<ignore::DirEntry, ignore::Error>;

fn walker<P: AsRef<Path>, A, I, E, B>(path: &P, walker_fn: A) -> B
where
    A: Fn(IgnoreResult) -> Result<I, E>,
    A: Send + Copy,
    B: std::iter::FromIterator<I>,
    I: Send,
    E: Send,
{
    let (sender, receiver) = std::sync::mpsc::channel();
    ignore::WalkBuilder::new(&path)
        .build_parallel()
        .run(move || {
            let sender = sender.clone();
            Box::new(move |result| {
                sender.send(walker_fn(result)).unwrap();
                ignore::WalkState::Continue
            })
        });
    receiver.iter().filter_map(Result::ok).collect()
}

fn anaylize_entry(entry: IgnoreResult) -> anyhow::Result<(PathBuf, Stats)> {
    let path = entry?.into_path();
    if !path.is_file() {
        return Err(anyhow::anyhow!("not a file"));
    }
    log::debug!("processing {}", path.display());
    let file = BufReader::new(File::open(&path)?);
    let line_lengths: Vec<usize> = file
        .lines()
        .filter_map(Result::ok)
        .map(|l| l.chars().count())
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
