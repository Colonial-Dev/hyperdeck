#![no_std]
#![no_main]

mod display;
mod keypad;
mod usb;

use display_interface_spi::SPIInterface;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::{MODE_0, MODE_3};
use fugit::RateExtU32;

use rp_pico::hal::gpio::FunctionSpi as SPI;
use rp_pico::hal::timer::{Instant, Timer};
use rp_pico::hal::{self, pac, Clock, Spi, I2C};
use rp_pico::hal::usb::UsbBus;

use usb_device::class_prelude::UsbBusAllocator;

use keypad::Keypad;

static mut TIMER: Option<Timer> = None;

type Duration64 = fugit::Duration<u64, 1, 100000>;

/// Custom panic handler. Resets the Pico into BOOTSEL (flashing) mode.
/// Useful for distinguishing between a hang/deadlock and panic/crash.
#[inline(never)]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    rp2040_hal::rom_data::reset_to_usb_boot(0, 0);

    loop {
        // The previous line hard resets the controller, so this is unreachable.
    }
}

#[rp_pico::entry]
fn main() -> ! {
    let mut keypad = hardware_init();
    keypad.set_brightness(0.1);
    loop {
        for (id, event) in keypad.update() {
            if id == 0 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(255, 0, 0);
                }
            }
            if id == 1 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(0, 255, 0);
                }
            }
            if id == 2 && matches!(event, keypad::KeyEvent::Held) {
                for i in 0..16 {
                    keypad.keys[i].pressed_color = keypad::Color::new(0, 0, 255);
                }
            }
            if id == 3 && matches!(event, keypad::KeyEvent::Held) {
                rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            }
            if id == 15 && matches!(event, keypad::KeyEvent::Pressed) {
                let result = usb::push_keyboard(usbd_hid::descriptor::KeyboardReport {
                    modifier: 0b00000101,
                    reserved: 0,
                    leds: 0,
                    keycodes: [0x17, 0x0, 0x0, 0x0, 0x0, 0x0]
                }); 

                if matches!(result, Err(usb_device::UsbError::WouldBlock)) {
                    keypad.keys[0].default_color = keypad::Color::new(0, 255, 0);
                }
            }

            if matches!(event, keypad::KeyEvent::Released) {
                let _ = usb::push_keyboard(usbd_hid::descriptor::KeyboardReport {
                    modifier: 0,
                    reserved: 0,
                    leds: 0,
                    keycodes: [0x0, 0x0, 0x0, 0x0, 0x0, 0x0]
                }); 
            }
        }
    }
}

fn hardware_init() -> Keypad {
    let mut pac = pac::Peripherals::take().unwrap();

    let sio = hal::Sio::new(pac.SIO);

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

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    // Safety: we're still in the initialization stage,
    // so there's no race risk.
    unsafe {
        TIMER = Some(timer);
    }

    let bus_allocator = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    
    usb::usb_init(bus_allocator);

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

    // Display
    let mut bl = pins.gpio22.into_push_pull_output();
    let mut rst = pins.gpio28.into_push_pull_output();
    
    let dc = pins.gpio16.into_push_pull_output();
    let cs = pins.gpio21.into_push_pull_output();

    let _ = pins.gpio26.into_mode::<SPI>();
    let _ = pins.gpio27.into_mode::<SPI>();

    let core = pac::CorePeripherals::take().unwrap();
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    bl.set_low().unwrap();
    rst.set_high().unwrap();

    let spi = Spi::<_, _, 8>::new(pac.SPI1).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        64_000_000.Hz(),
        &MODE_3,
    );

    let interface = SPIInterface::new(spi, dc, cs);

    let mut display = mipidsi::Builder::st7789_pico1(interface)
        .with_display_size(135, 240)
        .with_orientation(mipidsi::Orientation::Landscape(true))
        .init(&mut delay, Some(rst))
        .unwrap();

    display.clear(embedded_graphics::pixelcolor::Rgb565::BLACK).unwrap();

    const Y_MAX: i32 = 134;
    const X_MAX: i32 = 239;

    const CIRCLE_WIDTH: u32 = 60;
    const CIRCLE_RADIUS: i32 = 30;

    use embedded_graphics::{
        geometry::AnchorPoint,
        mono_font::{ascii::FONT_9X15, MonoTextStyleBuilder},
        pixelcolor::Rgb565,
        prelude::{DrawTarget, RgbColor},
        text::{Alignment, Baseline, Text, TextStyleBuilder},
    };

    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::*;

    let circle_origin_red = Circle::new(
        Point::new(0 - CIRCLE_RADIUS, 0 - CIRCLE_RADIUS),
        CIRCLE_WIDTH,
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED));

    let circle_xmax_blue = Circle::new(
        Point::new(X_MAX - CIRCLE_RADIUS, 0 - CIRCLE_RADIUS),
        CIRCLE_WIDTH,
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE));

    let circle_ymax_green = Circle::new(
        Point::new(0 - CIRCLE_RADIUS, Y_MAX - CIRCLE_RADIUS),
        CIRCLE_WIDTH,
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN));

    let circle_xmax_ymax_yellow = Circle::new(
        Point::new(X_MAX - CIRCLE_RADIUS, Y_MAX - CIRCLE_RADIUS),
        CIRCLE_WIDTH,
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW));

    circle_origin_red.draw(&mut display).unwrap();
    circle_xmax_blue.draw(&mut display).unwrap();
    circle_ymax_green.draw(&mut display).unwrap();
    circle_xmax_ymax_yellow.draw(&mut display).unwrap();

    let center_aligned = TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Middle)
        .build();

    let bb = display.bounding_box().offset(-20);

    let text_style = MonoTextStyleBuilder::new()
    .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
    .text_color(Rgb565::WHITE)
    .build();

    Text::with_text_style("HEXADECK", bb.anchor_point(AnchorPoint::TopCenter), text_style, center_aligned)
        .draw(&mut display)
        .unwrap();

    Text::with_text_style("HEXADECK", bb.anchor_point(AnchorPoint::Center), text_style, center_aligned)
        .draw(&mut display)
        .unwrap();

    Text::with_text_style("HEXADECK", bb.anchor_point(AnchorPoint::BottomCenter), text_style, center_aligned)
        .draw(&mut display)
        .unwrap();

    keypad
}

/// Get an Instant representing "now."
fn now() -> Instant {
    // Safety: get_counter merely reads from the timer -
    // nothing is mutated.
    unsafe {
        TIMER.as_ref().unwrap().get_counter()
    }
}
