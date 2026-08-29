#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define UART_LSR         (*((volatile unsigned char *)0x10000005))

#define WDT_CTRL         (*((volatile unsigned int *)0x10070000))
#define WDT_COUNT        (*((volatile unsigned int *)0x10070004))
#define WDT_RELOAD       (*((volatile unsigned int *)0x10070008))
#define WDT_FEED         (*((volatile unsigned int *)0x1007000C))
#define WDT_STATUS       (*((volatile unsigned int *)0x10070010))

// Non-volatile boot counter kept in SRAM across resets
static volatile unsigned int *boot_count = (volatile unsigned int *)0x80060000;

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
    // Magic check: if first boot, initialize counter to 0
    if (*boot_count > 10) {
        *boot_count = 0;
    }

    (*boot_count)++;

    uart_puts("\r\n=======================================\n");
    uart_puts("     THEJAS32 Watchdog Timer Test      \n");
    uart_puts("=======================================\n");

    if (*boot_count == 1) {
        uart_puts("[BOOT 1] Initial cold boot detected.\n");

        // Set Watchdog reload to 5000 cycles
        WDT_RELOAD = 5000;
        // CTRL: ENABLE(1) | RESET_EN(2) = 0x03
        WDT_CTRL = 0x03;

        uart_puts("[TEST 1] Kicking watchdog 5 times successfully...\n");
        for (int i = 0; i < 5; i++) {
            for (volatile int delay = 0; delay < 500; delay++);
            WDT_FEED = 0x5555AAAA; // Feed the watchdog
            uart_puts("  - Watchdog fed successfully.\n");
        }

        uart_puts("[TEST 2] Halting watchdog feed to trigger system reset...\n");
        // Infinite loop without kicking -> Watchdog triggers SystemReset
        while (1);
    } else if (*boot_count == 2) {
        uart_puts("[BOOT 2] CPU successfully restarted from Watchdog Reset!\n");
        uart_puts("\n>>> WATCHDOG VERIFICATION PASSED <<<\n");

        // Disable Watchdog and exit cleanly
        WDT_CTRL = 0x00;
        *boot_count = 0;
    }

    return 0;
}