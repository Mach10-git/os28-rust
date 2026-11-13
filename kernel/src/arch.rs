//! x86-64 privilege tables and fast syscall entry.

use core::{arch::{asm, global_asm}, mem, ptr};

pub const KERNEL_CODE: u16 = 0x08;
pub const KERNEL_DATA: u16 = 0x10;
pub const USER_DATA: u16 = 0x1b;
pub const USER_CODE: u16 = 0x23;
const TSS_SELECTOR: u16 = 0x28;

#[repr(C, packed)]
struct Tss {
    reserved0: u32,
    rsp0: u64, rsp1: u64, rsp2: u64,
    reserved1: u64,
    ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    iomap_base: u16,
}
impl Tss {
    const fn empty() -> Self {
        Self { reserved0: 0, rsp0: 0, rsp1: 0, rsp2: 0, reserved1: 0, ist: [0; 7], reserved2: 0, reserved3: 0, iomap_base: mem::size_of::<Self>() as u16 }
    }
}

#[repr(C, packed)]
struct DescriptorTablePointer { limit: u16, base: u64 }
#[repr(C)]
struct CpuLocal { user_rsp: u64, kernel_rsp: u64 }

static mut GDT: [u64; 7] = [0; 7];
static mut TSS: Tss = Tss::empty();
static mut CPU_LOCAL: CpuLocal = CpuLocal { user_rsp: 0, kernel_rsp: 0 };

global_asm!(r#"
    .global os28_load_gdt
os28_load_gdt:
    lgdt [rdi]
    push 0x08
    lea rax, [rip + .Lreload_cs]
    push rax
    lretq
.Lreload_cs:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov ax, 0x28
    ltr ax
    ret

    .global os28_syscall_entry
os28_syscall_entry:
    swapgs
    mov qword ptr gs:[0], rsp
    mov rsp, qword ptr gs:[8]
    push rax
    push rdi
    push rsi
    push rdx
    push r10
    push r8
    push r9
    push rcx
    push r11
    mov rdi, rsp
    sub rsp, 8
    call os28_syscall_dispatch
    add rsp, 8
    mov qword ptr [rsp + 64], rax
    pop r11
    and r11, 0x202
    pop rcx
    pop r9
    pop r8
    pop r10
    pop rdx
    pop rsi
    pop rdi
    pop rax
    mov rsp, qword ptr gs:[0]
    swapgs
    sysretq
"#);

unsafe extern "C" {
    fn os28_load_gdt(table: *const DescriptorTablePointer);
    fn os28_syscall_entry();
}

unsafe fn wrmsr(msr: u32, value: u64) {
    unsafe { asm!("wrmsr", in("ecx") msr, in("eax") value as u32, in("edx") (value >> 32) as u32, options(nostack, preserves_flags)); }
}
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32; let high: u32;
    unsafe { asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nostack, preserves_flags)); }
    ((high as u64) << 32) | low as u64
}

pub unsafe fn init() {
    let mut initial_rsp: u64;
    unsafe { asm!("mov {}, rsp", out(reg) initial_rsp, options(nomem, nostack, preserves_flags)); }
    let tss_base = ptr::addr_of!(TSS) as u64;
    let limit = (mem::size_of::<Tss>() - 1) as u64;
    let descriptor_low = (limit & 0xffff)
        | ((tss_base & 0xffff) << 16)
        | (((tss_base >> 16) & 0xff) << 32)
        | (0x89 << 40)
        | (((limit >> 16) & 0xf) << 48)
        | (((tss_base >> 24) & 0xff) << 56);
    unsafe {
        let gdt = ptr::addr_of_mut!(GDT);
        (*gdt)[1] = 0x00af_9a00_0000_ffff; // ring 0 code
        (*gdt)[2] = 0x00cf_9200_0000_ffff; // ring 0 data
        (*gdt)[3] = 0x00cf_f200_0000_ffff; // ring 3 data
        (*gdt)[4] = 0x00af_fa00_0000_ffff; // ring 3 code
        (*gdt)[5] = descriptor_low;
        (*gdt)[6] = tss_base >> 32;
        ptr::addr_of_mut!(TSS.rsp0).write_unaligned(initial_rsp);
        let gdtr = DescriptorTablePointer { limit: (mem::size_of::<[u64; 7]>() - 1) as u16, base: gdt as u64 };
        os28_load_gdt(&gdtr);
        ptr::addr_of_mut!(CPU_LOCAL.kernel_rsp).write_unaligned(initial_rsp);
        wrmsr(0xc0000101, 0); // IA32_GS_BASE: user GS base on kernel entry
        wrmsr(0xc0000102, ptr::addr_of!(CPU_LOCAL) as u64); // IA32_KERNEL_GS_BASE
        let efer = rdmsr(0xc0000080);
        wrmsr(0xc0000080, efer | 1); // EFER.SCE
        wrmsr(0xc0000081, ((0x10u64) << 48) | ((KERNEL_CODE as u64) << 32)); // IA32_STAR
        wrmsr(0xc0000082, os28_syscall_entry as usize as u64); // IA32_LSTAR
        wrmsr(0xc0000084, (1 << 8) | (1 << 9) | (1 << 10)); // IA32_FMASK: clear TF/IF/DF
    }
}

pub unsafe fn set_kernel_stack(top: u64) {
    unsafe {
        ptr::addr_of_mut!(TSS.rsp0).write_unaligned(top);
        ptr::addr_of_mut!(CPU_LOCAL.kernel_rsp).write_unaligned(top);
    }
}

pub unsafe fn enter_user(entry: u64, stack: u64) -> ! {
    unsafe {
        asm!(
            "push {user_data}", "push {user_stack}", "push 0x202",
            "push {user_code}", "push {user_entry}", "iretq",
            user_data = in(reg) USER_DATA as u64,
            user_stack = in(reg) stack,
            user_code = in(reg) USER_CODE as u64,
            user_entry = in(reg) entry,
            options(noreturn)
        );
    }
}
