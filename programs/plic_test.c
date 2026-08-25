// programs/plic_test.c

#define UART_RHR            (*((volatile unsigned char *)0x10000000))
#define UART_THR            (*((volatile unsigned char *)0x10000000))
#define UART_LSR            (*((volatile unsigned char *)0x10000005))
#define PLIC_PRIORITY_1     (*((volatile unsigned int *)0x0C000004))
#define PLIC_ENABLE         (*((volatile unsigned int *)0x0C002000))
#define PLIC_THRESHOLD      (*((volatile unsigned int *)0x0C200000))
#define PLIC_CLAIM_COMPLETE (*((volatile unsigned int *)0x0C200004))

static volatile int plic_handled = 0;

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

__attribute__((interrupt("machine")))
void trap_handler(void) {
    unsigned int mcause;
    __asm__ volatile("csrr %0, mcause" : "=r"(mcause));

    if (mcause == 0x8000000B) { // Machine External Interrupt
        unsigned int irq = PLIC_CLAIM_COMPLETE;
        
        if (irq == 1) {
            // 1. Read byte to clear the UART hardware buffer!
            char c = UART_RHR;

            uart_puts("\r\n[PLIC] Received character: '");
            uart_putc(c);
            uart_puts("'\r\n");

            plic_handled = 1;
        }

        // 2. Signal completion to PLIC
        PLIC_CLAIM_COMPLETE = irq;
    }
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\nConfiguring PLIC Interrupts...\n");

    // 1. Set trap handler
    __asm__ volatile("csrw mtvec, %0" :: "r"((unsigned int)trap_handler));

    // 2. Configure PLIC: Source 1 Priority = 3, Threshold = 0, Enable Source 1
    PLIC_PRIORITY_1 = 3;
    PLIC_THRESHOLD = 0;
    PLIC_ENABLE = (1 << 1);

    // 3. Enable MEIE (bit 11 = 0x800) in mie
    unsigned int mie_mask = 0x800;
    __asm__ volatile("csrs mie, %0" :: "r"(mie_mask));

    // 4. Enable Global Interrupts (MIE, bit 3)
    __asm__ volatile("csrsi mstatus, 0x08");

    uart_puts("Press any key to trigger UART PLIC interrupt...\n");

    while (!plic_handled) {
        __asm__ volatile("wfi"); // Sleep until interrupt fires
    }

    uart_puts("Successfully handled UART PLIC interrupt!\n");
    while (1);
}