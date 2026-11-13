# Syscall implementation status

The numbers and register convention in `ABI.md` are the frozen v1 contract. Current kernel implementation status:

| Call | Status |
| --- | --- |
| `write`, `exit_group` | Not implemented |
| `read`, `close`, `fstat` | Not implemented |
| `mmap`, `mprotect`, `munmap`, `brk` | Not implemented |
| `rt_sigaction`, `rt_sigprocmask`, `futex` | Not implemented |
| `uname`, `arch_prctl`, `set_tid_address`, `clock_gettime` | Not implemented |

The kernel currently has no syscall entry, user-mode process, VFS, or page-fault handling. Do not treat the ABI header as proof that an application can run yet.
