//! Tiny, single-core, preemptive kernel-thread scheduler for the first OS 28 milestone.
//! Contexts are switched from the PIT IRQ stack frame; no heap allocator is needed.

use core::{arch::asm, hint::spin_loop, ptr, sync::atomic::{AtomicU64, AtomicUsize, Ordering}};

const TASK_COUNT: usize = 3;
const STACK_SIZE: usize = 16 * 1024;
static mut STACKS: [[u8; STACK_SIZE]; TASK_COUNT] = [[0; STACK_SIZE]; TASK_COUNT];
static SAVED_RSP: [AtomicU64; TASK_COUNT] = [const { AtomicU64::new(0) }; TASK_COUNT];
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static WORK_A: AtomicU64 = AtomicU64::new(0);
static WORK_B: AtomicU64 = AtomicU64::new(0);

extern "C" fn worker_a() -> ! {
    loop { WORK_A.fetch_add(1, Ordering::Relaxed); spin_loop(); }
}
extern "C" fn worker_b() -> ! {
    loop { WORK_B.fetch_add(1, Ordering::Relaxed); spin_loop(); }
}

unsafe fn make_initial_stack(stack: *mut u8, entry: usize) -> u64 {
    let mut sp = unsafe { stack.add(STACK_SIZE) } as usize & !0xf;
    // iretq frame followed by the general-purpose-register frame consumed by irq stubs.
    let initial = [
        0x202_u64, 0x28, entry as u64,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // rax..r15
    ];
    for value in initial {
        sp -= core::mem::size_of::<u64>();
        unsafe { ptr::write(sp as *mut u64, value); }
    }
    sp as u64
}

pub unsafe fn init() {
    unsafe {
        let stacks = ptr::addr_of_mut!(STACKS);
        SAVED_RSP[0].store(0, Ordering::Relaxed); // The boot/idle context is saved on its first timer IRQ.
        SAVED_RSP[1].store(make_initial_stack((*stacks)[1].as_mut_ptr(), worker_a as usize), Ordering::Relaxed);
        SAVED_RSP[2].store(make_initial_stack((*stacks)[2].as_mut_ptr(), worker_b as usize), Ordering::Relaxed);
    }
    CURRENT.store(0, Ordering::Release);
}

/// Called by the timer IRQ with the complete saved register frame. Returns the next frame.
pub fn on_timer(saved_rsp: u64) -> u64 {
    let current = CURRENT.load(Ordering::Relaxed);
    SAVED_RSP[current].store(saved_rsp, Ordering::Release);
    let next = (current + 1) % TASK_COUNT;
    CURRENT.store(next, Ordering::Release);
    let next_rsp = SAVED_RSP[next].load(Ordering::Acquire);
    if next_rsp == 0 { saved_rsp } else { next_rsp }
}

pub fn counters() -> (u64, u64) {
    (WORK_A.load(Ordering::Relaxed), WORK_B.load(Ordering::Relaxed))
}

/// Keep the compiler from assuming an interrupt returns only to the same stack.
pub unsafe fn idle() -> ! {
    loop { unsafe { asm!("sti; hlt", options(nomem, nostack)); } }
}
