# Simple cli autoclicker for linux

Working on xorg and wayland

## Build

You need to have rust installed

### On ArchLinux

```sudo pacman -S rustup```
```rustup toolchain install stable```

### On any unix os

```curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh```
```rustup toolchain install stable```

```cargo build --release```

## To run ./theclicker

select your mouse or keyboard,
default is mouse
on mouse back and foword to activate
for more things add argument --help

### If crash

Is posibile to not work on any distribution: ```sudo usermod -aG input $USER```

OR

```sudo ./theclicker```
