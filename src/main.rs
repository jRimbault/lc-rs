use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    path: PathBuf,
    #[structopt(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::from_args();
    let entries: HashMap<PathBuf, Stats> = walker(&args.path, anaylize_entry);
    if args.verbose {
        for (filename, stats) in entries.iter() {
            println!("{}", filename.display());
            println!("{}", textwrap::indent(&stats.to_string(), "  "));
        }
    }
    let lines: Vec<usize> = entries.into_iter().flat_map(|(_, f)| f.lines).collect();
    let stats = Stats::from(lines);
    println!("{}", stats);
}

fn walker<P: AsRef<Path>, A, I, E, B>(path: &P, analyzer: A) -> B
where
    A: Fn(ignore::DirEntry) -> Result<I, E>,
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
                if result.is_err() {
                    return ignore::WalkState::Continue;
                }
                let entry = result.unwrap();
                sender.send(analyzer(entry)).unwrap();
                ignore::WalkState::Continue
            })
        });
    receiver.iter().filter_map(Result::ok).collect()
}

fn anaylize_entry(entry: ignore::DirEntry) -> anyhow::Result<(PathBuf, Stats)> {
    let path = entry.into_path();
    let file = BufReader::new(File::open(&path)?);
    let line_lengths: Vec<usize> = file
        .lines()
        .filter_map(Result::ok)
        .map(|l| l.chars().count())
        .collect();
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
        writeln!(
            f,
            "Length (mean ± σ):  {:>4.1} ±  {:>4.1}",
            self.mean, self.standard_deviation
        )?;
        write!(f, "Range (min … max):  {:>4} …  {:>4}", self.min, self.max)?;
        write!(f, " (excluding lengths < 2)")
    }
}
