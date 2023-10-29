pub mod args;
mod device;

use std::{
    fs::File,
    io::{stdout, IsTerminal, Read},
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::device::Device;
use base64::prelude::*;
use input_linux::{Key, KeyState};
use input_linux_sys::input_event;

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
    pub left_bind: u16,
    pub right_bind: u16,
    pub find_keycodes: bool,
    pub beep: bool,
    pub debug: bool,
    pub grab: bool,
    pub use_dev: String,
}

pub struct State {
    mouse_input: Device,
    mouse_output: Arc<Device>,

    left_auto_clicker_bind: u16,
    right_auto_clicker_bind: u16,

    cooldown: Duration,
    cooldown_pr: Duration,
    debug: bool,
    find_keycodes: bool,

    beep: bool,
}

impl State {
    fn try_from_cache(use_dev: String) -> Device {
        match File::open("/tmp/TheClicker") {
            Ok(mut file) => {
                println!("Device loaded from cache!");
                let mut buffer = vec![];
                let length = file.read_to_end(&mut buffer).unwrap();
                let buffer = BASE64_STANDARD.decode(&buffer[..length]).unwrap();
                let content = String::from_utf8(buffer).unwrap();
                let device = Device::dev_open(PathBuf::from(content)).unwrap();
                println!("Device name: {}", device.name);
                println!("For cleaning cache, you can add --clear-cache");
                device
            }
            Err(_) => Device::select_device(use_dev),
        }
    }

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
            use_dev,
        }: StateArgs,
    ) -> Self {
        let mouse_input = Self::try_from_cache(use_dev);
        let mouse_output = Device::uinput_open(PathBuf::from("/dev/uinput"), "ManClicker").unwrap();
        mouse_output.add_mouse_attributes();

        if grab {
            mouse_output.copy_attributes(&mouse_input);
            mouse_input
                .grab(true)
                .expect("Cannot grab the input device!");
        }

        mouse_output.create();

        Self {
            mouse_input,
            mouse_output: Arc::new(mouse_output),

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
        let input = self.mouse_input;
        let output = self.mouse_output.clone();

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

        let output = self.mouse_output;
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
