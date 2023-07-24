use core::convert::Infallible;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::*;
use rp_pico::hal::gpio::bank0::*;
use rp_pico::hal::gpio::{FunctionI2C, Output, Pin, PushPull};
use rp_pico::hal::i2c::Error;
use rp_pico::hal::spi::Enabled;
use rp_pico::hal::timer::Instant;
use rp_pico::hal::{Spi, I2C};
use rp_pico::pac::{I2C0, SPI0};

use crate::utils::{now, Duration};

type KeyI2c = I2C<I2C0, (Pin<Gpio4, FunctionI2C>, Pin<Gpio5, FunctionI2C>)>;
type LedSpi = Spi<Enabled, SPI0, 8>;
type CS = Pin<Gpio17, Output<PushPull>>;

pub struct Keypad {
    pub keys: [Key; 16],
    brightness: u8,
    i2c: KeyI2c,
    spi: LedSpi,
    cs: CS,
}

impl Keypad {
    const NUM_KEYS: usize = 16;
    const START_FRAME: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
    const END_FRAME: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
    const KEYPAD_ADDR: u8 = 0x20;
}

impl Keypad {
    pub fn new(i2c: KeyI2c, spi: LedSpi, cs: CS) -> Self {
        Self {
            keys: core::array::from_fn(|_| Key::new()),
            brightness: 0,
            i2c,
            spi,
            cs,
        }
    }

    pub fn update(&mut self) -> impl Iterator<Item = (u8, KeyEvent)> {
        // Yes, this is *technically* out of order, but updates happen
        // so fast that it doesn't really matter.
        self.update_leds().unwrap();
        self.update_state().unwrap()
    }

    pub fn set_colors(&mut self, colors: [(Color, Color); 16]) {
        // I need to index into both, actually
        #[allow(clippy::needless_range_loop)]
        for i in 0..Self::NUM_KEYS {
            self.keys[i].default_color = colors[i].0;
            self.keys[i].pressed_color = colors[i].1;
        }
    }

    /// Sets the brightness of the keypad LEDs.
    /// 
    /// Values lower than 0.0 or higher than 1.0 will be clamped to within that range.
    pub fn set_brightness(&mut self, brightness: f32) {
        let brightness = brightness.clamp(0.0, 1.0);
        // Map a percentage value (between 0.0 and 1.0) to a u8 between 224 and 255
        // (the brightness range accepted by the keypad LED protocol)
        self.brightness = 0b11100000 | (brightness * 0b11111 as f32) as u8;
    }

    fn update_leds(&mut self) -> Result<(), Infallible> {
        // Start SPI transaction
        self.cs.set_low()?;

        // https://cpldcpu.wordpress.com/2014/11/30/understanding-the-apa102-superled/
        // Start frame is 32 zero bits
        self.spi.write(&Self::START_FRAME)?;

        // 32 bit LED frame, one for each LED
        // <0xE0 + brightness> (30 brightness levels)
        // <B byte>
        // <G byte>
        // <R byte>
        for key in &self.keys {
            self.spi.write(&[self.brightness])?;
            self.spi.write(&key.color().as_bgr())?;
        }

        // End frame is 32 one bits
        // Not technically protocol-compliant (see above link)
        // but fine for this application since the number of LEDs is constant
        self.spi.write(&Self::END_FRAME)?;

        // End SPI transaction
        self.cs.set_high()?;

        Ok(())
    }

    fn update_state(&mut self) -> Result<impl Iterator<Item = (u8, KeyEvent)>, Error> {
        let mut buffer = [0_u8; 2];

        // Write zero constant to I2C bus
        // Unsure why exactly this is needed... but it is
        self.i2c.write(Self::KEYPAD_ADDR, &[0x0])?;

        // Read keypress states from the I2C bus into buffer
        self.i2c.read(Self::KEYPAD_ADDR, &mut buffer)?;

        // Bithacking to turn our two state bytes into a single u16,
        // where each bit represents the state of a key
        let state = !(buffer[0] as u16 | (buffer[1] as u16) << 8);

        // TODO Log Instant of last press (in order to support timed sleep mode)

        let mut events = [None; 16];

        // Update each key
        // Again, I need to index into both collections
        #[allow(clippy::needless_range_loop)]
        for i in 0..Self::NUM_KEYS {
            // Matches macro is ugly and unreadable
            #[allow(clippy::match_like_matches_macro)]
            let pressed = match state & (1 << i) {
                0 => false,
                _ => true,
            };

            events[i] = self.keys[i].update(pressed);
        }

        Ok(events
            .into_iter()
            .enumerate()
            .filter_map(|(i, event)| event.map(|e| (i as u8, e))))
    }
}

pub struct Key {
    pub default_color: Color,
    pub pressed_color: Color,
    pub last_pressed: Instant,
    pub pressed: bool,
    pub held: bool,
}

impl Key {
    const HOLD_TIME: Duration = Duration::millis(750);
}

impl Key {
    pub fn new() -> Self {
        Self {
            default_color: Color::new(16, 16, 16),
            pressed_color: Color::new(0, 255, 0),
            last_pressed: now(),
            pressed: false,
            held: false,
        }
    }

    pub fn update(&mut self, pressed: bool) -> Option<KeyEvent> {
        // New press
        if pressed && !self.pressed {
            self.last_pressed = now();
            self.pressed = true;

            Some(KeyEvent::Pressed)
        }
        // Old press (check to trigger hold event)
        else if (pressed && self.pressed) && (now() - self.last_pressed) >= Self::HOLD_TIME {
            self.held = true;

            Some(KeyEvent::Held)
        }
        // Released
        else if !pressed && self.pressed {
            self.pressed = false;
            self.held = false;

            Some(KeyEvent::Released)
        }
        // No event of note
        else {
            None
        }
    }

    pub fn color(&self) -> Color {
        match self.pressed {
            true => self.pressed_color,
            false => self.default_color,
        }
    }
}

#[derive(Clone, Copy)]
pub enum KeyEvent {
    Pressed,
    Held,
    Released,
}

#[derive(Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn as_bgr(&self) -> [u8; 3] {
        [self.b, self.g, self.r]
    }
}
