# Simple CLI autoclicker for linux
[![Crates.io](https://img.shields.io/crates/v/theclicker.svg)](https://crates.io/crates/theclicker)

Works on both xorg and wayland by utilising uinput and evdev!

## Running
![Running](images/running.png)

## Install
```cargo install theclicker```

## To run TheClicker

Run `theclicker`

Select your input device.

If it is not a legacy device, for example: `/dev/input/mouse{N}` or `/dev/input/mice`, choose the binding for left and right by pressing then confirming.
I recommend hold and grab mode.

Grab mode has been tested on KDE Plasma Wayland `6.2.5` and `6.5.4`

You can use `--help` to see more information!

## Build

If you don't have Rust installed you can install rust from [rustup](https://rustup.rs/)

You need to have the stable toolchain installed!

Then run: `cargo build --release`

The binary path will be: `./target/release/theclicker`

# Problems?

## Crashes

Add your user to the input group, (may not work on all systems): `sudo usermod -aG input $USER`

Or try running as root: ```sudo theclicker```

If it returns: `sudo: theclicker: command not found`, then edit your `/etc/sudoers` file with: `sudo visudo`

And edit this line:

`Defaults        secure_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"`

Add the path of TheClicker to the end like this (replace `youruser`):

`Defaults        secure_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/home/youruser/.cargo/bin/"`
