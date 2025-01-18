# Simple cli autoclicker for linux
[![Crates.io](https://img.shields.io/crates/v/theclicker.svg)](https://crates.io/crates/theclicker)

Working on xorg and wayland.

Is using uinput and evdev!

## Running
![Running](images/running.png)

## Install
```cargo install theclicker```

## To run TheClicker

Run `theclicker`

Select your input device.

Then if is not a legacy interface like: `/dev/input/mouse{N}` or `/dev/input/mice`
You choose the binding for left and right by pressing then confirming.
I recommend hold and grab mode.

Grab mode is only tested on KDE Plasma wayland 6.2.5

You can use `--help` to see more!

## Build

If you don't have Rust installed you can install rust from [rustup](https://rustup.rs/)

You need to have the stable toolchain installed!

Then run `cargo build --release`

The binary will be in `./target/release/theclicker`

# Problems?

## If crash

Is posibile to not work on any distribution: ```sudo usermod -aG input $USER```

OR

```sudo theclicker```

IF ```sudo theclicker``` RETURNS `sudo: theclicker: command not found`

You should edit you'r /etc/sudoers
if you can find

`Defaults        secure_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/snap/bin"`

You should commented like

`#Defaults        secure_path="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/snap/bin"`


And the same thing for

`Defaults        env_reset`
