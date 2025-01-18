use std::{
    fs::{self, File},
    io,
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

use crate::{choose_usize, choose_yes};

const VENDOR: u16 = 0x3232;
const VERSION: u16 = 0x1234;
const PRODUCT: u16 = 0x5678;

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

pub struct InputDevice {
    pub name: String,
    pub path: PathBuf,
    pub filename: String,
    pub handler: EvdevHandle<File>,
}

impl InputDevice {
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
            handler,
            name,
        })
    }

    pub fn devices() -> Vec<InputDevice> {
        fs::read_dir("/dev/input")
            .unwrap()
            .filter_map(|res| res.ok())
            .filter_map(|entry| {
                if let Ok(ty) = entry.file_type() {
                    if ty.is_dir() {
                        return None;
                    }
                }

                if entry.path().file_name().unwrap().to_str().unwrap() == "mice" {
                    return None;
                }

                InputDevice::dev_open(entry.path()).ok()
            })
            .collect::<Vec<InputDevice>>()
    }

    pub fn find_device(device_name: &str) -> Option<InputDevice> {
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

    pub fn select_device() -> InputDevice {
        loop {
            let mut devices = Self::devices();

            devices.retain(|device| {
                let Ok(event_bits) = device.handler.event_bits() else {
                    return true;
                };

                event_bits.get(EventKind::Key)
            });

            println!("Select input device: ");
            for device in devices.iter().enumerate() {
                println!("\t{}: Device: {}", device.0, device.1.name);
            }

            let num = choose_usize("", None);

            if num >= devices.len() {
                println!("Is to large!");
                continue;
            }

            if choose_yes(
                format!("Device selected: {}, Is Ok", devices[num].name),
                true,
            ) {
                let device = devices.remove(num);

                return device;
            }
        }
    }

    pub fn read(&self, events: &mut [input_event]) -> io::Result<usize> {
        self.handler.read(events)
    }

    pub fn grab(&self, grab: bool) -> io::Result<()> {
        self.handler.grab(grab)
    }

    pub fn empty_read_buffer(&self) {
        let fd = self.handler.as_inner().as_raw_fd();
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
}

pub struct OutputDevice {
    pub name: String,
    pub path: PathBuf,
    pub filename: String,
    pub handler: UInputHandle<File>,
}

impl OutputDevice {
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
            handler,
            name: name.to_string(),
            filename: name.to_string(),
        })
    }

    pub fn add_mouse_attributes(&self) {
        self.handler.set_evbit(EventKind::Key).unwrap();
        self.handler.set_evbit(EventKind::Synchronize).unwrap();

        self.handler.set_keybit(Key::ButtonLeft).unwrap();
        self.handler.set_keybit(Key::ButtonRight).unwrap();
    }

    /// Only copis attributes from DevInput to UInput
    pub fn copy_attributes(&self, debug: bool, from: &InputDevice) {
        let to = &self.handler;
        let from = &from.handler;

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
                println!("Copy relative_bits: {bits:?}")
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

    pub fn create(&self) {
        self.handler
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

    pub fn write(&self, events: &[input_event]) -> io::Result<usize> {
        self.handler.write(events)
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
