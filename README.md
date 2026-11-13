# OS 28 — Rust x86-64 foundation

This repository is the start of an independent, open-source x86-64 operating system. It is not Apple's macOS and does not include Apple's code, artwork, or proprietary components. The name is a working project name.

The current milestone is a freestanding Rust kernel that Limine can load. It initializes COM1, requests a framebuffer, draws a flat desktop and welcome window, installs an IDT for IRQ0/IRQ1, remaps the legacy 8259 PIC, and configures PIT channel 0 at about 100 Hz. PS/2 Set-1 keyboard input is queued and echoed to the framebuffer and COM1. A timer-driven round-robin scheduler switches among the boot context and two kernel worker tasks. There are no third-party Rust crates; the small amount of assembly handles x86 port I/O, IRQ entry/return, and `hlt`.

## Build the kernel

Install a recent stable Rust toolchain with the `x86_64-unknown-none` target, plus LLVM's `rust-lld` linker. From this directory:

```powershell
rustup target add x86_64-unknown-none
cargo build --release -p os28-kernel
```

The kernel ELF is at `target/x86_64-unknown-none/release/os28-kernel`.

## Boot it

Use an official Limine binary release and a QEMU PC-compatible machine (`qemu-system-x86_64`, default i440fx/PC machine). Make a bootable FAT/ISO image with this layout:

```text
/limine.conf
/boot/kernel.elf
```

Copy the built ELF to `boot/kernel.elf`, add the Limine boot files required by the chosen firmware mode, then boot the image, for example with `qemu-system-x86_64 -m 256M -cdrom os28.iso -serial stdio`. The QEMU display shows the framebuffer; `-serial stdio` shows kernel logs and keyboard echo. The project does not vendor a bootloader binary.

The IRQ setup currently handles only IRQ0 (PIT) and IRQ1 (PS/2 keyboard), with the remaining legacy PIC IRQ lines masked. The scheduler is kernel-thread-only: there are no ring-3 processes, address-space isolation, process syscalls, or ELF loading. There is no musl port yet; its prerequisites and status are in [`userspace/README.md`](userspace/README.md). The GUI is a static framebuffer desktop with keyboard text echo, not a window manager. Exception handlers, APIC/IOAPIC support, USB keyboards, mouse input, PCI enumeration, disk/network drivers, and a general device-driver framework are also not implemented. The keyboard driver handles basic US QWERTY Set-1 make codes only (no shift/caps-lock or extended keys).

## Design direction

The kernel is Rust-first (`no_std`); the Limine boot protocol is the only external boot interface. The UI uses an in-tree bitmap font and simple opaque shapes. Next milestones are exception handlers, physical and virtual memory management, ring-3 process isolation, syscall and ELF support, a VFS, ACPI/SMP discovery, then a functional window manager and application set.

## License

MIT. Limine is a separate project with its own license; obtain it from its official repository.
