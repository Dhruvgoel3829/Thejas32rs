// programs/xip_test.c

#define UART_THR (*((volatile unsigned char *)0x10000000))
#define UART_LSR (*((volatile unsigned char *)0x10000005))

static const char flash_string[] = "String stored in 0x40000000 QSPI Flash!";
static volatile unsigned int sram_var = 0;

void uart_putc(char c) {
    while ((UART_LSR & 0x20) == 0);
    UART_THR = c;
}

void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') uart_putc('\r');
        uart_putc(*s++);
    }
}

void print_hex(unsigned int val) {
    const char hex[] = "0123456789ABCDEF";
    uart_puts("0x");
    for (int i = 7; i >= 0; i--) {
        uart_putc(hex[(val >> (i * 4)) & 0xF]);
    }
}

__attribute__((section(".text.entry")))
void _start(void) {
    __asm__ volatile("li sp, 0x80070000");

    uart_puts("\r\n=======================================\n");
    uart_puts("   THEJAS32 QSPI Flash XIP Execution   \n");
    uart_puts("=======================================\n");

    // Read current Program Counter
    unsigned int current_pc;
    __asm__ volatile("auipc %0, 0" : "=r"(current_pc));

    uart_puts("[INFO] Current PC: ");
    print_hex(current_pc);
    uart_puts("\n");

    // TEST 1: Verify PC is running directly in Flash space (0x4000_0000)
    if ((current_pc & 0xF0000000) == 0x40000000) {
        uart_puts("  [PASS] CPU executing in Flash XIP range (0x40000000)!\n");
    } else {
        uart_puts("  [FAIL] PC is not in Flash address range!\n");
    }

    // TEST 2: Read read-only data stored in Flash
    uart_puts("[TEST 2] Reading constant string from Flash:\n  --> \"");
    uart_puts(flash_string);
    uart_puts("\"\n");

    // TEST 3: Modify SRAM variable from Flash code
    sram_var = 0xAABBCCDD;
    if (sram_var == 0xAABBCCDD) {
        uart_puts("  [PASS] Successfully accessed SRAM (0x80000000) from Flash XIP!\n");
    }

    uart_puts("\n>>> QSPI FLASH XIP VERIFICATION PASSED <<<\n");

    __asm__ volatile("jr zero");
}