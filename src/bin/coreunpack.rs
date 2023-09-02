use clap::Parser;
use coretools_rs::unpack_folder;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[command(name = "coreunpack_rs", version)]
struct Args {
    mod_path: PathBuf,
    #[arg(short = 'o', default_value = "unpacked")]
    out_path: PathBuf,
    #[arg(short = 'c')]
    clear_path: bool,
}

fn main() {
    let args = Args::parse();

    if args.clear_path && args.out_path.exists() {
        if args.out_path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(&args.out_path) {
                println!("can't clear path: `{e}`");
            }
        } else {
            println!("cant't clear path: not a dir");
        }
    }
    
    let beg = std::time::Instant::now();
    unpack_folder(&args.mod_path, &args.out_path).unwrap();
    let end = std::time::Instant::now();
    println!("unpacked in {}ms", (end-beg).as_millis());
}
