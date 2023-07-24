use rp2040_hal::timer::{Timer, Instant};
use rp2040_hal::rosc::{RingOscillator, Enabled};

pub static mut TIMER: Option<Timer> = None;
pub static mut ROSC: Option<RingOscillator<Enabled>> = None;

pub type Duration = fugit::Duration<u32, 1, 100000>;

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

/// Get an Instant representing "now."
pub fn now() -> Instant {
    // Safety: get_counter is a read-only operation.
    unsafe { TIMER.as_ref().unwrap().get_counter() }
}

/// Simple busy-waiting implementation using [`now()`].
pub fn wait(duration_ms: u32) {
    let length = Duration::millis(duration_ms);
    let start = now();

    loop {
        if now() - start >= length {
            return;
        }
    }
}

/// Generates a random u32 in the range `[min, max]` using the RP2040's ring oscillator.
pub fn random(min: u32, max: u32) -> u32 {
    fn random_bit() -> u32 {
        // Safety: get_random_bit() is a read-only operation.
        unsafe {
            match ROSC.as_ref().unwrap().get_random_bit() {
                false => 0,
                true => 1
            }
        }
    }
    
    let mut random = 0_u32;

    for _ in 0..32 {
        random = random << 1 | random_bit();
        // NOPs artificially extend the timing of this loop to give the ROSC time to, well, oscillate.
        // Without this, we would see some bias.
        // See https://people.ece.cornell.edu/land/courses/ece4760/RP2040/C_SDK_random/index_random.html
        cortex_m::asm::nop();
        cortex_m::asm::nop();
    }

    random % (max - min + 1) + min
}