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

Select your mouse or keyboard.
Default binds are for mouse!
On mouse back or forword to activate left or right clicker!

You can use --find-keycodes to find what keycode you are pressing!
You can use --left-bind or --right-bind to set on what keycode the clicker will activate!

You can use --help to see more!

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