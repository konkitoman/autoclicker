use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::exit,
    time::SystemTime,
};

use clap::Parser;
use input_linux::{
    sys::{input_event, BUS_USB},
    EvdevHandle, EventKind, EventTime, InputEvent, InputId, Key, KeyEvent, KeyState,
    SynchronizeEvent, UInputHandle,
};

const VENDOR: u16 = 0x3232;
const VERSION: u16 = 0x1234;
const PRODUCT: u16 = 0x5678;

pub enum UInputOrDev {
    Uinput(UInputHandle<File>),
    DevInput(EvdevHandle<File>),
}

pub enum DeviceType {
    Mouse,
    Keyboard,
}

impl DeviceType {
    pub fn is_mouse(&self) -> bool {
        matches!(self, DeviceType::Mouse)
    }

    pub fn is_keyboard(&self) -> bool {
        matches!(self, DeviceType::Keyboard)
    }
}

pub struct Device {
    pub name: String,
    pub path: PathBuf,
    handler: UInputOrDev,
    pub ty: DeviceType,
}

impl Device {
    pub fn dev_open(mut path: PathBuf, ty: DeviceType) -> Result<Self, String> {
        if path.is_symlink() {
            // This means that the path is /dev/input/by-path/{ } or /dev/input/by-id/{ }
            path = PathBuf::from("/dev/input")
                .join(std::fs::read_link(&path).unwrap().file_name().unwrap());
        }

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
        let name = std::ffi::CStr::from_bytes_until_nul(&name_bytes)
            .expect("Invalid Device Name")
            .to_str()
            .expect("Invalid String");

        Ok(Self {
            path,
            handler: UInputOrDev::DevInput(handler),
            name: name.to_string(),
            ty,
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
            ty: DeviceType::Mouse,
        })
    }

    pub fn add_mouse_attributes(&self) {
        match &self.handler {
            UInputOrDev::Uinput(device) => {
                device.set_evbit(EventKind::Key).unwrap();
                device.set_evbit(EventKind::Synchronize).unwrap();

                device.set_keybit(Key::ButtonLeft).unwrap();
                device.set_keybit(Key::ButtonRight).unwrap();
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

    pub fn devices() -> Vec<Device> {
        fs::read_dir("/dev/input/by-id")
            .unwrap()
            .filter_map(|res| res.ok())
            .filter_map(|entry| {
                Device::dev_open(entry.path(), {
                    let file_name = entry.file_name().into_string().unwrap();
                    if file_name.ends_with("event-mouse") {
                        DeviceType::Mouse
                    } else if file_name.ends_with("event-kbd") {
                        DeviceType::Keyboard
                    } else {
                        return None;
                    }
                })
                .ok()
            })
            .collect::<Vec<Device>>()
    }

    pub fn find_device(device_name: &str) -> Option<Device> {
        let devices = Self::devices();

        for device in devices {
            if device.name.trim() == device_name {
                return Some(device);
            }
        }

        let devices = Self::devices();
        devices
            .into_iter()
            .find(|device| device.name.trim().contains(device_name))
    }

    pub fn select_device() -> Device {
        loop {
            let mut devices = Self::devices();

            println!("Select input device: ");

            println!(" Mouses: ");
            for device in devices.iter().enumerate().filter(|(_, d)| d.ty.is_mouse()) {
                println!("  {}: Device: {}", device.0, device.1.name);
            }

            println!(" Keyboards: ");
            for device in devices
                .iter()
                .enumerate()
                .filter(|(_, d)| d.ty.is_keyboard())
            {
                println!("  {}: Device: {}", device.0, device.1.name);
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
            if user_input.trim().to_lowercase() == "y" || user_input.trim().is_empty() {
                let device = devices.remove(num);

                let mut cache_file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open("/tmp/TheClicker")
                    .unwrap();
                let mut args = crate::Args::parse();
                args.use_device = Some(device.name.clone());
                args.clear_cache = false;

                cache_file
                    .write_all(ron::to_string(&args).unwrap().as_bytes())
                    .unwrap();

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

    EventTime::new(time.as_secs() as i64, time.subsec_micros() as i64)
}
