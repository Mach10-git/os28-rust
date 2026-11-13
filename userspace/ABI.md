# OS 28 x86-64 userspace ABI — frozen v1

**Status:** v1 contract. Reserved syscall numbers and semantics are not to be repurposed; extensions use new numbers.

OS 28 v1 uses the System V AMD64 function calling convention and the Linux x86-64 syscall convention. This deliberate compatibility target lets OS 28 run statically linked `x86_64-linux-musl` ELF programs as kernel support is brought up. The kernel remains independent Rust code; Linux compatibility here is an application ABI, not Linux source reuse.

## System call instruction and registers

Userspace executes `syscall`. Registers at entry:

| Register | Meaning |
| --- | --- |
| `rax` | syscall number; result on return |
| `rdi`, `rsi`, `rdx` | arguments 1–3 |
| `r10`, `r8`, `r9` | arguments 4–6 |
| `rcx`, `r11` | clobbered by the instruction |

Success returns a nonnegative result in `rax`. Errors return `-errno` in the range `-1..=-4095`, as on Linux x86-64. User pointers are validated and copied by the kernel; they are never dereferenced directly while handling a syscall.

## v1 syscall numbers

These numbers retain Linux x86-64 meanings for musl compatibility:

| Number | Name | v1 contract |
| ---: | --- | --- |
| 0 | `read` | Read from a file descriptor |
| 1 | `write` | Write to a file descriptor |
| 3 | `close` | Close a file descriptor |
| 5 | `fstat` | Return file metadata |
| 9 | `mmap` | Map anonymous/private pages |
| 10 | `mprotect` | Change user page permissions |
| 11 | `munmap` | Unmap user pages |
| 12 | `brk` | Set/query the process heap end |
| 13 | `rt_sigaction` | Install/inspect supported signal actions |
| 14 | `rt_sigprocmask` | Update the process signal mask |
| 60 | `exit` | Terminate the calling thread |
| 63 | `uname` | Return OS 28 identity and ABI version |
| 158 | `arch_prctl` | `ARCH_SET_FS` and `ARCH_GET_FS` for TLS |
| 202 | `futex` | Private wait/wake operations |
| 218 | `set_tid_address` | Set clear-child-tid address |
| 228 | `clock_gettime` | Monotonic and realtime clocks |
| 231 | `exit_group` | Terminate the process |

The table specifies the frozen v1 ABI. It does **not** assert that every call is already implemented; kernel support is tracked in `syscalls.md`.

## Process and memory layout

User virtual addresses are canonical lower-half addresses below `0x0000_8000_0000_0000`. The initial ELF load range is `0x0000_0000_0040_0000` upward; the user stack grows down from `0x0000_7fff_ffff_f000`. The kernel is supervisor-only in the upper half. Page size is 4096 bytes. The initial process starts with `argc`, `argv[]`, `envp[]`, and an auxiliary vector on a 16-byte-aligned stack, following the x86-64 System V process-entry convention.

ELF ABI: ELF64, little-endian, `EM_X86_64`, static `ET_EXEC` first; PIE/`ET_DYN`, dynamic linking, TLS, signals, and SMP are later milestones. `stdin`, `stdout`, and `stderr` are descriptors 0, 1, and 2.

## C ABI smoke program

`smoke/hello.c` is the first acceptance program. It is intended to be linked statically with musl for `x86_64-linux-musl`; the kernel will provide the compatible syscall subset needed by its startup path and `write`/`exit_group`.
