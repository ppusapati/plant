/* STM32H743VIT6 Memory Layout */
MEMORY
{
    /* Flash: 2MB */
    FLASH  (rx)  : ORIGIN = 0x08000000, LENGTH = 2048K

    /* DTCM RAM: 128KB - Fast, tightly coupled (stack, critical data) */
    DTCM   (rwx) : ORIGIN = 0x20000000, LENGTH = 128K

    /* AXI SRAM: 512KB - Main RAM */
    RAM    (rwx) : ORIGIN = 0x24000000, LENGTH = 512K

    /* SRAM1: 128KB */
    SRAM1  (rwx) : ORIGIN = 0x30000000, LENGTH = 128K

    /* SRAM2: 128KB */
    SRAM2  (rwx) : ORIGIN = 0x30020000, LENGTH = 128K

    /* SRAM3: 32KB */
    SRAM3  (rwx) : ORIGIN = 0x30040000, LENGTH = 32K

    /* SRAM4: 64KB - Backup domain */
    SRAM4  (rwx) : ORIGIN = 0x38000000, LENGTH = 64K
}

/* Stack in DTCM for fast access */
_stack_start = ORIGIN(DTCM) + LENGTH(DTCM);
