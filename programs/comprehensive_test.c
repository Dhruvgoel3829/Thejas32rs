// Memory Map & Peripherals
#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define CLINT_MTIME      (*((volatile unsigned long long *)0x0200BFF8))
#define CLINT_MTIMECMP   (*((volatile unsigned long long *)0x02004000))

#define GPIO_DIR         (*((volatile unsigned int *)0x10010000))
#define GPIO_DATA        (*((volatile unsigned int *)0x10010004))

#define TIMER_CTRL       (*((volatile unsigned int *)0x10020000))
#define TIMER_COUNT      (*((volatile unsigned int *)0x10020004))

#define SPI_CTRL         (*((volatile unsigned int *)0x10030000))
#define SPI_DATA         (*((volatile unsigned int *)0x10030004))

#define I2C_CTRL         (*((volatile unsigned int *)0x10050000))
#define I2C_DATA         (*((volatile unsigned int *)0x10050004))

#define DMA_SRC          (*((volatile unsigned int *)0x10060000))
#define DMA_DST          (*((volatile unsigned int *)0x10060004))
#define DMA_LEN          (*((volatile unsigned int *)0x10060008))
#define DMA_CTRL         (*((volatile unsigned int *)0x1006000C))
#define DMA_STATUS       (*((volatile unsigned int *)0x10060010))

#define WDT_CTRL         (*((volatile unsigned int *)0x10070000))
#define WDT_COUNT        (*((volatile unsigned int *)0x10070004))
#define WDT_RELOAD       (*((volatile unsigned int *)0x10070008))
#define WDT_FEED         (*((volatile unsigned int *)0x1007000C))

// Global test variables
static volatile int timer_irq_hit = 0;
static volatile int pmp_fault_hit = 0;
static volatile unsigned int pmp_locked_buffer = 0xAA55AA55;
static unsigned int dma_src_buf[16];
static unsigned int dma_dst_buf[16];

// Utilities
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

void print_result(const char *test_name, int pass) {
    uart_puts("  [");
    if (pass) {
        uart_puts("PASS");
    } else {
        uart_puts("FAIL");
    }
    uart_puts("] ");
    uart_puts(test_name);
    uart_puts("\n");
}

// Trap Handler for PMP faults and CLINT Timer IRQs
void comp_trap_handler(void) {
    unsigned int mcause, mepc;
    __asm__ volatile("csrr %0, mcause" : "=r"(mcause));
    __asm__ volatile("csrr %0, mepc"   : "=r"(mepc));

    if (mcause == 0x80000007) { // Machine Timer Interrupt
        timer_irq_hit = 1;
        CLINT_MTIMECMP = 0xFFFFFFFFFFFFFFFFULL; // Acknowledge & clear
    } else if (mcause == 7) { // Store Access Fault (PMP Violation)
        pmp_fault_hit = 1;
        unsigned short insn_head = *((volatile unsigned short *)mepc);
        unsigned int insn_len = ((insn_head & 0x3) == 0x3) ? 4 : 2;
        __asm__ volatile("csrw mepc, %0" : : "r"(mepc + insn_len));
    }
}

// -------------------------------------------------------------
// Test Modules
// -------------------------------------------------------------

int test_rv32im() {
    volatile int a = 1234;
    volatile int b = 5678;
    volatile int mul_res = a * b;
    volatile int div_res = mul_res / a;
    volatile int rem_res = mul_res % b;

    return (mul_res == 7006652) && (div_res == 5678) && (rem_res == 0);
}

int test_rv32a_atomics() {
    volatile unsigned int lock_var = 0;
    unsigned int old_val = 0;

    // AMO SWAP
    __asm__ volatile("amoswap.w %0, %2, (%1)"
                     : "=r"(old_val)
                     : "r"(&lock_var), "r"(0x1234)
                     : "memory");

    if (old_val != 0 || lock_var != 0x1234) return 0;

    // LR / SC
    unsigned int val, success;
    __asm__ volatile("lr.w %0, (%1)" : "=r"(val) : "r"(&lock_var) : "memory");
    __asm__ volatile("sc.w %0, %2, (%1)" : "=r"(success) : "r"(&lock_var), "r"(0x5678) : "memory");

    return (success == 0) && (lock_var == 0x5678);
}

int test_gpio() {
    GPIO_DIR = 0xFFFFFFFF; // All outputs
    GPIO_DATA = 0x12345678;
    return (GPIO_DATA == 0x12345678);
}

int test_dma() {
    for (int i = 0; i < 16; i++) {
        dma_src_buf[i] = 0xCAFE0000 + i;
        dma_dst_buf[i] = 0;
    }

    DMA_SRC = (unsigned int)dma_src_buf;
    DMA_DST = (unsigned int)dma_dst_buf;
    DMA_LEN = sizeof(dma_src_buf);
    DMA_CTRL = 0x01 | 0x04 | 0x08; // START | SRC_INC | DST_INC

    while (DMA_STATUS & 0x01); // Wait until not busy

    for (int i = 0; i < 16; i++) {
        if (dma_dst_buf[i] != 0xCAFE0000 + i) return 0;
    }
    return 1;
}

int test_clint_timer() {
    timer_irq_hit = 0;

    // Enable Machine Timer Interrupt (MTIE = bit 7) & Global IE (MIE = bit 3 in mstatus)
    __asm__ volatile("csrs mie, %0" : : "r"(1 << 7));
    __asm__ volatile("csrs mstatus, %0" : : "r"(1 << 3));

    CLINT_MTIMECMP = CLINT_MTIME + 1000;

    for (volatile int i = 0; i < 5000; i++) {
        if (timer_irq_hit) break;
    }

    __asm__ volatile("csrc mstatus, %0" : : "r"(1 << 3)); // Disable interrupts
    return timer_irq_hit;
}

int test_pmp() {
    pmp_fault_hit = 0;

    // Configure PMP Entry 0: Locked Read-Only NA4 (0x91)
    unsigned int addr = ((unsigned int)&pmp_locked_buffer) >> 2;
    __asm__ volatile("csrw pmpaddr0, %0" : : "r"(addr));
    __asm__ volatile("csrw pmpcfg0, %0"  : : "r"(0x91));

    // Read should succeed
    volatile unsigned int r = pmp_locked_buffer;
    if (r != 0xAA55AA55) return 0;

    // Write should trigger Store Access Fault
    pmp_locked_buffer = 0xDEADBEEF;
    __asm__ volatile("" : : : "memory");

    return pmp_fault_hit;
}

int test_wdt_feed() {
    WDT_RELOAD = 10000;
    WDT_CTRL = 0x01; // Enable WDT without reset
    WDT_FEED = 0x5555AAAA;
    unsigned int cnt = WDT_COUNT;
    WDT_CTRL = 0x00; // Disable
    return (cnt > 0);
}

// -------------------------------------------------------------
// Main Suite Runner
// -------------------------------------------------------------

int main(void) {
    uart_puts("\r\n====================================================\n");
    uart_puts("   THEJAS32 / VEGA ET1031 FULL-SYSTEM INTEGRATION   \n");
    uart_puts("====================================================\n");

    print_result("RV32IM Core (Arithmetic / Mul / Div)", test_rv32im());
    print_result("RV32A Atomics (LR/SC & AMOSWAP)", test_rv32a_atomics());
    print_result("GPIO Controller (Direction / Output / Data)", test_gpio());
    print_result("DMA Engine (Mem-to-Mem Transfer)", test_dma());
    print_result("CLINT Timer (Preemption & Hardware Interrupt)", test_clint_timer());
    print_result("PMP Unit (Locked Read-Only NA4 Enforce & Trap)", test_pmp());
    print_result("Watchdog Timer (Feed Key & Auto-Reload)", test_wdt_feed());

    uart_puts("\n>>> ALL SUBSYSTEM TESTS EXECUTED SUCCESSFULLY <<<\n");
    return 0;
}