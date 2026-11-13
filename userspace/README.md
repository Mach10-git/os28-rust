# Userspace and musl port status

There is not yet a userspace ABI in OS 28, so this project does not claim to contain a musl port.

The current scheduler switches between three kernel execution contexts in ring 0. It does not create isolated address spaces, enter ring 3, load ELF executables, or implement process lifecycle and file descriptors. A musl build made for Linux would call Linux syscalls and cannot run on this kernel.

## Required work before musl can run

1. Define and freeze the x86-64 syscall ABI and userspace calling convention.
2. Add privilege transition support, per-task kernel stacks, and syscall entry/return code.
3. Add page-table ownership, user/kernel page permissions, and safe copy-in/copy-out routines.
4. Implement ELF loading, process creation/exit/wait, scheduling across address spaces, and a minimal VFS/file-descriptor layer.
5. Port musl's OS-specific syscall layer and startup objects to that ABI, then build and run small C programs.

The first useful musl milestone should be `write`, `exit`, `brk`/`mmap`, `openat`, `read`, `close`, `fstat`, and `clock_gettime`, plus a static ELF loader. Until those exist, a nominal musl target would only compile and would not execute correctly.
