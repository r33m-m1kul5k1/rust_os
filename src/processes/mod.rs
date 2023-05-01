//! this module includes:
//! 1. definisions of process and thread objects
//! 2. context switch - change the flow of execusion
//! 3. a scheduler

pub mod objects;
pub mod scheduler;

use lazy_static::lazy_static;
use log::{error, info};
use scheduler::Scheduler;
use spin::Mutex;

use self::objects::ProcessData;

lazy_static! {
    pub static ref KERNEL_SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::empty());
}

pub fn spawn_process(process_code: u64) -> usize {
    KERNEL_SCHEDULER.lock().push_process(process_code)
}

pub fn execute_process(pid: usize) {

    // To release the scheduler lock we must end it's lifetime with {}.
    let process = { KERNEL_SCHEDULER.lock().get_process(pid) };
    match process {
        Ok(process) => process.execute(),
        Err(()) => {
            error!("cannot execute process with pid {:#x}", pid)
        }
    }
}

pub fn pause_process(pid: u64) {
    
}

pub fn get_process_info(pid: usize) -> Option<ProcessData> {
    KERNEL_SCHEDULER.try_lock().unwrap().get_process_info(pid).ok()
}

pub fn kill_process(pid: usize) -> ! {
    let process = { KERNEL_SCHEDULER.try_lock().unwrap().terminate_process(pid) };
    match process {
        Ok(()) => {
            info!("succesfully killed process: {:#x}", pid);
        }
        Err(()) => error!("failed to kill process: {:#x}", pid),
    }
    loop {}
}
