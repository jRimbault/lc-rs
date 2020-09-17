use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    path: std::path::PathBuf,
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
                sender.send(entry).unwrap();
                ignore::WalkState::Continue
            })
        });
    let entries: Vec<ignore::DirEntry> = receiver.iter().collect();
    println!("{:?}", entries);
}
