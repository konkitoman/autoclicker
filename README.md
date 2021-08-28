# Simple cli autoclicker for linux
Working on xorg and wayland

## Build
nead rust for install
### on arch:
```sudo pacman -S rustup``` <br>
```rustup toolchain install stable``` <br>
### on any unix os:
```curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh``` <br>
```rustup toolchain install stable``` <br>

```cargo build --relase```

## To run ./theclicker
select your mouse or keyboard,
default is mouse
on mouse back and foword to activate
for more things add argument --help

### If crash nead to add to your user, input group
```sudo usermod -aG input $USER```