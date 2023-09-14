pub mod args;
pub mod device;

use std::fs;

use clap::Parser;
use theclicker::State;

use crate::args::Args;

fn main() {
    let parse = Args::parse();

    let mut beep = true;

    if parse.clear_cache {
        let _ = fs::remove_file("/tmp/TheClicker");
    }

    if parse.no_beep {
        beep = false;
    }

    let state = State::new(
        parse.cooldown,
        parse.cooldown_press_release,
        parse.debug,
        parse.find_keycodes,
        parse.left_bind,
        parse.right_bind,
        beep,
    );
    println!("Launched!\n");
    println!("Cooldown is set to {}ms!", parse.cooldown);
    println!(
        "Cooldown between press and release is set to {}ms!",
        parse.cooldown_press_release
    );

    state.main_loop();
}
