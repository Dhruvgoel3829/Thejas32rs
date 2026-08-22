#define UART_RHR (*((volatile unsigned char *)0x10000000))
#define UART_THR (*((volatile unsigned char *)0x10000000))
#define UART_LSR (*((volatile unsigned char *)0x10000005))

void uart_putc(char c) {
    while ((UART_LSR & 0x20) == 0); // Wait until TX empty
    UART_THR = c;
}

void uart_puts(const char *s) {
    while (*s) {
        uart_putc(*s++);
    }
}

char uart_getc(void) {
    while ((UART_LSR & 0x01) == 0); // Wait until character is received
    return UART_RHR;
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\n==============================\n");
    uart_puts("   THEJAS32 Interactive Echo  \n");
    uart_puts("Type characters and press Enter\n");
    uart_puts("==============================\n> ");

    while (1) {
        char c = uart_getc();
        if (c == '\r' || c == '\n') {
            uart_puts("\r\n> ");
        } else {
            uart_putc(c); // Echo character back
        }
    }
}