mod args;
mod device;

pub use args::Args;

use std::{
    io::{stdout, IsTerminal, Write},
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

pub use device::{Device, DeviceType};
use input_linux::{sys::input_event, Key, KeyState};

pub struct KeyCode(u16);

impl std::fmt::Display for KeyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.0;
        f.write_fmt(format_args!("KeyCode: {code}"))?;
        if let Ok(key) = Key::from_code(code) {
            f.write_fmt(format_args!(", Key: {key:?}"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct ToggleStates {
    ///Primary click
    left: bool,

    //Secundary click
    right: bool,
}

pub struct StateNormal {
    left_bind: u16,
    right_bind: u16,

    hold: bool,
    grab: bool,

    cooldown: Duration,
    cooldown_pr: Duration,
}

impl StateNormal {
    pub fn run(self, shared: Shared) {
        let (transmitter, receiver) = mpsc::channel::<ToggleStates>();

        let mut events: [input_event; 1] = unsafe { std::mem::zeroed() };
        let input = shared.input;
        let output = shared.output.clone();

        let left_bind = self.left_bind;
        let right_bind = self.right_bind;

        let debug = shared.debug;
        let grab = self.grab;

        let mut state = ToggleStates::default();
        let hold = self.hold;

        thread::spawn(move || loop {
            input.read(&mut events).unwrap();

            for event in events.iter() {
                if debug {
                    println!("Event: {:?}", event);
                }

                let mut used = false;
                let old_state = state;

                let pressed = matches!(event.value, 1 | 2);
                for (bind, s) in [(left_bind, &mut state.left), (right_bind, &mut state.right)] {
                    if event.code == bind {
                        if hold {
                            if pressed != *s {
                                *s = pressed;
                            }
                        } else if pressed {
                            *s = !*s;
                        }
                        used = true;
                    }
                }

                if old_state != state {
                    transmitter.send(state).unwrap();
                }

                if grab && !used {
                    output
                        .write(&events)
                        .expect("Cannot write to virtual device!");
                }
            }
        });

        let mut toggle = ToggleStates::default();
        let beep = shared.beep;
        println!();
        print_active(&toggle);

        let output = shared.output;
        loop {
            if let Some(recv) = if toggle.left | toggle.right {
                receiver.try_recv().ok()
            } else {
                receiver.recv().ok()
            } {
                toggle = recv;

                if beep {
                    // ansi beep sound
                    print!("\x07");
                }

                print_active(&toggle);
            }

            if toggle.left {
                output.send_key(Key::ButtonLeft, KeyState::PRESSED);
            }
            if toggle.right {
                output.send_key(Key::ButtonRight, KeyState::PRESSED);
            }

            if !self.cooldown_pr.is_zero() {
                thread::sleep(self.cooldown_pr);
            }

            if toggle.left {
                output.send_key(Key::ButtonLeft, KeyState::RELEASED);
            }

            if toggle.right {
                output.send_key(Key::ButtonRight, KeyState::RELEASED);
            }
            thread::sleep(self.cooldown);
        }
    }
}

pub enum Variant {
    Normal(StateNormal),
    Legacy {},
}

impl Variant {
    pub fn run(self, shared: Shared) {
        match self {
            Variant::Normal(state_normal) => state_normal.run(shared),
            Variant::Legacy {} => todo!(),
        }
    }
}

pub struct Shared {
    debug: bool,
    beep: bool,
    input: Device,
    output: Arc<Device>,
}

pub struct TheClicker {
    shared: Shared,
    variant: Variant,
}

impl TheClicker {
    pub fn new(
        Args {
            debug,
            beep,
            command,
        }: Args,
    ) -> Self {
        let output = Device::uinput_open(PathBuf::from("/dev/uinput"), "TheClicker").unwrap();
        output.add_mouse_attributes();

        let command = command.unwrap_or_else(command_from_user_input);

        print!("Using args: `");
        if debug {
            print!("--debug ")
        }
        if beep {
            print!("--beep ")
        }
        match command {
            args::Command::Run {
                device_query,
                left_bind,
                right_bind,
                hold,
                grab,
                cooldown,
                cooldown_press_release,
            } => {
                print!("run -d{device_query:?} -l{left_bind} -r{right_bind} -c{cooldown} -C{cooldown_press_release}");
                if hold {
                    print!(" -H")
                }
                if grab {
                    print!(" --grab")
                }
                println!("`");

                let input = 'try_set_input: {
                    if device_query.is_empty() {
                        eprintln!("Device query is empty!");
                        std::process::exit(1);
                    }

                    if device_query.starts_with('/') {
                        let Ok(device) = Device::dev_open(PathBuf::from(&device_query)) else {
                            eprintln!("Cannot open device: {device_query}");
                            std::process::exit(2);
                        };
                        break 'try_set_input device;
                    } else {
                        let Some(device) = Device::find_device(&device_query) else {
                            eprintln!("Cannot find device: {device_query}");

                            std::process::exit(3);
                        };
                        break 'try_set_input device;
                    }
                };

                if grab {
                    output.copy_attributes(debug, &input);
                    input.grab(true).expect("Cannot grab input device!");
                }

                output.create();

                Self {
                    shared: Shared {
                        debug,
                        beep,
                        input,
                        output: Arc::new(output),
                    },
                    variant: Variant::Normal(StateNormal {
                        left_bind,
                        right_bind,
                        hold,
                        grab,
                        cooldown: Duration::from_millis(cooldown),
                        cooldown_pr: Duration::from_millis(cooldown_press_release),
                    }),
                }
            }
            args::Command::RunLegecy {} => todo!(),
        }
    }

    pub fn main_loop(self) {
        self.variant.run(self.shared);
    }
}

fn print_active(toggle: &ToggleStates) {
    let is_terminal = stdout().is_terminal();

    if is_terminal {
        print!("\x1b[0K");
    }

    print!("Active: ");
    if toggle.left {
        print!("left")
    }
    if toggle.right {
        if toggle.left {
            print!(", ")
        }
        print!("right")
    }
    println!();

    if is_terminal {
        print!("\x1b[1F");
    }
}

fn command_from_user_input() -> args::Command {
    let input_device = Device::select_device();

    println!("Device name: {}", input_device.name);

    let legacy =
        input_device.filename.starts_with("mouse") || input_device.filename.starts_with("mice");

    if legacy {
        eprintln!("\x1B[1;31mThe legacy interface is not implemented, you cannot use `/dev/input/mouse{{N}}` or `/dev/input/mice`!\x1B[0;39m");
        unimplemented!();
    } else {
        let left_bind = choose_key(&input_device, "left_bind");
        let right_bind = choose_key(&input_device, "right_bind");
        let hold = choose_yes("You want to hold the bind / active hold_mode?", true);
        println!("\x1B[1;33mWarning: if you enable grab mode you can get softlocked\x1B[1;39m, if the compositor will not use TheClicker device.");
        println!("If the device input is grabed, the input device will be emulated by TheClicker, and when you press a binding that will not be sent");
        let grab = choose_yes("You want to grab the input device?", false);
        println!("Grab: {grab}");
        let cooldown = choose_usize("Choose cooldown, the min is 25", 25) as u64;
        let cooldown_press_release =
            choose_usize("Choose cooldown between press and release", 0) as u64;

        args::Command::Run {
            left_bind,
            right_bind,
            hold,
            grab,
            cooldown,
            cooldown_press_release,
            device_query: input_device.path.to_str().unwrap().to_owned(),
        }
    }
}

fn choose_key(input_device: &Device, name: &str) -> u16 {
    let mut events: [input_linux::sys::input_event; 1] = unsafe { std::mem::zeroed() };
    println!("Waiting for key presses from the selected device");
    loop {
        input_device.empty_read_buffer();
        println!("Choose key for {name}:");
        'outer: while let Ok(len) = input_device.read(&mut events) {
            for event in &events[..len] {
                if event.type_ == input_linux::sys::EV_KEY as u16 && matches!(event.value, 1 | 2) {
                    break 'outer;
                }
            }
        }

        println!("\t{}", KeyCode(events[0].code));

        if choose_yes("You want to choose this", true) {
            break events[0].code;
        }
    }
}

fn choose_yes(message: impl std::fmt::Display, default: bool) -> bool {
    println!(
        "\x1B[1;39m{message} [{}]\x1B[0;39m",
        if default { "Y/n" } else { "y/N" }
    );
    print!("-> ");
    _ = std::io::stdout().flush();

    let response = std::io::stdin()
        .lines()
        .next()
        .expect("Cannot read from stdin")
        .expect("Cannot read from stdin");

    matches!(response.as_str().trim(), "Yes" | "yes" | "Y" | "y")
        || (default && response.is_empty())
}

fn choose_usize(message: impl std::fmt::Display, default: usize) -> usize {
    loop {
        print!("\x1B[1;39m{message} [\x1B[1;32m{default}\x1B[0;39m]\x1B[0;39m: \x1B[1;32m",);
        _ = std::io::stdout().flush();
        let response = std::io::stdin()
            .lines()
            .next()
            .expect("Cannot read from stdin")
            .expect("Cannot read from stdin");
        print!("\x1B[0;39m");
        _ = std::io::stdout().flush();

        if response.is_empty() {
            return default;
        }

        let Ok(num) = response.parse::<usize>() else {
            println!("{response:?} Is not a number!");
            continue;
        };

        return num;
    }
}
