// programs/atomic_test.c

#define UART_THR (*((volatile unsigned char *)0x10000000))

static volatile unsigned int test_var = 100;
static volatile unsigned int lock = 0;

void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') UART_THR = '\r';
        UART_THR = *s++;
    }
}

void print_hex(unsigned int val) {
    const char hex[] = "0123456789ABCDEF";
    uart_puts("0x");
    for (int i = 7; i >= 0; i--) {
        UART_THR = hex[(val >> (i * 4)) & 0xF];
    }
}

// Atomic Spinlock Acquire using AMOSWAP
void spinlock_acquire(volatile unsigned int *lock_ptr) {
    unsigned int locked = 1;
    while (locked) {
        __asm__ volatile(
            "amoswap.w.aq %0, %1, (%2)"
            : "=r"(locked)
            : "r"(1), "r"(lock_ptr)
            : "memory"
        );
    }
}

// Atomic Spinlock Release using AMOSWAP
void spinlock_release(volatile unsigned int *lock_ptr) {
    __asm__ volatile(
        "amoswap.w.rl zero, zero, (%0)"
        :
        : "r"(lock_ptr)
        : "memory"
    );
}

// Atomic Fetch-and-Add using AMOADD
unsigned int atomic_fetch_add(volatile unsigned int *ptr, unsigned int val) {
    unsigned int old;
    __asm__ volatile(
        "amoadd.w %0, %1, (%2)"
        : "=r"(old)
        : "r"(val), "r"(ptr)
        : "memory"
    );
    return old;
}

// Atomic Compare-And-Swap using LR.W and SC.W
int atomic_cas(volatile unsigned int *ptr, unsigned int expected, unsigned int desired) {
    unsigned int actual;
    unsigned int failed;
    __asm__ volatile(
        "1: lr.w      %0, (%2)      \n"
        "   bne       %0, %3, 2f    \n"
        "   sc.w      %1, %4, (%2)  \n"
        "   bnez      %1, 1b        \n"
        "2:                         \n"
        : "=&r"(actual), "=&r"(failed)
        : "r"(ptr), "r"(expected), "r"(desired)
        : "memory"
    );
    return (actual == expected);
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("   THEJAS32 RV32A Atomic Test Suite   \n");
    uart_puts("=======================================\n");

    // 1. Test AMOADD
    uart_puts("[TEST 1] AMOADD.W: Adding 25 to 100...\n");
    unsigned int prev = atomic_fetch_add(&test_var, 25);
    if (prev == 100 && test_var == 125) {
        uart_puts("  [PASS] Old: 100, New Value: 125\n");
    }

    // 2. Test Spinlock (AMOSWAP)
    uart_puts("[TEST 2] AMOSWAP.W Spinlock Acquire/Release...\n");
    spinlock_acquire(&lock);
    test_var += 10;
    spinlock_release(&lock);
    if (test_var == 135 && lock == 0) {
        uart_puts("  [PASS] Spinlock protected critical section passed.\n");
    }

    // 3. Test LR.W / SC.W (CAS)
    uart_puts("[TEST 3] LR.W / SC.W Compare-and-Swap...\n");
    int cas_ok = atomic_cas(&test_var, 135, 200);
    if (cas_ok && test_var == 200) {
        uart_puts("  [PASS] CAS succeeded: 135 -> 200\n");
    }

    int cas_fail = atomic_cas(&test_var, 999, 500);
    if (!cas_fail && test_var == 200) {
        uart_puts("  [PASS] CAS correctly rejected mismatch.\n");
    }

    uart_puts("\nAll RV32A atomic tests PASSED!\n");
    while (1);
}