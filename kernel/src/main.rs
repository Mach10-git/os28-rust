#![no_std]
#![no_main]

use core::{arch::{asm, global_asm}, fmt::{self, Write}, panic::PanicInfo, ptr, sync::atomic::{AtomicU64, AtomicUsize, AtomicU8, Ordering}};

mod gui;
mod scheduler;

// Limine protocol: request markers, base revision 3, and a framebuffer request.
#[used]
#[unsafe(link_section = ".limine_requests_start")]
static REQUESTS_START: [u64; 4] = [
    0xf6b8f4b39de7d1ae, 0xfab91a6940fcb9cf,
    0x785c6ed015d3e316, 0x181e920a7852b9d9,
];

#[used]
#[unsafe(link_section = ".limine_requests")]
static mut BASE_REVISION: [u64; 3] = [
    0xf9562b2d5c95a6c8, 0x6a7b384944536bdc, 3,
];

#[repr(C)]
pub(crate) struct Framebuffer { pub(crate) address: *mut u8, pub(crate) width: u64, pub(crate) height: u64, pub(crate) pitch: u64, pub(crate) bpp: u16, memory_model: u8, pub(crate) red_mask_size: u8, pub(crate) red_mask_shift: u8, pub(crate) green_mask_size: u8, pub(crate) green_mask_shift: u8, pub(crate) blue_mask_size: u8, pub(crate) blue_mask_shift: u8, _unused: [u8; 7], edid_size: u64, edid: *mut u8, mode_count: u64, modes: *mut *mut u8 }
#[repr(C)]
struct FramebufferResponse { revision: u64, count: u64, framebuffers: *mut *mut Framebuffer }
#[repr(C)]
struct FramebufferRequest { id: [u64; 4], revision: u64, response: *mut FramebufferResponse }

#[used]
#[unsafe(link_section = ".limine_requests")]
static mut FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest {
    id: [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b, 0x9d5827dcd881dd75, 0xa3148604f6fab11b],
    revision: 0,
    response: ptr::null_mut(),
};

#[used]
#[unsafe(link_section = ".limine_requests_end")]
static REQUESTS_END: [u64; 2] = [0xadc0e0531bb10d03, 0x9572709f31764c62];

struct Serial;
impl Serial {
    unsafe fn out(port: u16, value: u8) { unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags)); } }
    unsafe fn input(port: u16) -> u8 { let value: u8; unsafe { asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags)); } value }
    fn init() { unsafe { Self::out(0x3f9, 0); Self::out(0x3fb, 0x80); Self::out(0x3f8, 3); Self::out(0x3f9, 0); Self::out(0x3fb, 3); Self::out(0x3fa, 0xc7); Self::out(0x3fc, 0x0b); } }
}
impl Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            unsafe { while Self::input(0x3fd) & 0x20 == 0 {} Self::out(0x3f8, byte); }
        }
        Ok(())
    }
}
macro_rules! serial_println { ($($arg:tt)*) => {{ let _ = writeln!(Serial, $($arg)*); }} }

// QEMU's PC-compatible machine exposes the legacy 8259 PIC, PIT, i8042
// keyboard controller, and COM1 UART. These are deliberately tiny polling/
// interrupt-driven drivers, not full ACPI/PCI device discovery drivers.
const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xa0;
const PIC2_DATA: u16 = 0xa1;
const IDT_SIZE: usize = 256;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry { offset_low: u16, selector: u16, ist: u8, attributes: u8, offset_mid: u16, offset_high: u32, reserved: u32 }
impl IdtEntry {
    const MISSING: Self = Self { offset_low: 0, selector: 0, ist: 0, attributes: 0, offset_mid: 0, offset_high: 0, reserved: 0 };
    fn interrupt_gate(handler: usize) -> Self {
        Self { offset_low: handler as u16, selector: 0x28, ist: 0,
            attributes: 0x8e, offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32, reserved: 0 }
    }
}
#[repr(C, packed)]
struct Idtr { limit: u16, base: u64 }
static mut IDT: [IdtEntry; IDT_SIZE] = [IdtEntry::MISSING; IDT_SIZE];
static TICKS: AtomicU64 = AtomicU64::new(0);
static KEY_QUEUE: [AtomicU8; 128] = [const { AtomicU8::new(0) }; 128];
static KEY_HEAD: AtomicUsize = AtomicUsize::new(0);
static KEY_TAIL: AtomicUsize = AtomicUsize::new(0);

fn enqueue_key(byte: u8) {
    let head = KEY_HEAD.load(Ordering::Relaxed);
    let next = (head + 1) % KEY_QUEUE.len();
    if next != KEY_TAIL.load(Ordering::Acquire) {
        KEY_QUEUE[head].store(byte, Ordering::Relaxed);
        KEY_HEAD.store(next, Ordering::Release);
    }
}

fn dequeue_key() -> Option<u8> {
    let tail = KEY_TAIL.load(Ordering::Relaxed);
    if tail == KEY_HEAD.load(Ordering::Acquire) { return None; }
    let byte = KEY_QUEUE[tail].load(Ordering::Relaxed);
    KEY_TAIL.store((tail + 1) % KEY_QUEUE.len(), Ordering::Release);
    Some(byte)
}

global_asm!(r#"
    .macro IRQ_STUB name, number
    .global \name
\name:
    cld
    push rax
    push rcx
    push rdx
    push rbx
    push rbp
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    mov r12, rsp
    mov rsi, r12
    and rsp, -16
    mov edi, \number
    call irq_dispatch
    mov rsp, rax
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rbp
    pop rbx
    pop rdx
    pop rcx
    pop rax
    iretq
    .endm
    IRQ_STUB irq0_stub, 0
    IRQ_STUB irq1_stub, 1
"#);
unsafe extern "C" { fn irq0_stub(); fn irq1_stub(); }

fn io_wait() { unsafe { asm!("out 0x80, al", in("al") 0u8, options(nomem, nostack, preserves_flags)); } }
unsafe fn outb(port: u16, value: u8) { unsafe { asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags)); } }
unsafe fn inb(port: u16) -> u8 { let value: u8; unsafe { asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags)); } value }

fn remap_pic() {
    unsafe {
        let mask1 = inb(PIC1_DATA); let mask2 = inb(PIC2_DATA);
        outb(PIC1_CMD, 0x11); io_wait(); outb(PIC2_CMD, 0x11); io_wait();
        outb(PIC1_DATA, 0x20); io_wait(); outb(PIC2_DATA, 0x28); io_wait();
        outb(PIC1_DATA, 4); io_wait(); outb(PIC2_DATA, 2); io_wait();
        outb(PIC1_DATA, 1); io_wait(); outb(PIC2_DATA, 1); io_wait();
        // Enable only PIT (IRQ0) and keyboard (IRQ1); mask all other IRQs.
        outb(PIC1_DATA, (mask1 & !0x03) | 0xfc); outb(PIC2_DATA, 0xff);
    }
}

fn init_idt() {
    unsafe {
        let idt = ptr::addr_of_mut!(IDT);
        (*idt)[32] = IdtEntry::interrupt_gate(irq0_stub as usize);
        (*idt)[33] = IdtEntry::interrupt_gate(irq1_stub as usize);
        let descriptor = Idtr { limit: (core::mem::size_of::<[IdtEntry; IDT_SIZE]>() - 1) as u16, base: idt as u64 };
        asm!("lidt [{}]", in(reg) &descriptor, options(readonly, nostack, preserves_flags));
    }
}

fn init_pit() {
    // PIT channel 0, square wave, approximately 100 interrupts per second.
    let divisor: u16 = 11_932;
    unsafe { outb(0x43, 0x36); outb(0x40, divisor as u8); outb(0x40, (divisor >> 8) as u8); }
}

fn keyboard_char(scancode: u8) -> Option<u8> {
    // US QWERTY Set-1 make codes; ignore key releases and extended keys.
    const MAP: [u8; 58] = [0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, b'\t', b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's', b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v', b'b', b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' '];
    if scancode & 0x80 != 0 || scancode >= MAP.len() as u8 { None }
    else { let byte = MAP[scancode as usize]; if byte == 0 { None } else { Some(byte) } }
}

#[unsafe(no_mangle)]
extern "C" fn irq_dispatch(irq: u32, saved_rsp: u64) -> u64 {
    match irq {
            0 => {
                TICKS.fetch_add(1, Ordering::Relaxed);
                unsafe { outb(PIC1_CMD, 0x20); }
                scheduler::on_timer(saved_rsp)
            }
            1 => {
                let code = unsafe { inb(0x60) };
                if let Some(ch) = keyboard_char(code) { enqueue_key(ch); }
                unsafe { outb(PIC1_CMD, 0x20); }
                saved_rsp
            }
            _ => saved_rsp
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    Serial::init();
    serial_println!("OS 28 — Rust x86-64 kernel");
    serial_println!("Booted through the Limine protocol.");
    let mut framebuffer: *const Framebuffer = ptr::null();
    unsafe {
        let response = FRAMEBUFFER_REQUEST.response;
        if !response.is_null() && (*response).count > 0 {
            let fb = *(*response).framebuffers;
            if !fb.is_null() && (*fb).bpp == 32 {
                framebuffer = fb;
                gui::draw_desktop(&*fb);
                serial_println!("Framebuffer: {}x{} @ {} bpp", (*fb).width, (*fb).height, (*fb).bpp);
            }
        } else {
            serial_println!("No framebuffer response; continuing in serial mode.");
        }
        // Confirm the bootloader honored the requested base revision.
        if ptr::read_volatile(ptr::addr_of!(BASE_REVISION[2])) != 0 {
            serial_println!("Limine base revision 3 unavailable.");
        }
    }
    init_idt();
    remap_pic();
    init_pit();
    unsafe { scheduler::init(); }
    serial_println!("QEMU legacy drivers: COM1 ready, PIT ~100 Hz, PS/2 keyboard ready.");
    serial_println!("Kernel scheduler started with two preemptible worker tasks.");
    serial_println!("PS/2 input is buffered; type on the emulated keyboard.");
    unsafe { asm!("sti", options(nomem, nostack)); }
    loop {
        while let Some(ch) = dequeue_key() {
            if !framebuffer.is_null() { gui::draw_key(unsafe { &*framebuffer }, ch); }
            if ch == b'\n' { let _ = Serial.write_str("\r\n"); }
            else if ch == 8 { let _ = Serial.write_str("\x08 \x08"); }
            else { let _ = Serial.write_str(unsafe { core::str::from_utf8_unchecked(core::slice::from_ref(&ch)) }); }
        }
        // STI's one-instruction delay makes this pair safe against lost wakeups.
        unsafe { asm!("sti; hlt", options(nomem, nostack)); }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    Serial::init();
    serial_println!("KERNEL PANIC: {}", info);
    loop { unsafe { asm!("cli; hlt", options(nomem, nostack)); } }
}
