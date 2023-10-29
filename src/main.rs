pub mod args;
pub mod device;

use std::fs;

use clap::Parser;
use theclicker::State;

use crate::args::Args;

fn main() {
    let Args {
        clear_cache,
        cooldown,
        cooldown_press_release,
        left_bind,
        right_bind,
        find_keycodes,
        no_beep,
        debug,
        no_grab,
        use_dev,
    } = Args::parse();

    if clear_cache {
        let _ = fs::remove_file("/tmp/TheClicker");
    }

    let beep = !no_beep;
    let grab = !no_grab;

    let state = State::new(theclicker::StateArgs {
        cooldown,
        cooldown_press_release,
        left_bind,
        right_bind,
        find_keycodes,
        beep,
        debug,
        grab,
        use_dev,
    });

    println!();
    println!("Cooldown is {}ms!", cooldown);
    println!(
        "Cooldown between press and release is {}ms!",
        cooldown_press_release
    );

    state.main_loop();
}
