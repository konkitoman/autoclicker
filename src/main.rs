use clap::Parser;
use theclicker::{Args, State};

fn main() {
    State::new(Args::parse()).main_loop();
}
