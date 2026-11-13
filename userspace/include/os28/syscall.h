#ifndef OS28_SYSCALL_H
#define OS28_SYSCALL_H

/* Raw v1 syscall entry for freestanding x86-64 C code. */
static inline long os28_syscall6(long number, long a1, long a2, long a3,
                                long a4, long a5, long a6) {
    register long r10 __asm__("r10") = a4;
    register long r8  __asm__("r8")  = a5;
    register long r9  __asm__("r9")  = a6;
    long result;
    __asm__ volatile ("syscall"
        : "=a"(result)
        : "a"(number), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8), "r"(r9)
        : "rcx", "r11", "memory");
    return result;
}

#define OS28_SYS_read          0
#define OS28_SYS_write         1
#define OS28_SYS_close         3
#define OS28_SYS_fstat         5
#define OS28_SYS_mmap          9
#define OS28_SYS_mprotect     10
#define OS28_SYS_munmap       11
#define OS28_SYS_brk          12
#define OS28_SYS_rt_sigaction 13
#define OS28_SYS_rt_sigprocmask 14
#define OS28_SYS_exit         60
#define OS28_SYS_uname        63
#define OS28_SYS_arch_prctl  158
#define OS28_SYS_futex       202
#define OS28_SYS_set_tid_address 218
#define OS28_SYS_clock_gettime 228
#define OS28_SYS_exit_group  231

#endif
