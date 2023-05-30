use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::exit,
};

use input_linux::{EvdevHandle, EventKind, InputId, Key, UInputHandle};
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
    pub fn add_mouse_atributes(&self) {
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

    pub fn select_device() -> Device {
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
            if num > devices.len() {
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
                let buffer = base64::encode(devices[num].path.to_str().unwrap());
                let buffer = buffer.as_bytes();
                cache_file.write_all(buffer).unwrap();
                return device;
            }
        }
    }
}
