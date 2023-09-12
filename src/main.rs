use std::{fs, process::abort};

use input_linux::Key;
use theclicker::State;

fn main() {
    let mut cooldown = 25;
    let mut debug = false;
    let mut find_keycodes = false;
    let mut args: Vec<String> = Vec::new();
    let mut left_bind: u16 = Key::ButtonSide.into();
    let mut right_bind: u16 = Key::ButtonExtra.into();
    let mut beep = true;

    for arg in std::env::args() {
        args.push(arg);
    }

    for (index, arg) in std::env::args().enumerate() {
        if index > 0 {
            match arg.as_str() {
                "--clear-cache" => {
                    let _ = fs::remove_file("/tmp/TheClicker");
                }
                "--cooldown" => {
                    if let Some(arg) = args.get(index + 1) {
                        let num = match arg.trim().parse::<u64>() {
                            Ok(num) => num,
                            Err(_) => {
                                eprintln!("Specify a number!");
                                abort()
                            }
                        };
                        cooldown = num;
                    } else {
                        println!("After --cooldown specify the cooldown");
                        abort()
                    }
                }
                "--help" => {
                    println!("--clear-cache for cleaning the cache!");
                    println!("--cooldown [num] for set the cooldown in milliseconds!");
                    println!("--left-bind [keycode] bind left autoclicker to keycode");
                    println!("--right-bind [keycode] bind right autoclicker to keycode");
                    println!("--find-keycodes for finding what key is press");
                    println!("--no-beep for not beeping when the autoclicker state is changed");
                    return;
                }
                "--debug" => {
                    debug = true;
                    println!("Debugging!");
                }
                "--left-bind" => {
                    if let Some(arg) = args.get(index + 1) {
                        let num = match arg.trim().parse::<u16>() {
                            Ok(num) => num,
                            Err(_) => {
                                println!("Specify a keycode number!");
                                return;
                            }
                        };
                        left_bind = num;
                        println!("Left autoclicker is bind on {}", num);
                    } else {
                        eprintln!("No keycode after --left-bind");
                        abort()
                    }
                }
                "--right-bind" => {
                    if let Some(arg) = args.get(index + 1) {
                        let num = match arg.trim().parse::<u16>() {
                            Ok(num) => num,
                            Err(_) => {
                                println!("Specify a keycode number!");
                                return;
                            }
                        };
                        right_bind = num;
                        println!("Right autoclicker is bind on {}", num);
                    } else {
                        eprintln!("No keycode after --right-bind");
                        abort()
                    }
                }
                "--find-keycodes" => {
                    find_keycodes = true;
                    println!("Autoclicker inactive!");
                }
                "--no-beep" => {
                    beep = false;
                    println!("Beep off");
                }
                _ => {
                    println!("Invalid argument: {arg}");
                    abort();
                }
            }
        }
    }

    let state = State::new(cooldown, debug, find_keycodes, left_bind, right_bind, beep);
    println!("Launched!\n");

    println!("Cooldown is set to {}ms!", cooldown);

    state.main_loop();
}
