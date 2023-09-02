use clap::Parser;
use coretools_rs::pack_folder;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
#[command(name = "corepack_rs", version)]
struct Args {
    #[arg(name = "MOD DIR")]
    mod_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let beg = std::time::Instant::now();
    pack_folder(&args.mod_path).unwrap();
    let end = std::time::Instant::now();
    println!("packed in {}ms", (end-beg).as_millis());
}