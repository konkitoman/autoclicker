use clap::Parser;
use theclicker::{Args, TheClicker};

fn main() {
    TheClicker::new(Args::parse()).main_loop();
}
