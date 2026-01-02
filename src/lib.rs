mod args;
mod device;

pub use args::Args;

use std::{
    io::{IsTerminal, Write, stdout},
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

pub use device::{DeviceType, InputDevice, OutputDevice};
use input_linux::{Key, KeyState, sys::input_event};

const WAIT_KEY_RELEASE: std::time::Duration = std::time::Duration::from_millis(100);

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
pub struct AutoclickerState {
    left: bool,
    middle: bool,
    right: bool,
    lock: bool,
}

pub struct StateNormal {
    left_bind: Option<u16>,
    middle_bind: Option<u16>,
    right_bind: Option<u16>,

    lock_unlock_bind: Option<u16>,

    hold: bool,
    grab: bool,

    cooldown: Duration,
    cooldown_pr: Duration,
}

impl StateNormal {
    pub fn run(self, shared: Shared) {
        let (transmitter, receiver) = mpsc::channel::<AutoclickerState>();

        let mut events: [input_event; 1] = unsafe { std::mem::zeroed() };
        let input = shared.input;
        let output = shared.output.clone();

        let left_bind = self.left_bind;
        let middle_bind = self.middle_bind;
        let right_bind = self.right_bind;

        let debug = shared.debug;
        let grab = self.grab;

        let mut state = AutoclickerState::default();
        let hold = self.hold;

        state.lock = self.lock_unlock_bind.is_some();
        _ = transmitter.send(state);

        thread::spawn(move || {
            loop {
                let len = match input.read(&mut events) {
                    Ok(len) => len,
                    Err(err) => {
                        eprintln!("\x1B[1;31mCaptured device error: {err}\x1B[22;39m");
                        eprintln!("\x1B[1;33mThe Clicker will terminate!\x1B[22;39m");
                        eprintln!();
                        std::process::exit(1);
                    }
                };

                for event in &events[0..len] {
                    if debug {
                        println!("Event: {:?}", event);
                    }

                    let mut used = false;
                    let old_state = state;

                    let pressed = matches!(event.value, 1 | 2);

                    if !state.lock {
                        for (bind, state) in [
                            (left_bind, &mut state.left),
                            (right_bind, &mut state.right),
                            (middle_bind, &mut state.middle),
                        ] {
                            if let Some(bind) = bind
                                && event.code == bind
                            {
                                if hold {
                                    if pressed != *state {
                                        *state = pressed;
                                    }
                                } else if pressed {
                                    *state = !*state;
                                }
                                used = true;
                            }
                        }
                    }

                    if let Some(bind) = self.lock_unlock_bind
                        && event.code == bind
                        && pressed
                    {
                        state.lock = !state.lock;
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
            }
        });

        autoclicker(
            shared.beep,
            receiver,
            &shared.output,
            self.cooldown,
            self.cooldown_pr,
        );
    }
}

pub struct StateLegacy {
    cooldown: Duration,
    cooldown_pr: Duration,
}

impl StateLegacy {
    fn run(self, shared: Shared) {
        let (transmitter, receiver) = mpsc::channel::<AutoclickerState>();

        let input = shared.input;

        let fd = input.handler.as_inner().as_raw_fd();
        let mut data: [u8; 3] = [0; 3];
        let mut state = AutoclickerState {
            lock: true,
            ..Default::default()
        };
        transmitter.send(state).unwrap();

        let mut old_left = 0;
        let mut old_right = 0;
        let mut old_middle = 0;

        std::thread::spawn(move || {
            let fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(fd) };
            loop {
                let Ok(len) = nix::unistd::read(fd, &mut data) else {
                    panic!("Cannot read from input device!");
                };

                if len != 3 {
                    continue;
                }

                let left = data[0] & 1;
                let right = (data[0] >> 1) & 1;
                let middle = (data[0] >> 2) & 1;

                let old_state = state;

                if !state.lock {
                    for (value, old_value, state) in [
                        (left, old_left, &mut state.left),
                        (right, old_right, &mut state.right),
                    ] {
                        if value == 1 && old_value == 0 {
                            *state = !*state;
                        }
                    }
                }

                if middle == 1 && old_middle == 0 {
                    state.lock = !state.lock;
                }

                old_left = left;
                old_right = right;
                old_middle = middle;

                if old_state != state {
                    transmitter.send(state).unwrap();
                }
            }
        });

        autoclicker(
            shared.beep,
            receiver,
            &shared.output,
            self.cooldown,
            self.cooldown_pr,
        );
    }
}

fn autoclicker(
    beep: bool,
    receiver: std::sync::mpsc::Receiver<AutoclickerState>,
    output: &OutputDevice,
    cooldown: Duration,
    cooldown_pr: Duration,
) {
    let mut toggle = AutoclickerState::default();
    println!();
    print_active(&toggle);

    loop {
        if let Some(recv) = if toggle.left | toggle.middle | toggle.right {
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
        if toggle.middle {
            output.send_key(Key::ButtonMiddle, KeyState::PRESSED);
        }
        if toggle.right {
            output.send_key(Key::ButtonRight, KeyState::PRESSED);
        }

        if !cooldown_pr.is_zero() {
            thread::sleep(cooldown_pr);
        }

        if toggle.left {
            output.send_key(Key::ButtonLeft, KeyState::RELEASED);
        }
        if toggle.middle {
            output.send_key(Key::ButtonMiddle, KeyState::RELEASED);
        }
        if toggle.right {
            output.send_key(Key::ButtonRight, KeyState::RELEASED);
        }
        thread::sleep(cooldown);
    }
}

pub enum Variant {
    Normal(StateNormal),
    Legacy(StateLegacy),
}

impl Variant {
    pub fn run(self, shared: Shared) {
        match self {
            Variant::Normal(state_normal) => state_normal.run(shared),
            Variant::Legacy(state_legacy) => state_legacy.run(shared),
        }
    }
}

pub struct Shared {
    debug: bool,
    beep: bool,
    input: InputDevice,
    output: Arc<OutputDevice>,
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
        let output = OutputDevice::uinput_open(PathBuf::from("/dev/uinput"), "TheClicker").unwrap();

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
                middle_bind,
                right_bind,
                lock_unlock_bind,
                hold,
                grab,
                cooldown,
                cooldown_press_release,
            } => {
                output.add_mouse_attributes(false);
                print!("run -d{device_query:?} -c{cooldown} -C{cooldown_press_release}");

                if let Some(bind) = left_bind {
                    print!(" -l{bind}")
                }
                if let Some(bind) = middle_bind {
                    print!(" -m{bind}")
                }
                if let Some(bind) = right_bind {
                    print!(" -r{bind}")
                }
                if let Some(bind) = lock_unlock_bind {
                    print!(" -T{bind}")
                }
                if hold {
                    print!(" -H")
                }
                if grab {
                    print!(" --grab")
                }
                println!("`");

                let input = input_device_from_query(device_query);
                if input.filename.starts_with("mouse") && input.filename.as_str() == "mice" {
                    eprintln!("Use the run-legacy for legacy devices");
                    std::process::exit(4);
                }

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
                        middle_bind,
                        right_bind,
                        lock_unlock_bind,
                        hold,
                        grab,
                        cooldown: Duration::from_millis(cooldown),
                        cooldown_pr: Duration::from_millis(cooldown_press_release),
                    }),
                }
            }
            args::Command::RunLegacy {
                device_query,
                cooldown,
                cooldown_press_release,
            } => {
                output.add_mouse_attributes(true);
                println!("run-legacy -d{device_query:?} -c{cooldown} -C{cooldown_press_release}`");

                let input = input_device_from_query(device_query);
                if input.filename.as_str() == "mice" {
                    eprintln!(
                        "You cannot use the /dev/input/mice, because receivers events from all other /dev/input/mouse{{N}}"
                    );
                    std::process::exit(5);
                }

                output.create();

                Self {
                    shared: Shared {
                        debug,
                        beep,
                        input,
                        output: Arc::new(output),
                    },
                    variant: Variant::Legacy(StateLegacy {
                        cooldown: Duration::from_millis(cooldown),
                        cooldown_pr: Duration::from_millis(cooldown_press_release),
                    }),
                }
            }
        }
    }

    pub fn main_loop(self) {
        self.variant.run(self.shared);
    }
}

fn input_device_from_query(device_query: String) -> InputDevice {
    'try_set_input: {
        if device_query.is_empty() {
            eprintln!("Device query is empty!");
            std::process::exit(1);
        }

        if device_query.starts_with('/') {
            let Ok(device) = InputDevice::dev_open(PathBuf::from(&device_query)) else {
                eprintln!("Cannot open device: {device_query}");
                std::process::exit(2);
            };
            break 'try_set_input device;
        } else {
            let Some(device) = InputDevice::find_device(&device_query) else {
                eprintln!("Cannot find device: {device_query}");

                std::process::exit(3);
            };
            break 'try_set_input device;
        }
    }
}

fn print_active(toggle: &AutoclickerState) {
    let is_terminal = stdout().is_terminal();

    if is_terminal {
        print!("\x1b[0K");
    }

    print!("Active: ");
    if toggle.lock {
        print!("LOCKED: ")
    }
    if toggle.left {
        print!("left ")
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
    let input_device = InputDevice::select_device();

    println!("Device name: {}", input_device.name);

    let legacy = input_device.filename.starts_with("mouse");

    if legacy {
        eprintln!("\x1B[1;31mUsing legacy interface for PS/2 device\x1B[0;39m");
        let cooldown = choose_usize("Choose cooldown, the min is 25", Some(25)) as u64;
        let cooldown_press_release =
            choose_usize("Choose cooldown between press and release", Some(0)) as u64;

        args::Command::RunLegacy {
            device_query: input_device.path.to_str().unwrap().to_owned(),
            cooldown,
            cooldown_press_release,
        }
    } else {
        let lock_unlock_bind = choose_yes(
            "Lock Unlock mode, useful for mouse without side buttons",
            false,
        )
        .then(|| choose_key(&input_device, "lock_unlock_bind"));

        let left_bind = choose_yes("You want a binding for left autoclicker?", true)
            .then(|| choose_key(&input_device, "left_bind"));
        let middle_bind = choose_yes("You want a binding for middle autoclicker?", false)
            .then(|| choose_key(&input_device, "middle_bind"));
        let right_bind = choose_yes("You want a binding for right autoclicker?", true)
            .then(|| choose_key(&input_device, "right_bind"));
        let hold = choose_yes("You want to hold the bind / active hold_mode?", true);
        println!(
            "\x1B[1;33mWarning: if you enable grab mode you can get softlocked\x1B[0;39m, if the compositor will not use TheClicker device."
        );
        println!(
            "If the device input is grabbed, the input device will be emulated by TheClicker, and when you press a binding that will not be sent"
        );
        let grab = choose_yes("You want to grab the input device?", true);
        println!("Grab: {grab}");
        let mut cooldown = choose_usize("Choose cooldown, the min is 25", Some(25)) as u64;
        if cooldown < 25 {
            cooldown = 25;
            println!("\x1B[1;39mThe cooldown was set to \x1B[1;32m25\x1B[0;39m");
            println!(
                "\x1B[1;33mThe linux kernel does not permit more the 40 events from a device per second!\x1B[0;39m"
            );
            println!(
                "\x1B[;32mIf your kernel permits that, you can bypass this dialog using the command args and modify the -c argument.\x1B[;39m"
            );
        }
        let cooldown_press_release =
            choose_usize("Choose cooldown between press and release", Some(0)) as u64;

        std::thread::sleep(WAIT_KEY_RELEASE);

        args::Command::Run {
            left_bind,
            right_bind,
            middle_bind,
            hold,
            grab,
            lock_unlock_bind,
            cooldown,
            cooldown_press_release,
            device_query: input_device.path.to_str().unwrap().to_owned(),
        }
    }
}

fn choose_key(input_device: &InputDevice, name: &str) -> u16 {
    let mut events: [input_linux::sys::input_event; 1] = unsafe { std::mem::zeroed() };
    std::thread::sleep(WAIT_KEY_RELEASE);
    println!("\x1B[1;33mWaiting for key presses from the selected device\x1B[22;39m");
    _ = input_device.grab(true);
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
        _ = input_device.grab(false);

        println!("\t{}", KeyCode(events[0].code));

        if matches!(
            events[0].code as i32,
            input_linux::sys::KEY_LEFTCTRL | input_linux::sys::KEY_C
        ) {
            println!("\x1B[1;31mThis key is blacklisted\x1B[22;39m");
            std::process::exit(10);
        }

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

fn choose_usize(message: impl std::fmt::Display, default: Option<usize>) -> usize {
    loop {
        print!(
            "\x1B[1;39m{message} {} \x1B[1;32m",
            if let Some(default) = default {
                format!("[\x1B[1;32m{default}\x1B[0;39m]\x1B[0;39m:")
            } else {
                "->".to_owned()
            }
        );
        _ = std::io::stdout().flush();
        let response = std::io::stdin()
            .lines()
            .next()
            .expect("Cannot read from stdin")
            .expect("Cannot read from stdin");
        print!("\x1B[0;39m");
        _ = std::io::stdout().flush();

        if let Some(default) = default
            && response.is_empty()
        {
            return default;
        }

        let Ok(num) = response.parse::<usize>() else {
            println!("{response:?} Is not a number!");
            continue;
        };

        return num;
    }
}
