mod args;
mod device;

pub use args::Args;

use std::{
    io::{stdout, IsTerminal},
    path::PathBuf,
    str::FromStr,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

pub use device::{Device, DeviceType};
use input_linux::{sys::input_event, Key, KeyState};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct ToggleStates {
    ///Primary click
    left: bool,

    //Secundary click
    right: bool,
}

pub struct StateArgs {
    pub cooldown: u64,
    pub cooldown_press_release: u64,
    pub left_bind: Option<u16>,
    pub right_bind: Option<u16>,
    pub find_keycodes: bool,
    pub beep: bool,
    pub debug: bool,
    pub hold: bool,
    pub grab: bool,
    pub grab_kbd: bool,
    pub use_device: Option<String>,
    pub use_dev_path: Option<String>,
    pub device_type: Option<DeviceType>,
}

pub struct State {
    input: Device,
    output: Arc<Device>,

    left_auto_clicker_bind: u16,
    right_auto_clicker_bind: u16,

    cooldown: Duration,
    cooldown_pr: Duration,
    debug: bool,
    hold: bool,
    find_keycodes: bool,

    beep: bool,
}

impl State {
    pub fn new(
        StateArgs {
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
            device_type,
            hold,
        }: StateArgs,
    ) -> Self {
        let input;

        'try_set_input: {
            if let Some(dev_path) = use_dev_path {
                let Ok(path) = PathBuf::from_str(&dev_path) else {
                    eprintln!("Cannot make {dev_path} to path, invalid path");
                    std::process::exit(3);
                };

                let Some(device_type) = device_type else {
                    eprintln!("You didn't specified the device type!");
                    std::process::exit(5);
                };

                if let Ok(device) = Device::dev_open(path, device_type) {
                    input = device;
                    break 'try_set_input;
                }

                eprintln!("Cannot open device: {dev_path}");
                std::process::exit(2);
            }

            if let Some(device_name) = use_device {
                if let Some(device) = Device::find_device(&device_name) {
                    input = device;
                    break 'try_set_input;
                }

                eprintln!("Cannot find device: {device_name}");
                std::process::exit(1);
            }

            input = Device::select_device();
        }

        println!("Using: {}", input.name);

        let output = Device::uinput_open(PathBuf::from("/dev/uinput"), "TheClicker").unwrap();
        output.add_mouse_attributes();

        if grab {
            if input.ty.is_keyboard() && !grab_kbd {
                eprintln!("Grab mode is disabled for keyboard!");
                eprintln!("You can use --grab-kbd to override that");
            } else {
                output.copy_attributes(&input);
                input.grab(true).expect("Cannot grab the input device!");
            }
        }

        output.create();

        let left_bind = left_bind.unwrap_or(match input.ty {
            device::DeviceType::Mouse => 275,
            device::DeviceType::Keyboard => 26,
        });

        let right_bind = right_bind.unwrap_or(match input.ty {
            device::DeviceType::Mouse => 276,
            device::DeviceType::Keyboard => 27,
        });

        Self {
            input,
            output: Arc::new(output),

            left_auto_clicker_bind: left_bind,
            right_auto_clicker_bind: right_bind,

            cooldown: Duration::from_millis(cooldown),
            debug,
            hold,
            find_keycodes,
            beep,
            cooldown_pr: Duration::from_millis(cooldown_press_release),
        }
    }
    pub fn main_loop(self) {
        let (transmitter, receiver) = mpsc::channel::<ToggleStates>();

        let mut events: [input_event; 1] = unsafe { std::mem::zeroed() };
        let input = self.input;
        let output = self.output.clone();

        let debug = self.debug;
        let find_keycodes = self.find_keycodes;

        let left_bind = self.left_auto_clicker_bind;
        let right_bind = self.right_auto_clicker_bind;

        if let Ok(key) = Key::from_code(left_bind) {
            println!("Left bind code: {left_bind}, key: {key:?}");
        } else {
            println!("Left bind code: {left_bind}");
        }

        if let Ok(key) = Key::from_code(right_bind) {
            println!("Right bind code: {right_bind}, key: {key:?}");
        } else {
            println!("Right bind code: {right_bind}");
        }

        let mut state = ToggleStates::default();
        let hold = self.hold;

        thread::spawn(move || loop {
            input.read(&mut events).unwrap();

            for event in events.iter() {
                if debug {
                    println!("Event: {:?}", event);
                }

                let mut used = false;
                'handle_events: {
                    if find_keycodes && event.value == 1 {
                        if let Ok(key) = Key::from_code(event.code) {
                            println!("Keycode: {}, key: {key:?}", event.code);
                        } else {
                            println!("Keycode: {}", event.code);
                        }
                        break 'handle_events;
                    }

                    let old_state = state;

                    let pressed = event.value == 1 || event.value == 2;;
                    for (bind, s) in [(left_bind, &mut state.left), (right_bind, &mut state.right)]
                    {
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
                }

                if !used {
                    output
                        .write(&events)
                        .expect("Cannot write to virtual device!");
                }
            }
        });

        let mut toggle = ToggleStates::default();
        let beep = self.beep;
        println!();
        print_active(&toggle);

        let output = self.output;
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
