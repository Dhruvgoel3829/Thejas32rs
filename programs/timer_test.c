#define UART_THR            (*((volatile unsigned char *)0x10000000))
#define CLINT_MTIMECMP_LOW  (*((volatile unsigned int *)0x02004000))
#define CLINT_MTIMECMP_HIGH (*((volatile unsigned int *)0x02004004))
#define CLINT_MTIME_LOW     (*((volatile unsigned int *)0x0200BFF8))
#define CLINT_MTIME_HIGH    (*((volatile unsigned int *)0x0200BFFC))

static volatile int timer_ticks = 0;

void uart_puts(const char *s) {
    while (*s) {
        UART_THR = *s++;
    }
}

static inline unsigned long long get_mtime(void) {
    unsigned int high, low, high2;
    do {
        high = CLINT_MTIME_HIGH;
        low  = CLINT_MTIME_LOW;
        high2 = CLINT_MTIME_HIGH;
    } while (high != high2);
    return (((unsigned long long)high) << 32) | low;
}

static inline void set_mtimecmp(unsigned long long val) {
    // 1. Prevent trigger by setting low word to max
    CLINT_MTIMECMP_LOW = 0xFFFFFFFF;
    // 2. Set new high word
    CLINT_MTIMECMP_HIGH = (unsigned int)(val >> 32);
    // 3. Set new low word
    CLINT_MTIMECMP_LOW = (unsigned int)(val & 0xFFFFFFFF);
}

// Trap handler
__attribute__((interrupt("machine")))
void trap_handler(void) {
    unsigned int mcause;
    __asm__ volatile("csrr %0, mcause" : "=r"(mcause));

    if (mcause == 0x80000007) {
        timer_ticks++;
        uart_puts("[INTERRUPT] Timer Tick!\n");

        // Schedule next interrupt in 2000 cycles (more breathing room)
        set_mtimecmp(get_mtime() + 2000);
    }
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("Setting up Timer Interrupt...\n");

    // 1. Set mtvec
    __asm__ volatile("csrw mtvec, %0" :: "r"((unsigned int)trap_handler));

    // 2. Schedule first interrupt
    set_mtimecmp(get_mtime() + 2000);

    // 3. Enable MTIE (bit 7)
    unsigned int mie_mask = 0x80;
    __asm__ volatile("csrs mie, %0" :: "r"(mie_mask));

    // 4. Enable global interrupts MIE (bit 3)
    __asm__ volatile("csrsi mstatus, 0x08");

    uart_puts("Waiting for timer ticks...\n");

    while (timer_ticks < 3) {
        // Wait for handler to increment ticks
    }

    // Disable timer interrupt once test is complete
    __asm__ volatile("csrc mie, %0" :: "r"(mie_mask));

    uart_puts("Successfully handled 3 timer interrupts!\n");

    while (1) {
        __asm__ volatile("wfi");
    }
}