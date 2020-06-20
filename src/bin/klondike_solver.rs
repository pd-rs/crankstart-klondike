#![allow(unused, dead_code)]
use anyhow::Error;

#[path = "../klondike.rs"]
mod klondike;

use crate::klondike::Table;

fn main() -> Result<(), Error> {
    let table = Table::new(321);
    println!("table = {:#?}", table);
    Ok(())
}
