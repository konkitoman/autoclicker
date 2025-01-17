use std::{
    fs::{self, File},
    io::{self, Write},
    os::fd::AsRawFd,
    path::PathBuf,
    process::exit,
    time::SystemTime,
};

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
    pub filename: String,
    handler: UInputOrDev,
}

impl Device {
    pub fn dev_open(mut path: PathBuf) -> Result<Self, String> {
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

        let name_bytes = handler.device_name().unwrap_or(vec![]);
        let name = String::from_utf8_lossy(&name_bytes);

        let name = format!("{name}-{}", path.file_name().unwrap().to_str().unwrap());

        Ok(Self {
            filename: path.file_name().unwrap().to_str().unwrap().to_owned(),
            path,
            handler: UInputOrDev::DevInput(handler),
            name,
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
            filename: name.to_string(),
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
    pub fn copy_attributes(&self, debug: bool, from: &Device) {
        match (&self.handler, &from.handler) {
            (UInputOrDev::Uinput(to), UInputOrDev::DevInput(from)) => {
                if let Ok(bits) = from.event_bits() {
                    if debug {
                        println!("Copy event_bits: {bits:?}")
                    }
                    for bit in bits.iter() {
                        to.set_evbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.relative_bits() {
                    if debug {
                        println!("Copy releative_bits: {bits:?}")
                    }
                    for bit in bits.iter() {
                        to.set_relbit(bit).unwrap();
                    }
                }

                // FIX: - TheClicker: kernel bug: device has min == max on ABS_VOLUME
                // if let Ok(bits) = from.absolute_bits() {
                //     if debug {
                //         println!("Copy absolute_bits: {bits:?}")
                //     }
                //     for bit in bits.iter() {
                //         to.set_absbit(bit).unwrap();
                //     }
                // }

                if let Ok(bits) = from.misc_bits() {
                    if debug {
                        println!("Copy misc_bits: {bits:?}")
                    }
                    for bit in bits.iter() {
                        to.set_mscbit(bit).unwrap();
                    }
                }

                if let Ok(bits) = from.key_bits() {
                    if debug {
                        println!("Copy key_bits: {bits:?}")
                    }
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
                        input_linux::sys::FF_MAX_EFFECTS as u32,
                        &[],
                    )
                    .unwrap();
            }
            UInputOrDev::DevInput(_) => todo!(),
        }
    }

    pub fn empty_read_buffer(&self) {
        let fd = match &self.handler {
            UInputOrDev::DevInput(dev_input) => dev_input.as_inner().as_raw_fd(),
            _ => unreachable!(),
        };
        let mut pollfd = nix::libc::pollfd {
            fd,
            events: nix::libc::POLLIN,
            revents: 0,
        };
        let mut events: [input_event; 1] = unsafe { std::mem::zeroed() };
        loop {
            _ = unsafe { nix::libc::poll(&mut pollfd, 1, 0) };
            if pollfd.revents & nix::libc::POLLIN != nix::libc::POLLIN {
                break;
            }
            _ = self.read(&mut events);
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
        fs::read_dir("/dev/input")
            .unwrap()
            .filter_map(|res| res.ok())
            .filter_map(|entry| {
                if let Ok(ty) = entry.file_type() {
                    if ty.is_dir() {
                        return None;
                    }
                }
                Device::dev_open(entry.path()).ok()
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

            devices.retain(|device| {
                let UInputOrDev::DevInput(handler) = &device.handler else {
                    unreachable!()
                };

                let Ok(event_bits) = handler.event_bits() else {
                    return true;
                };

                event_bits.get(EventKind::Key)
            });

            println!("Select input device: ");
            for device in devices.iter().enumerate() {
                println!("\t{}: Device: {}", device.0, device.1.name);
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
