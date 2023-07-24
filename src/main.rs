#![no_std]
#![no_main]

mod config;
mod display;
mod keypad;
mod usb;
mod utils;

use cortex_m::delay::Delay;
use embedded_hal::spi::{MODE_0, MODE_3};
use fugit::RateExtU32;
use hal::rosc::RingOscillator;
use rp2040_hal::gpio::FunctionSpi as SPI;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::pwm::Slices;
use rp2040_hal::timer::Timer;
use rp2040_hal::usb::UsbBus;
use rp2040_hal::{self as hal, pac, Clock, Spi, I2C};
use usb_device::class_prelude::UsbBusAllocator;

use crate::display::{Display, Command::*};
use crate::keypad::Keypad;

struct Hardware {
    display: Display,
    keypad: Keypad,
    delay: Delay
}

#[rp_pico::entry]
fn main() -> ! {
    let mut hw = hardware_init();

    hw.display.set_brightness(1.0);
    hw.display.send_command(Splash);
    hw.delay.delay_ms(2000);

    hw.keypad.set_brightness(0.1);

    loop {
        for (id, event) in hw.keypad.update() {
            if id == 0 && matches!(event, keypad::KeyEvent::Pressed) {
                hw.display.send_command(Splash);
            }
            if id == 0 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    hw.keypad.keys[i].pressed_color = keypad::Color::new(255, 0, 0);
                }
            }
            if id == 1 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    hw.keypad.keys[i].pressed_color = keypad::Color::new(0, 255, 0);
                }
            }
            if id == 2 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    hw.keypad.keys[i].pressed_color = keypad::Color::new(0, 0, 255);
                }
            }
            if id == 3 && matches!(event, keypad::KeyEvent::Held) {
                rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            }
            if id == 15 && matches!(event, keypad::KeyEvent::Pressed) {
                let _ = usb::push_keyboard(usbd_hid::descriptor::KeyboardReport {
                    modifier: 0b00000101,
                    reserved: 0,
                    leds: 0,
                    keycodes: [0x17, 0x0, 0x0, 0x0, 0x0, 0x0],
                });
            }

            if matches!(event, keypad::KeyEvent::Released) {
                let _ = usb::push_keyboard(usbd_hid::descriptor::KeyboardReport {
                    modifier: 0,
                    reserved: 0,
                    leds: 0,
                    keycodes: [0x0, 0x0, 0x0, 0x0, 0x0, 0x0],
                });
            }
        }
    }
}

fn hardware_init() -> Hardware {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut sio = hal::Sio::new(pac.SIO);

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // Safety: we're still in the initialization stage,
    // so there's no race risk.
    unsafe { 
        utils::TIMER = Timer::new(pac.TIMER, &mut pac.RESETS).into();
        utils::ROSC = RingOscillator::new(pac.ROSC).initialize().into();
    }

    let bus_allocator = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Intializes USB bus and HIDs
    usb::init(bus_allocator);

    // I2C for keypad keys
    let i2c = I2C::i2c0(
        pac.I2C0,
        pins.gpio4.into_mode(), // SDA
        pins.gpio5.into_mode(), // SCL
        400.kHz(),
        &mut pac.RESETS,
        125_000_000.Hz(),
    );

    // SPI for keypad LEDs
    let cs = pins.gpio17.into_push_pull_output();
    let _ = pins.gpio18.into_mode::<SPI>();
    let _ = pins.gpio19.into_mode::<SPI>();

    let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        4_000_000.Hz(),
        &MODE_0,
    );

    let keypad = Keypad::new(i2c, spi, cs);

    // Setup display SPI
    let _ = pins.gpio26.into_mode::<SPI>();
    let _ = pins.gpio27.into_mode::<SPI>();

    let spi = Spi::<_, _, 8>::new(pac.SPI1).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        64_000_000.Hz(),
        &MODE_3,
    );

    // Setup delay and multicore for display
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);

    // Setup PWM for display backlight control
    let pwm_slices = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm = pwm_slices.pwm3;

    pwm.default_config();
    pwm.enable();

    let mut channel_a = pwm.channel_a;
    let _ = channel_a.output_to(pins.gpio22);

    // Initialize the display
    // WARNING: this starts the RP2040's second core!
    let display = Display::new(
        pins.gpio16,
        pins.gpio21,
        pins.gpio28,
        channel_a,
        spi,
        &mut delay,
        &mut mc,
    );

    //usb::config_mode();

    Hardware {
        display,
        keypad,
        delay
    }
}


