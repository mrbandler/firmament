#![no_std]
#![no_main]

use firmament_fm::{debug_log, read_volatile, wfi, write_volatile};

const GPIO_PORT0_ODR: *mut u32 = 0x4000_0014 as *mut u32;
const GPIO_PORT0_IDR: *const u32 = 0x4000_0010 as *const u32;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    debug_log("firmware booted");

    let mut toggle: u32 = 0;
    loop {
        toggle ^= 1;
        unsafe { write_volatile(GPIO_PORT0_ODR, toggle) };

        let val = unsafe { read_volatile(GPIO_PORT0_IDR) };
        let _ = val; // read-back, host printed it

        if toggle == 1 {
            debug_log("LED on");
        } else {
            debug_log("LED off");
        }

        // wfi();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    debug_log("PANIC");
    loop {
        wfi();
    }
}
