use std::{
    fs::{self, File},
    io::Read,
};

use clap::Parser;
use theclicker::{Args, DeviceType, State};

fn main() {
    let Args {
        clear_cache,
        mut cooldown,
        mut cooldown_press_release,
        mut left_bind,
        mut right_bind,
        mut find_keycodes,
        mut no_beep,
        mut debug,
        mut no_grab,
        mut use_device,
        mut grab_kbd,
        mut use_dev_path,
        mut is_keyboard,
        mut is_mouse,
    } = Args::parse();

    if clear_cache {
        let _ = fs::remove_file("/tmp/TheClicker");
    }

    if let Ok(mut file) = File::open("/tmp/TheClicker") {
        println!("Loaded from cache!");
        eprintln!("Args are disabled if we have cache!");
        eprintln!("You can use --clear-cache");
        let mut string = String::default();
        file.read_to_string(&mut string).unwrap();
        Args {
            clear_cache: _,
            cooldown,
            cooldown_press_release,
            left_bind,
            right_bind,
            find_keycodes,
            no_beep,
            debug,
            no_grab,
            grab_kbd,
            use_device,
            use_dev_path,
            is_keyboard,
            is_mouse,
        } = ron::from_str::<Args>(&string).unwrap();
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
        use_device,
        grab_kbd,
        use_dev_path,
        device_type: match (is_mouse, is_keyboard) {
            (true, false) => Some(DeviceType::Mouse),
            (false, true) => Some(DeviceType::Keyboard),
            (false, false) => None,
            _ => {
                eprintln!("You cannot set both -K and -M");
                std::process::exit(4)
            }
        },
    });

    println!();
    println!("Cooldown is {}ms!", cooldown);
    println!(
        "Cooldown between press and release is {}ms!",
        cooldown_press_release
    );

    state.main_loop();
}
