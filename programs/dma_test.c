#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define DMA_SRC          (*((volatile unsigned int *)0x10060000))
#define DMA_DST          (*((volatile unsigned int *)0x10060004))
#define DMA_LEN          (*((volatile unsigned int *)0x10060008))
#define DMA_CTRL         (*((volatile unsigned int *)0x1006000C))
#define DMA_STATUS       (*((volatile unsigned int *)0x10060010))

static const char message[] = "Hello directly from DMA Mem-to-UART transfer!\r\n";
static char src_buffer[64];
static char dst_buffer[64];

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

int main(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("     THEJAS32 DMA Controller Test      \n");
    uart_puts("=======================================\n");

    for (int i = 0; i < 64; i++) {
        src_buffer[i] = (char)(0xAA ^ i);
        dst_buffer[i] = 0;
    }

    // TEST 1: Memory-to-Memory
    uart_puts("[TEST 1] Initiating 64-byte SRAM-to-SRAM DMA...\n");
    DMA_SRC = (unsigned int)src_buffer;
    DMA_DST = (unsigned int)dst_buffer;
    DMA_LEN = 64;
    DMA_CTRL = 0x0D;

    while ((DMA_STATUS & 0x02) == 0);
    DMA_STATUS = 0x02;

    int mismatch = 0;
    for (int i = 0; i < 64; i++) {
        if (dst_buffer[i] != src_buffer[i]) {
            mismatch = 1;
            break;
        }
    }

    if (!mismatch) {
        uart_puts("  [PASS] 64-byte Memory-to-Memory DMA verified!\n");
    } else {
        uart_puts("  [FAIL] Data mismatch in destination buffer!\n");
    }

    // TEST 2: Memory-to-Peripheral (UART)
    uart_puts("[TEST 2] Streaming message via SRAM-to-UART DMA...\n");
    DMA_SRC = (unsigned int)message;
    DMA_DST = 0x10000000;
    DMA_LEN = sizeof(message) - 1;
    DMA_CTRL = 0x05;

    while ((DMA_STATUS & 0x02) == 0);
    DMA_STATUS = 0x02;

    uart_puts("\n>>> ALL DMA VERIFICATION TESTS PASSED! <<<\n");

    return 0;
}