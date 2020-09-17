use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    path: PathBuf,
}

fn main() {
    let args = Args::from_args();
    let (sender, receiver) = std::sync::mpsc::channel();
    ignore::WalkBuilder::new(&args.path)
        .build_parallel()
        .run(move || {
            let sender = sender.clone();
            Box::new(move |result| {
                if result.is_err() {
                    return ignore::WalkState::Continue;
                }
                let entry = result.unwrap();
                sender.send(anaylize_entry(entry)).unwrap();
                ignore::WalkState::Continue
            })
        });
    let entries: std::collections::HashMap<PathBuf, FileStats> =
        receiver.iter().filter_map(Result::ok).collect();
    println!("{:#?}", entries);
}

fn anaylize_entry(entry: ignore::DirEntry) -> anyhow::Result<(PathBuf, FileStats)> {
    let file = BufReader::new(File::open(entry.path())?);
    let line_lengths: Vec<usize> = file
        .lines()
        .filter_map(Result::ok)
        .map(|l| l.chars().count())
        .collect();
    Ok((entry.path().into(), FileStats::from(line_lengths.as_ref())))
}

#[derive(Debug, Default)]
struct FileStats {
    max: usize,
    mean: f64,
    median: f64,
}

impl From<&[usize]> for FileStats {
    fn from(list: &[usize]) -> Self {
        FileStats {
            max: *list.iter().max().unwrap_or(&0),
            mean: stats::mean(list.iter().copied()),
            median: stats::median(list.iter().copied()).unwrap_or(0.),
        }
    }
}
