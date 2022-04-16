#![deny(unsafe_code)] // xd
#![allow(clippy::let_and_return)]

mod backpack_db;
mod botto;
mod wikiparse;

use clap::Parser;

#[derive(clap::Parser, Debug)]
struct Args{
    #[clap(long)]
    data: String,
    #[clap(long)]
    summary: Option<String>,
}

fn main() {
    let args = Args::parse();
    botto::main( &args);
}
