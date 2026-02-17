/*
 * ZisK linker script for riscv64imac-unknown-none-elf target.
 * Memory layout matches addresses in ziskos_definitions.rs.
 */

ENTRY(_start)

MEMORY
{
    ROM (rx)  : ORIGIN = 0x80000000, LENGTH = 128M
    RAM (rwx) : ORIGIN = 0xa0000000, LENGTH = 512M
}

SECTIONS
{
    .text : ALIGN(4)
    {
        *(.text.init)
        *(.text .text.*)
    } > ROM

    .rodata : ALIGN(8)
    {
        *(.srodata .srodata.*)
        *(.rodata .rodata.*)
    } > ROM

    .data : ALIGN(8)
    {
        _global_pointer = . + 0x800;
        *(.sdata .sdata.*)
        *(.data .data.*)
    } > RAM AT > ROM

    .bss (NOLOAD) : ALIGN(8)
    {
        _bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        _bss_end = .;
    } > RAM

    . = ALIGN(8);
    _kernel_heap_bottom = .;

    . = ORIGIN(RAM) + LENGTH(RAM);
    _init_stack_top = .;
}
