use rp2040_hal::multicore::Stack;

static mut CORE1_STACK: Stack<4096> = Stack::new();

