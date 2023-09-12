mod device;

use std::{
    fs::File,
    io::{stdout, IsTerminal, Read},
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::device::Device;
use input_linux::{Key, KeyState};
use input_linux_sys::input_event;

#[derive(Default)]
pub struct ToggleStates {
    ///Primary click
    left: bool,

    //Secundary click
    right: bool,
}

pub struct State {
    mouse_input: Device,
    mouse_output: Device,

    left_auto_clicker_bind: u16,
    right_auto_clicker_bind: u16,

    cooldown: Duration,
    debug: bool,
    find_keycodes: bool,

    beep: bool,
}
impl State {
    fn try_from_cache() -> Device {
        match File::open("/tmp/TheClicker") {
            Ok(mut file) => {
                println!("Device loaded from cache!");
                let mut buffer = vec![];
                let length = file.read_to_end(&mut buffer).unwrap();
                let buffer = base64::decode(&buffer[..length]).unwrap();
                let content = String::from_utf8(buffer).unwrap();
                let device = Device::dev_open(PathBuf::from(content)).unwrap();
                println!("Device name: {}", device.name);
                println!("For cleaning cache, you can add --clear-cache");
                device
            }
            Err(_) => {
                println!("Select input device: ");
                Device::select_device()
            }
        }
    }

    pub fn new(
        cooldown: u64,
        debug: bool,
        find_keycodes: bool,
        l: u16,
        r: u16,
        beep: bool,
    ) -> Self {
        let mouse_input = Self::try_from_cache();
        let mouse_output = Device::uinput_open(PathBuf::from("/dev/uinput"), "ManClicker").unwrap();
        mouse_output.add_mouse_attributes();
        mouse_output.create();

        println!("Launching...");

        Self {
            mouse_input,
            mouse_output,

            left_auto_clicker_bind: l,
            right_auto_clicker_bind: r,

            cooldown: Duration::from_millis(cooldown),
            debug,
            find_keycodes,
            beep,
        }
    }
    pub fn main_loop(self) {
        let (transmitter, receiver) = mpsc::channel();

        let mut events: [input_event; 1] = unsafe { std::mem::zeroed() };
        let input = self.mouse_input;

        let debug = self.debug;
        let find_keycodes = self.find_keycodes;

        let left_bind = self.left_auto_clicker_bind;
        let right_bind = self.right_auto_clicker_bind;

        if let Ok(key) = Key::from_code(left_bind) {
            println!("Left bind: code: {left_bind}, key: {key:?}");
        } else {
            println!("Left bind: code: {left_bind}");
        }

        if let Ok(key) = Key::from_code(right_bind) {
            println!("Right bind: code: {right_bind}, key: {key:?}");
        } else {
            println!("Right bind: code: {right_bind}");
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

                let pressed = event.value == 1;
                if event.code == left_bind {
                    if pressed && !states.left && !find_keycodes {
                        transmitter.send(1).unwrap();
                    }
                    states.left = pressed;
                }

                if event.code == right_bind {
                    if pressed && !states.right && !find_keycodes {
                        transmitter.send(2).unwrap();
                    }
                    states.right = pressed;
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
                output.send_key(Key::ButtonLeft, KeyState::RELEASED);
            }
            if toggle.right {
                output.send_key(Key::ButtonRight, KeyState::PRESSED);
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
