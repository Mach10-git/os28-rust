ENTRY(_start)

SECTIONS {
    . = 0xffffffff80000000;
    .text : ALIGN(4K) { *(.text .text.*) }
    .rodata : ALIGN(4K) { KEEP(*(.limine_requests_start)) KEEP(*(.limine_requests)) KEEP(*(.limine_requests_end)) *(.rodata .rodata.*) }
    .data : ALIGN(4K) { *(.data .data.*) }
    .bss : ALIGN(4K) { *(COMMON) *(.bss .bss.*) }
    /DISCARD/ : { *(.eh_frame*) *(.comment) }
}
