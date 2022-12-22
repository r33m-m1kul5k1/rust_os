#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(CrabOS::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use bootloader::BootInfo;
use CrabOS::{memory::pmm::FrameDistributer, log, hlt_loop, test_panic_handler, println};
use x86_64::structures::paging::FrameAllocator;

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {

    log::logger::init(log::LevelFilter::Info);

    let mut distributer = FrameDistributer::new(&boot_info.memory_map);
    
    for _ in 1..20 {
        
        println!("frame {:?} allocated", distributer.allocate_frame().unwrap());
    }
    
    for _ in 1..40 {
        
        println!("region: {:?}", distributer.get_region().unwrap());
    }
    
    test_main();
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
