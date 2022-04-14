#![deny(unsafe_code)] // xd
#![allow(clippy::let_and_return)]

mod backpack_db;
mod botto;
mod wikiparse;

const DATA_DUMP_PATH: &str = "../data/ItemData-0.13.1.json";

fn main() {
    botto::main();
}
