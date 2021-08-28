use std::{
    fs,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, SystemTime},
};

mod device;

use device::Device;
use input_linux::{EventTime, InputEvent, Key, KeyEvent, KeyState, SynchronizeEvent};
use input_linux_sys::{input_event, timeval};

const EMPTY_INPUT_EVENT: input_event = input_event {
    time: timeval {
        tv_sec: 0,
        tv_usec: 0,
    },
    type_: 0,
    code: 0,
    value: 0,
};

pub struct ToggleStates {
    ///Primary click
    left: bool,
    //Secundary click
    right: bool,
}
impl ToggleStates {
    pub fn new() -> Self {
        Self {
            left: false,
            right: false,
        }
    }
}
pub struct ButtonLastStates {
    /// Mouse 4
    extra: bool,
    ///Mouse 5
    side: bool,
}
impl ButtonLastStates {
    pub fn new() -> Self {
        Self {
            extra: true,
            side: true,
        }
    }
}

pub struct State {
    mouse_input: Device,
    mouse_output: Device,

    button_states: ButtonLastStates,
    toggle_states: ToggleStates,

    left_auto_clicker_bind: u16,
    right_auto_clicker_bind: u16,

    cooldown: Duration,
    debug: bool,
    find_keycodes: bool,
}
impl State {
    fn try_from_cache() -> Device {
        match File::open("/tmp/TheClicker") {
            Ok(mut file) => {
                println!("Device loaded from cache!");
                let mut buffer = vec![0];
                let lenght = file.read_to_end(&mut buffer).unwrap();
                let buffer = &buffer[1..lenght];
                let buffer = base64::decode(buffer).unwrap();
                let content = std::str::from_utf8(&buffer).unwrap();
                let device = Device::dev_open(PathBuf::from(content)).unwrap();
                println!("Device name: {}", device.name);
                println!("For cleaning cache plese add --clear-cache");
                device
            }
            Err(_) => Device::select_device(),
        }
    }

    pub fn new(cooldown: u64, debug: bool, find_keycodes: bool, l: u16, r: u16) -> Self {
        let mouse_input = Self::try_from_cache();
        let mouse_output = Device::uinput_open(PathBuf::from("/dev/uinput"), "ManClicker").unwrap();
        mouse_output.add_mouse_atributes();
        mouse_output.create();

        println!("Launching...");

        Self {
            mouse_input,
            mouse_output,

            button_states: ButtonLastStates::new(),
            toggle_states: ToggleStates::new(),

            left_auto_clicker_bind: l,
            right_auto_clicker_bind: r,

            cooldown: Duration::from_millis(cooldown),
            debug,
            find_keycodes,
        }
    }
    #[allow(unreachable_code)]
    pub fn main_loop(self) {
        let (transmiter, reciver) = mpsc::channel();

        let mut events = [EMPTY_INPUT_EVENT; 1];
        let input = self.mouse_input;
        let mut states = self.button_states;

        let debug = self.debug;
        let find_keycodes = self.find_keycodes;

        let left_bind = self.left_auto_clicker_bind;
        let right_bind = self.right_auto_clicker_bind;

        thread::spawn(move || loop {
            //geting events
            input.read(&mut events).unwrap();
            //Repeat for evry event
            for event in events.iter() {
                if debug {
                    println!("Event: {:?}", event);
                }
                if find_keycodes {
                    if event.value == 1 {
                        println!("Keycode: {}", event.code);
                    }
                }
                if event.code == left_bind {
                    if event.value == 1 {
                        if states.side {
                            if !find_keycodes {
                                transmiter.send(1).unwrap();
                            }
                        }
                        states.side = false;
                    } else {
                        states.side = true;
                    }
                }
                if event.code == right_bind {
                    if event.value == 1 {
                        if states.extra {
                            if !find_keycodes {
                                transmiter.send(2).unwrap();
                            }
                        }
                        states.extra = false;
                    } else {
                        states.extra = true;
                    }
                }
            }
        });

        let mut toggle = self.toggle_states;
        let output = self.mouse_output;
        loop {
            match reciver.try_recv() {
                Ok(toglev) => {
                    if toglev == 1 {
                        toggle.left = !toggle.left;
                    }
                    if toglev == 2 {
                        toggle.right = !toggle.right;
                    }
                }
                Err(_) => {}
            }
            if toggle.left {
                let events: [input_event; 2] = [
                    InputEvent::from(KeyEvent::new(
                        get_current_time(),
                        Key::ButtonLeft,
                        KeyState::pressed(true),
                    ))
                    .as_raw()
                    .to_owned(),
                    InputEvent::from(SynchronizeEvent::report(get_current_time()))
                        .as_raw()
                        .to_owned(),
                ];
                output.write(&events).unwrap();
                let events: [input_event; 2] = [
                    InputEvent::from(KeyEvent::new(
                        get_current_time(),
                        Key::ButtonLeft,
                        KeyState::pressed(false),
                    ))
                    .as_raw()
                    .to_owned(),
                    InputEvent::from(SynchronizeEvent::report(get_current_time()))
                        .as_raw()
                        .to_owned(),
                ];
                output.write(&events).unwrap();
            }
            if toggle.right {
                let events: [input_event; 2] = [
                    InputEvent::from(KeyEvent::new(
                        get_current_time(),
                        Key::ButtonRight,
                        KeyState::pressed(true),
                    ))
                    .as_raw()
                    .to_owned(),
                    InputEvent::from(SynchronizeEvent::report(get_current_time()))
                        .as_raw()
                        .to_owned(),
                ];
                output.write(&events).unwrap();
                let events: [input_event; 2] = [
                    InputEvent::from(KeyEvent::new(
                        get_current_time(),
                        Key::ButtonRight,
                        KeyState::pressed(false),
                    ))
                    .as_raw()
                    .to_owned(),
                    InputEvent::from(SynchronizeEvent::report(get_current_time()))
                        .as_raw()
                        .to_owned(),
                ];
                output.write(&events).unwrap();
            }
            thread::sleep(self.cooldown);
        }
    }
}

fn main() {
    let mut cooldown = 25;
    let mut debug = false;
    let mut find_keycodes = false;
    let mut args: Vec<String> = Vec::new();
    let mut left_bind: u16 = Key::ButtonSide.into();
    let mut right_bind: u16 = Key::ButtonExtra.into();
    for arg in std::env::args() {
        args.push(arg);
    }
    for (index, arg) in std::env::args().enumerate() {
        if index > 0 {
            match arg.as_str() {
                "--clear-cache" => {
                    fs::remove_file("/tmp/TheClicker").unwrap();
                }
                "--cooldown" => {
                    if args.len() <= index + 1 {
                        println!("Plese specify a number for cooldown!");
                        return;
                    }
                    let num = match args[index + 1].trim().parse::<u64>() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Specifi a number!");
                            return;
                        }
                    };
                    cooldown = num;
                }
                "--help" => {
                    println!("--clear-cache for cleaning the cache!");
                    println!("--cooldown [num] for set the cooldown!");
                    println!("--left-bind [keycode] bind left autoclicker to keycode");
                    println!("--right-bind [keycode] bind left autoclicker to keycode");
                    println!("--find-keycodes for finding what key is press");
                    return;
                }
                "--debug" => {
                    debug = true;
                    println!("Debuging!");
                }
                "--left-bind" => {
                    let num = match args[index + 1].trim().parse::<u16>() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Specifi a keycode number!");
                            return;
                        }
                    };
                    left_bind = num;
                    println!("Left autoclicker is bind on {}", num);
                }
                "--right-bind" => {
                    let num = match args[index + 1].trim().parse::<u16>() {
                        Ok(num) => num,
                        Err(_) => {
                            println!("Specifi a keycode number!");
                            return;
                        }
                    };
                    right_bind = num;
                    println!("Right autoclicker is bind on {}", num);
                }
                "--find-keycodes" => {
                    find_keycodes = true;
                    println!("Autoclicker inactive!");
                }
                _ => {}
            }
        }
    }

    let state = State::new(cooldown, debug, find_keycodes, left_bind, right_bind);
    println!("Launched!\n");

    println!("Cooldown is set to {}!", cooldown);

    state.main_loop();
}

fn get_current_time() -> EventTime {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    EventTime::new(time.as_secs() as i64, time.as_micros() as i64)
}
