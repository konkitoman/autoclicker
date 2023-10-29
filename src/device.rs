use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::exit,
    time::SystemTime,
};

use base64::prelude::*;
use input_linux::{
    EvdevHandle, EventKind, EventTime, InputEvent, InputId, Key, KeyEvent, KeyState,
    SynchronizeEvent, UInputHandle,
};
use input_linux_sys::{input_event, BUS_USB};

const VENDOR: u16 = 0x3232;
const VERSION: u16 = 0x1234;
const PRODUCT: u16 = 0x5678;

pub enum UInputOrDev {
    Uinput(UInputHandle<File>),
    DevInput(EvdevHandle<File>),
}

pub struct Device {
    pub name: String,
    pub path: PathBuf,
    handler: UInputOrDev,
}

impl Device {
    pub fn dev_open(path: PathBuf) -> Result<Self, String> {
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(err) => {
                println!("Error: {}", err);
                println!("Invalid device OR Not having access to the file, try as root!");
                exit(1);
            }
        };
        let handler = EvdevHandle::new(file);

        if !handler.event_bits().unwrap().get(EventKind::Key) {
            return Err(String::from("Unavailable device!"));
        }

        let name_bytes = handler.device_name().unwrap();
        let name = std::str::from_utf8(&name_bytes).unwrap();

        Ok(Self {
            path,
            handler: UInputOrDev::DevInput(handler),
            name: name.to_string(),
        })
    }

    pub fn uinput_open(path: PathBuf, name: &str) -> Result<Self, String> {
        let file = match fs::OpenOptions::new().write(true).open(&path) {
            Ok(file) => file,
            Err(err) => {
                println!("Error: {}", err);
                println!("Not having access to create device, try as root!");
                exit(1);
            }
        };

        let handler = UInputHandle::new(file);

        Ok(Self {
            path,
            handler: UInputOrDev::Uinput(handler),
            name: name.to_string(),
        })
    }

    pub fn add_mouse_attributes(&self) {
        match &self.handler {
            UInputOrDev::Uinput(mouse) => {
                mouse.set_evbit(EventKind::Key).unwrap();
                mouse.set_evbit(EventKind::Synchronize).unwrap();

                mouse.set_keybit(Key::ButtonLeft).unwrap();
                mouse.set_keybit(Key::ButtonRight).unwrap();
            }
            UInputOrDev::DevInput(_) => {
                todo!()
            }
        }
    }

    /// Only copis attributes from DevInput to UInput
    pub fn copy_attributes(&self, from: &Device) {
        match (&self.handler, &from.handler) {
            (UInputOrDev::Uinput(to), UInputOrDev::DevInput(from)) => {
                if let Ok(bits) = from.event_bits() {
                    for bit in bits.iter() {
                        to.set_evbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.relative_bits() {
                    for bit in bits.iter() {
                        to.set_relbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.absolute_bits() {
                    for bit in bits.iter() {
                        to.set_absbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.misc_bits() {
                    for bit in bits.iter() {
                        to.set_mscbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.key_bits() {
                    for bit in bits.iter() {
                        to.set_keybit(bit).unwrap();
                    }
                }
            }
            _ => {
                todo!()
            }
        }
    }

    pub fn create(&self) {
        match &self.handler {
            UInputOrDev::Uinput(device) => {
                device
                    .create(
                        &InputId {
                            bustype: BUS_USB,
                            vendor: VENDOR,
                            product: PRODUCT,
                            version: VERSION,
                        },
                        self.name.as_bytes(),
                        0,
                        &[],
                    )
                    .unwrap();
            }
            UInputOrDev::DevInput(_) => todo!(),
        }
    }
    pub fn read(&self, events: &mut [input_event]) -> io::Result<usize> {
        match &self.handler {
            UInputOrDev::Uinput(device) => device.read(events),
            UInputOrDev::DevInput(device) => device.read(events),
        }
    }

    pub fn write(&self, events: &[input_event]) -> io::Result<usize> {
        match &self.handler {
            UInputOrDev::Uinput(device) => device.write(events),
            UInputOrDev::DevInput(_) => todo!(),
        }
    }

    pub fn grab(&self, grab: bool) -> io::Result<()> {
        match &self.handler {
            UInputOrDev::Uinput(_) => todo!(),
            UInputOrDev::DevInput(device) => device.grab(grab),
        }
    }

    pub fn select_device(use_dev: String) -> Device {
        loop {
            let devices = fs::read_dir("/dev/input")
                .unwrap()
                .filter_map(|res| res.ok())
                .filter(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .map(|s| s.contains("event"))
                        .unwrap_or(false)
                })
                .filter_map(|entry| Device::dev_open(entry.path()).ok())
                .collect::<Vec<Device>>();

            if !use_dev.is_empty() {
                let mut num: usize = 0;
                let mut found: bool = false;
                for device in devices.iter().enumerate() {
                    match device.1.name.trim_matches(char::from(0)).eq(&use_dev) {
                        true => {
                            num = device.0;
                            found = true;
                            println!("Using {}.", device.1.name);
                            break;
                        },
                        false => {
                            continue;
                        }
                    }
                }
                if !found {
                    for device in devices.iter().enumerate() {
                        match device.1.name.trim_matches(char::from(0)).contains(&use_dev) {
                            true => {
                                num = device.0;
                                found = true;
                                println!("Using {}.", device.1.name);
                                break;
                            },
                            false => {
                                continue;
                            }
                        }
                    }
                }

                if !found {
                    println!("Unable to find device: {}", use_dev);
                    println!("Terminating");
                    std::process::exit(1);
                }

                let device = Device::dev_open(devices[num].path.clone()).unwrap();
                return device;
            }

            println!("Select input device: ");
            for device in devices.iter().enumerate() {
                println!("{}: Device: {}", device.0, device.1.name);
            }
            let mut user_input = String::new();
            print!("-> ");
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut user_input).unwrap();
            let num: usize = match user_input.trim().parse() {
                Ok(num) => num,
                Err(_) => {
                    println!("Is not an number!");
                    continue;
                }
            };

            if num >= devices.len() {
                println!("Is to large!");
                continue;
            }

            println!("Device selected: {}, Is Ok [Y/n]", devices[num].name);
            print!("-> ");
            std::io::stdout().flush().unwrap();
            user_input = String::new();
            std::io::stdin().read_line(&mut user_input).unwrap();
            if user_input.trim() == "Y" || user_input.trim() == "y" {
                let device = Device::dev_open(devices[num].path.clone()).unwrap();
                let mut cache_file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open("/tmp/TheClicker")
                    .unwrap();
                let buffer = BASE64_STANDARD.encode(devices[num].path.to_str().unwrap());
                let buffer = buffer.as_bytes();
                cache_file.write_all(buffer).unwrap();
                return device;
            }
        }
    }

    pub fn send_key(&self, key: Key, state: KeyState) {
        let events: [input_event; 2] = [
            InputEvent::from(KeyEvent::new(get_current_time(), key, state))
                .as_raw()
                .to_owned(),
            InputEvent::from(SynchronizeEvent::report(get_current_time()))
                .as_raw()
                .to_owned(),
        ];
        self.write(&events)
            .expect("Cannot send key event: {events:?}");
    }
}

pub fn get_current_time() -> EventTime {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    EventTime::new(time.as_secs() as i64, time.as_micros() as i64)
}
