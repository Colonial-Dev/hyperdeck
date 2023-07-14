# Hexadeck

Pure Rust firmware for my [Raspberry Pi Pico](https://www.raspberrypi.com/products/raspberry-pi-pico/)-powered macropad, built using Pimoroni's [RGB keypad base](https://shop.pimoroni.com/products/pico-rgb-keypad-base) and [display pack](https://shop.pimoroni.com/products/pico-display-pack).

## Building

You'll need the Rust toolchain for `thumbv6m-none-eabi` and the `elf2uf2-rs` tool:

```
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs --locked
```

After that, the standard `cargo` commands should work. If a Pico is connected, `cargo run` will automatically flash the executable.