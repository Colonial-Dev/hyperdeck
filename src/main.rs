#![no_std]
#![no_main]

mod keypad;

use fugit::RateExtU32;
use rp_pico::entry;
use panic_halt as _;

use embedded_hal::spi::MODE_0;
use rp_pico::hal::Clock;
use rp_pico::hal::Spi;
use rp_pico::hal::I2C;
use rp_pico::hal::gpio::FunctionSpi as SPI;
use rp_pico::hal::pac;
use rp_pico::hal;

#[entry]
fn main() -> ! {
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

  let i2c = I2C::i2c0(
    pac.I2C0,
    pins.gpio4.into_mode(), // SDA
    pins.gpio5.into_mode(), // SCL
    400.kHz(),
    &mut pac.RESETS,
    125_000_000.Hz(),
  );

  let cs = pins.gpio17.into_push_pull_output();
  let _ = pins.gpio18.into_mode::<SPI>();
  let _ = pins.gpio19.into_mode::<SPI>();

  let spi = Spi::<_, _, 8>::new(pac.SPI0).init(
    &mut pac.RESETS,
    clocks.peripheral_clock.freq(),
    4_000_000.Hz(),
    &MODE_0
  );

  let keys : [keypad::Key; 16]= core::array::from_fn(|i| {
    let mut key = keypad::Key::default();

    key.pressed_color = keypad::Color { r: 0, g: 255, b: 0};

    key
  });

  let mut keypad = keypad::Keypad {
    keys,
    i2c,
    spi,
    cs
  };


  loop {
    keypad.update().unwrap();
    keypad.set_leds().unwrap();
  }
}