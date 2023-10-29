pub mod args;
mod device;

use std::{
    io::{stdout, IsTerminal},
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::device::Device;
use input_linux::{sys::input_event, Key, KeyState};

#[derive(Default)]
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
    pub grab: bool,
    pub grab_kbd: bool,
    pub use_device: Option<String>,
}

pub struct State {
    input: Device,
    output: Arc<Device>,

    left_auto_clicker_bind: u16,
    right_auto_clicker_bind: u16,

    cooldown: Duration,
    cooldown_pr: Duration,
    debug: bool,
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
        }: StateArgs,
    ) -> Self {
        let input;

        if let Some(device_name) = use_device {
            if let Some(device) = Device::find_device(&device_name) {
                input = device;
            } else {
                eprintln!("Cannot find device: {device_name}");
                std::process::exit(1);
            }
        } else {
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
            find_keycodes,
            beep,
            cooldown_pr: Duration::from_millis(cooldown_press_release),
        }
    }
    pub fn main_loop(self) {
        let (transmitter, receiver) = mpsc::channel();

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

        let mut states = ToggleStates::default();

        thread::spawn(move || loop {
            input.read(&mut events).unwrap();

            for event in events.iter() {
                if debug {
                    println!("Event: {:?}", event);
                }

                if find_keycodes && event.value == 1 {
                    if let Ok(key) = Key::from_code(event.code) {
                        println!("Keycode: {}, key: {key:?}", event.code);
                    } else {
                        println!("Keycode: {}", event.code);
                    }
                }

                let mut used = false;
                let pressed = event.value == 1;
                if event.code == left_bind {
                    if pressed && !states.left && !find_keycodes {
                        transmitter.send(1).unwrap();
                    }
                    states.left = pressed;
                    used = true;
                }

                if event.code == right_bind {
                    if pressed && !states.right && !find_keycodes {
                        transmitter.send(2).unwrap();
                    }
                    states.right = pressed;
                    used = true;
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
                if recv == 1 {
                    toggle.left = !toggle.left;
                }
                if recv == 2 {
                    toggle.right = !toggle.right;
                }

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
