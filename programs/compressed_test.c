// programs/compressed_test.c

#define UART_THR (*((volatile unsigned char *)0x10000000))
#define UART_LSR (*((volatile unsigned char *)0x10000005))

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

int calculate_factorial(int n) {
    int res = 1;
    for (int i = 2; i <= n; i++) {
        res *= i;
    }
    return res;
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("   THEJAS32 RV32IC Compressed Test    \n");
    uart_puts("=======================================\n");

    int fact = calculate_factorial(5);

    if (fact == 120) {
        uart_puts("  [PASS] 16-bit compressed instructions executed!\n");
        uart_puts(">>> RV32IC VERIFICATION PASSED <<<\n");
    } else {
        uart_puts(">>> TEST FAILED <<<\n");
    }

    // Terminate emulator by jumping to PC = 0
    __asm__ volatile("jr zero");
}