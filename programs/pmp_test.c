#define UART_THR (*((volatile unsigned char *)0x10000000))
#define UART_LSR (*((volatile unsigned char *)0x10000005))

static volatile int trap_fired = 0;
static volatile unsigned int protected_buffer = 0x12345678;

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

void pmp_trap_handler(void) {
    unsigned int mcause, mepc;
    __asm__ volatile("csrr %0, mcause" : "=r"(mcause));
    __asm__ volatile("csrr %0, mepc"   : "=r"(mepc));

    if (mcause == 7) { // Store Access Fault
        uart_puts("  [TRAP] Caught PMP Store Access Fault (Cause 7) as expected!\n");
        trap_fired = 1;

        // Determine if faulting instruction was 16-bit (RV32C) or 32-bit (RV32I)
        unsigned short insn_head = *((volatile unsigned short *)mepc);
        unsigned int insn_len = ((insn_head & 0x3) == 0x3) ? 4 : 2;

        __asm__ volatile("csrw mepc, %0" : : "r"(mepc + insn_len));
    }
}

__attribute__((section(".text.entry")))
void _start(void) {
    __asm__ volatile("li sp, 0x80070000");

    uart_puts("\r\n=======================================\n");
    uart_puts("    THEJAS32 PMP Unit Test Suite       \n");
    uart_puts("=======================================\n");

    // 1. Setup trap vector
    extern void pmp_trap_entry(void);
    __asm__ volatile("csrw mtvec, %0" : : "r"(pmp_trap_entry));

    // 2. Configure PMP Entry 0:
    // Protect protected_buffer as Locked Read-Only (L=1, A=NA4, R=1, W=0, X=0)
    // pmp0cfg byte: 0x80 (L) | 0x10 (NA4) | 0x01 (R) = 0x91
    unsigned int pmp_addr0 = ((unsigned int)&protected_buffer) >> 2;
    __asm__ volatile("csrw pmpaddr0, %0" : : "r"(pmp_addr0));
    __asm__ volatile("csrw pmpcfg0, %0"  : : "r"(0x91));

    // 3. Test Read Access
    uart_puts("[TEST 1] Reading from protected address...\n");
    unsigned int val = protected_buffer;
    if (val == 0x12345678) {
        uart_puts("  [PASS] Read succeeded.\n");
    }

    // 4. Test Write Access to Locked Read-Only region
    uart_puts("[TEST 2] Attempting write to Locked Read-Only PMP region...\n");
    trap_fired = 0;
    protected_buffer = 0xDEADBEEF;

    // Memory barrier to force reloading trap_fired from memory
    __asm__ volatile("" : : : "memory");

    if (trap_fired == 1) {
        uart_puts("  [PASS] PMP access restriction enforced successfully!\n");
        uart_puts("\n>>> ALL PMP TESTS PASSED <<<\n");
    } else {
        uart_puts("  [FAIL] Write succeeded without triggering PMP trap!\n");
    }

    __asm__ volatile("jr zero");
}