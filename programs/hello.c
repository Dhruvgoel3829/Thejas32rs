#define UART_THR (*((volatile unsigned char *)0x10000000))

void uart_puts(const char *s) {
    while (*s) {
        UART_THR = *s++;
    }
}

void _start() {
    uart_puts("Hello from bare-metal C on THEJAS32 Emulator!\n");
    while (1);
}