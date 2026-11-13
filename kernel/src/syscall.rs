//! Entry frame and Linux-compatible x86-64 syscall dispatch.

#[repr(C)]
pub struct SyscallFrame {
    pub rflags: u64,
    pub rip: u64,
    pub r9: u64,
    pub r8: u64,
    pub r10: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rax: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn os28_syscall_dispatch(frame: *mut SyscallFrame) -> i64 {
    if frame.is_null() { return -14; } // EFAULT
    // The ABI is frozen in userspace/ABI.md. Calls remain ENOSYS until the
    // process memory manager and VFS can validate every supplied user pointer.
    let _number = unsafe { (*frame).rax };
    -38 // ENOSYS
}
