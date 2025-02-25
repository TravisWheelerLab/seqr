use clap::Parser;
use seqr::{Cli, run};

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
