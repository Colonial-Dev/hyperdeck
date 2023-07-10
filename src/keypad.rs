use core::convert::Infallible;

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::prelude::_embedded_hal_blocking_spi_Write;
use embedded_hal::prelude::_embedded_hal_blocking_i2c_Read;
use embedded_hal::prelude::_embedded_hal_blocking_i2c_Write;

use rp_pico::hal::{Spi, spi::Enabled, I2C, i2c::Error};
use rp_pico::pac::{SPI0, I2C0};
use rp_pico::hal::gpio::{
    bank0::*, Pin,
    FunctionI2C, 
    Output, PushPull
};

const KEYPAD_ADDR: u8 = 0x20;
const START_FRAME: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
const END_FRAME:   [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];


type KeyI2c = I2C<I2C0, (Pin<Gpio4, FunctionI2C>, Pin<Gpio5, FunctionI2C>)>;
type LedSpi = Spi<Enabled, SPI0, 8>;
type CS = Pin<Gpio17, Output<PushPull>>;

pub struct Keypad {
    pub keys: [Key; 16],
    pub i2c: KeyI2c,
    pub spi: LedSpi,
    pub cs: CS
}

impl Keypad {
    pub fn set_leds(&mut self) -> Result<(), Infallible> {
        self.cs.set_low()?;

        self.spi.write(&START_FRAME)?;

        for key in &self.keys {
            self.spi.write(&[0xFF])?;

            if key.pressed {
                self.spi.write(&[key.pressed_color.b])?;
                self.spi.write(&[key.pressed_color.g])?;
                self.spi.write(&[key.pressed_color.r])?;
            } else {
                self.spi.write(&[key.default_color.b])?;
                self.spi.write(&[key.default_color.g])?;
                self.spi.write(&[key.default_color.r])?;
            }
        }
        
        self.spi.write(&END_FRAME)?;

        self.cs.set_high()?;

        Ok(())
    }

    pub fn update(&mut self) -> Result<(), Error> {
        let mut buffer = [0_u8; 2];
        
        self.i2c.write(KEYPAD_ADDR, &[0x0])?;
        self.i2c.read(KEYPAD_ADDR, &mut buffer)?;

        let state = !(buffer[0] as u16 | (buffer[1] as u16) << 8);

        for i in 0..16 {
            let pressed = state & (1 << i);

            if pressed != 0 {
                self.keys[i].pressed = true;
            } else {
                self.keys[i].pressed = false;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {

}

pub struct Key {
    pub default_color: Color,
    pub pressed_color: Color,
    pub pressed: bool,
}

impl Default for Key {
    fn default() -> Self {
        Key {
            default_color: Color {r: 255, g: 255, b: 255},
            pressed_color: Color {r: 0, g: 255, b: 0},
            pressed: false
        }
    }
}