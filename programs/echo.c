#define UART_RHR (*((volatile unsigned char *)0x10000000))
#define UART_THR (*((volatile unsigned char *)0x10000000))
#define UART_LSR (*((volatile unsigned char *)0x10000005))

void uart_putc(char c) {
    while ((UART_LSR & 0x20) == 0);
    UART_THR = c;
}

void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') {
            uart_putc('\r'); // Explicit Carriage Return before Line Feed
        }
        uart_putc(*s++);
    }
}

char uart_getc(void) {
    while ((UART_LSR & 0x01) == 0);
    return UART_RHR;
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\n==============================\n");
    uart_puts("   THEJAS32 Interactive Echo  \n");
    uart_puts("Type characters and press Enter\n");
    uart_puts("==============================\n> ");

    while (1) {
        char c = uart_getc();

        if (c == '\r' || c == '\n') {
            uart_puts("\n> ");
        } else if (c == 0x08 || c == 0x7F) { // Backspace / Delete
            // Erase the character visually on the terminal
            uart_putc('\b');
            uart_putc(' ');
            uart_putc('\b');
        } else {
            uart_putc(c); // Echo character back
        }
    }
}