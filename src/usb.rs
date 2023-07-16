use rp_pico::hal::usb::UsbBus;
use rp_pico::pac::{self, interrupt};

use usb_device::{
    prelude::*,
    class_prelude::*,
    UsbError
};

use usbd_hid::descriptor::{
    generator_prelude::*,
    KeyboardReport,
    MediaKeyboardReport,
    SystemControlReport,
};
use usbd_hid::hid_class::HIDClass;

type Device = UsbDevice<'static, UsbBus>;
type Bus = UsbBusAllocator<UsbBus>;
type Hid = HIDClass<'static, UsbBus>;

static mut USB_DEVICE: Option<Device> = None;
static mut USB_BUS: Option<Bus> = None;

static mut HID_KEYBOARD: Option<Hid> = None;
static mut HID_SYSCTL: Option<Hid> = None;
static mut HID_MEDIA: Option<Hid> = None;

pub fn usb_init(bus_allocator: Bus) {
    // Safety: interrupts haven't been started yet.
    let bus_ref = unsafe { 
        USB_BUS = Some(bus_allocator);
        // Safety: taking a mutable reference to the Bus is now instant UB.
        // DON'T DO IT.
        USB_BUS.as_ref().unwrap()
    };

    let hid_keyboard = HIDClass::new(bus_ref, KeyboardReport::desc(), 60);
    let hid_sysctl = HIDClass::new(bus_ref, SystemControlReport::desc(), 60);
    let hid_media = HIDClass::new(bus_ref, MediaKeyboardReport::desc(), 60);

    unsafe {
        HID_KEYBOARD = Some(hid_keyboard);
        HID_SYSCTL = Some(hid_sysctl);
        HID_MEDIA = Some(hid_media);
    }

    let usb_device = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x0011, 0x0))
        .manufacturer("Colonial")
        .product("Hexapad")
        .serial_number("0")
        .device_class(0)
        .build();

    unsafe {
        USB_DEVICE = Some(usb_device);
    }

    unsafe {
        // Enable USB interrupt
        pac::NVIC::unmask(pac::Interrupt::USBCTRL_IRQ);
    }
}

pub fn push_keyboard(report: KeyboardReport) -> Result<usize, UsbError> {
    critical_section::with(|_| unsafe {
        HID_KEYBOARD.as_mut().map(|hid| hid.push_input(&report))
    })
    .unwrap()
}

/// Whenever the USB hardware generates an interrupt request, this function is called.
#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    // Safety: taking a mutable reference to these *should* be okay,
    // as the interrupt preempts the rest of the program (with the exception
    // of the display controller on core1, which can't touch these due to privacy rules.)
    let usb_dev = USB_DEVICE.as_mut().unwrap();

    let hid_keyboard = HID_KEYBOARD.as_mut().unwrap();
    let hid_sysctl = HID_SYSCTL.as_mut().unwrap();
    let hid_media = HID_MEDIA.as_mut().unwrap();

    usb_dev.poll(&mut [hid_keyboard, hid_sysctl, hid_media]);

    // This is needed for reasons only known to the wizards
    // at the USB-IF (it has something to do with caps lock LEDs?)
    let mut throwaway_buf = [0; 64];
    let _ = hid_keyboard.pull_raw_output(&mut throwaway_buf);
    let _ = hid_sysctl.pull_raw_output(&mut throwaway_buf);
    let _ = hid_media.pull_raw_output(&mut throwaway_buf);
}