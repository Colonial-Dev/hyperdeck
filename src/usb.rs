use rp_pico::hal::usb::UsbBus;
use rp_pico::pac::{self, interrupt};
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usb_device::UsbError;
use usbd_hid::descriptor::generator_prelude::*;
use usbd_hid::descriptor::KeyboardReport;
use usbd_hid::hid_class::HIDClass;
use usbd_serial::SerialPort;

type Device = UsbDevice<'static, UsbBus>;
type Bus = UsbBusAllocator<UsbBus>;
type Hid = HIDClass<'static, UsbBus>;
type Serial = SerialPort<'static, UsbBus>;

static mut USB_DEVICE: Option<Device> = None;
static mut USB_BUS: Option<Bus> = None;

static mut SERIAL: Option<Serial> = None;
static mut HID: Option<Hid> = None;

pub fn init(bus_allocator: Bus) {
    let bus_ref = unsafe {
        // Safety: interrupts haven't been started yet.
        USB_BUS = Some(bus_allocator);
        USB_BUS.as_ref().unwrap()
    };

    let hid = HIDClass::new(bus_ref, KeyboardReport::desc(), 60);
    let serial = SerialPort::new(bus_ref);

    unsafe {
        HID = Some(hid);
        SERIAL = Some(serial);
    }

    let usb_device = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x0011, 0x0))
        .manufacturer("Colonial")
        .product("Hyperdeck")
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

pub fn config_mode() {
    let mut found = false;

    critical_section::with(|_| unsafe {
        // Safety: taking a mutable reference to these is okay within a critical section.
        let usb_dev = USB_DEVICE.as_mut().unwrap();
        let hid = HID.as_mut().unwrap();
        let serial = SERIAL.as_mut().unwrap();

        let mut throwaway_buf = [0; 64];

        loop {
            if !usb_dev.poll(&mut [hid, serial]) {
                let _ = hid.pull_raw_output(&mut throwaway_buf);
                continue;
            }

            let mut magic_buf = [0u8; 5];

            match serial.read(&mut magic_buf[..]) {
                Ok(count) => {
                    if magic_buf == [b'H', b'Y', b'P', b'E', b'R'] {
                        found = true;
                        return;
                    }
                },
                Err(UsbError::WouldBlock) => {

                },
                Err(err) => Err(err).unwrap() 
            }
        }
    });

    if found {
        panic!("Magic recevied :)")
    }
}

pub fn push_report(report: [u8; 8]) -> Result<usize, UsbError> {
    todo!()
}

pub fn push_keyboard(report: KeyboardReport) -> Result<usize, UsbError> {
    critical_section::with(|_| unsafe { HID.as_mut().map(|hid| hid.push_input(&report)) })
        .unwrap()
}

/// Whenever the USB hardware generates an interrupt request, this function is called.
#[allow(non_snake_case)]
#[interrupt]
unsafe fn USBCTRL_IRQ() {
    // Safety: taking a mutable reference to these is okay,
    // as the interrupt preempts the rest of the program.
    let usb_dev = USB_DEVICE.as_mut().unwrap();

    let hid = HID.as_mut().unwrap();
    let serial = SERIAL.as_mut().unwrap();

    usb_dev.poll(&mut [hid, serial]);

    // This is needed for reasons only known to the wizards
    // at the USB-IF (it has something to do with caps lock LEDs?)
    let mut throwaway_buf = [0; 64];
    let _ = hid.pull_raw_output(&mut throwaway_buf);
    let _ = serial.read(&mut throwaway_buf);
}
