#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

// CLINT Timer Registers
#define CLINT_MTIME      (*((volatile unsigned long long *)0x0200BFF8))
#define CLINT_MTIMECMP   (*((volatile unsigned long long *)0x02004000))

#define TIMER_INTERVAL   50000ULL // Context switch every 50,000 cycles
#define STACK_SIZE       1024

// Task Control Block
typedef struct {
    unsigned int *sp;
    unsigned int stack[STACK_SIZE];
} TCB;

static TCB task1;
static TCB task2;
static int current_task_id = 0;
volatile unsigned int *current_sp = 0;

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

// Add to programs/scheduler_test.c

void *memset(void *s, int c, unsigned int n) {
    unsigned char *p = (unsigned char *)s;
    while (n--) {
        *p++ = (unsigned char)c;
    }
    return s;
}

void *memcpy(void *dest, const void *src, unsigned int n) {
    char *d = (char *)dest;
    const char *s = (const char *)src;
    while (n--) {
        *d++ = *s++;
    }
    return dest;
}

// -------------------------------------------------------------
// Tasks
// -------------------------------------------------------------

void task_alpha(void) {
    int count = 0;
    while (1) {
        uart_puts("[Task A] Executing iteration ");
        uart_putc('0' + (count % 10));
        uart_puts("\n");
        count++;

        // Burn CPU cycles between ticks
        for (volatile int i = 0; i < 80000; i++);
    }
}

void task_beta(void) {
    int count = 0;
    while (1) {
        uart_puts("   ---> [Task B] Preempted into Task B! Count: ");
        uart_putc('0' + (count % 10));
        uart_puts("\n");
        count++;

        for (volatile int i = 0; i < 80000; i++);
    }
}

// Initialize task stack with dummy register frame
void init_task_stack(TCB *tcb, void (*entry)(void)) {
    unsigned int *top = &tcb->stack[STACK_SIZE - 1];

    // Align to 16-byte boundary
    top = (unsigned int *)((unsigned int)top & ~0xF);

    // Reserve 128 bytes (32 words)
    top -= 32;

    for (int i = 0; i < 32; i++) {
        top[i] = 0;
    }

    // Offset 124 (index 31): mepc = task entry point
    top[31] = (unsigned int)entry;

    // Offset 0 (index 0): ra = task entry point (safety fallback)
    top[0]  = (unsigned int)entry;

    tcb->sp = top;
}

// -------------------------------------------------------------
// Kernel Init & Switcher
// -------------------------------------------------------------

void scheduler_init(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("    THEJAS32 Preemptive RTOS Scheduler \n");
    uart_puts("=======================================\n");

    // Initialize both tasks
    init_task_stack(&task1, task_alpha);
    init_task_stack(&task2, task_beta);

    current_task_id = 0;
    current_sp = task1.sp;

    // Schedule the first timer interrupt in CLINT
    CLINT_MTIMECMP = CLINT_MTIME + TIMER_INTERVAL;

    // Enable Machine Timer Interrupt (MTIE = bit 7)
    __asm__ volatile("csrs mie, %0" : : "r"(1 << 7));
}

// Called directly from trap_entry with the old SP, returns the new SP
unsigned int *scheduler_switch(unsigned int *old_sp) {
    // 1. Acknowledge and schedule next timer interrupt
    CLINT_MTIMECMP = CLINT_MTIME + TIMER_INTERVAL;

    // 2. Save current task SP
    if (current_task_id == 0) {
        task1.sp = old_sp;
        current_task_id = 1;
        return task2.sp;
    } else {
        task2.sp = old_sp;
        current_task_id = 0;
        return task1.sp;
    }
}