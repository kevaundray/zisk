/* Linker script for mixed_stack_ops test with data section */
ENTRY(_start)

MEMORY
{
    ROM (rx) : ORIGIN = 0x80000000, LENGTH = 0x10000000
    RAM (rw) : ORIGIN = 0xa0020000, LENGTH = 0x1FFE0000
}

SECTIONS
{
    . = 0x80000000;
    
    .text : {
        *(.text.init)
        *(.text)
    } > ROM
    
    .rodata : {
        *(.rodata)
    } > ROM
    
    /* Place data sections in RAM */
    . = ORIGIN(RAM);
    
    .data : {
        *(.data)
    } > RAM AT > ROM
    
    .bss : {
        *(.bss)
    } > RAM
    
    _end = .;
}