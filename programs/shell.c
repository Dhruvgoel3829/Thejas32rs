#define UART_RHR       (*((volatile unsigned char *)0x10000000))
#define UART_THR       (*((volatile unsigned char *)0x10000000))
#define UART_LSR       (*((volatile unsigned char *)0x10000005))
#define CLINT_MTIME    (*((volatile unsigned long long *)0x0200BFF8))

// --- UART Helpers ---
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

char uart_getc(void) {
    while ((UART_LSR & 0x01) == 0);
    return UART_RHR;
}

// --- String & Parsing Utilities ---
int strcmp(const char *a, const char *b) {
    while (*a && (*a == *b)) { a++; b++; }
    return *(const unsigned char *)a - *(const unsigned char *)b;
}

int strncmp(const char *a, const char *b, int n) {
    while (n && *a && (*a == *b)) { a++; b++; n--; }
    if (n == 0) return 0;
    return *(const unsigned char *)a - *(const unsigned char *)b;
}

void print_hex32(unsigned int val) {
    const char hex_chars[] = "0123456789ABCDEF";
    uart_puts("0x");
    for (int i = 7; i >= 0; i--) {
        uart_putc(hex_chars[(val >> (i * 4)) & 0xF]);
    }
}

unsigned int parse_hex(const char *s) {
    unsigned int val = 0;
    if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) s += 2;
    while (*s) {
        char c = *s++;
        val <<= 4;
        if (c >= '0' && c <= '9') val |= (c - '0');
        else if (c >= 'a' && c <= 'f') val |= (c - 'a' + 10);
        else if (c >= 'A' && c <= 'F') val |= (c - 'A' + 10);
        else break;
    }
    return val;
}

// --- Command Execution ---
void execute_command(char *cmd) {
    // Skip leading spaces
    while (*cmd == ' ') cmd++;
    if (*cmd == '\0') return;

    if (strcmp(cmd, "help") == 0) {
        uart_puts("\nAvailable commands:\n");
        uart_puts("  help                - Display this menu\n");
        uart_puts("  mtime               - Read CLINT hardware timer\n");
        uart_puts("  peek <addr>         - Read 32-bit value from address\n");
        uart_puts("  poke <addr> <val>   - Write 32-bit value to address\n");
        uart_puts("  dump <addr>         - Hexdump 64 bytes from address\n");
    } else if (strcmp(cmd, "mtime") == 0) {
        unsigned int high = (unsigned int)(CLINT_MTIME >> 32);
        unsigned int low  = (unsigned int)(CLINT_MTIME & 0xFFFFFFFF);
        uart_puts("\nmtime: ");
        print_hex32(high);
        uart_putc('_');
        print_hex32(low);
        uart_puts("\n");
    } else if (strncmp(cmd, "peek", 4) == 0) {
        char *arg = cmd + 4;
        while (*arg == ' ') arg++;
        unsigned int addr = parse_hex(arg);
        unsigned int val = *((volatile unsigned int *)addr);
        uart_puts("\n[");
        print_hex32(addr);
        uart_puts("] = ");
        print_hex32(val);
        uart_puts("\n");
    } else if (strncmp(cmd, "poke", 4) == 0) {
        char *arg = cmd + 4;
        while (*arg == ' ') arg++;
        unsigned int addr = parse_hex(arg);
        while (*arg && *arg != ' ') arg++;
        while (*arg == ' ') arg++;
        unsigned int val = parse_hex(arg);

        *((volatile unsigned int *)addr) = val;
        uart_puts("\nWrote ");
        print_hex32(val);
        uart_puts(" to [");
        print_hex32(addr);
        uart_puts("]\n");
    } else if (strncmp(cmd, "dump", 4) == 0) {
        char *arg = cmd + 4;
        while (*arg == ' ') arg++;
        unsigned int addr = parse_hex(arg) & ~0x3; // 4-byte align

        uart_puts("\n--- Hexdump ---\n");
        for (int row = 0; row < 4; row++) {
            print_hex32(addr + row * 16);
            uart_puts(": ");
            for (int col = 0; col < 4; col++) {
                unsigned int val = *((volatile unsigned int *)(addr + row * 16 + col * 4));
                print_hex32(val);
                uart_putc(' ');
            }
            uart_puts("\n");
        }
    } else {
        uart_puts("\nUnknown command: ");
        uart_puts(cmd);
        uart_puts("\nType 'help' for available commands.\n");
    }
}

// --- Main Entry ---
__attribute__((section(".text.entry")))
void _start(void) {
    char buf[64];
    int idx = 0;

    uart_puts("\r\n========================================\n");
    uart_puts("   THEJAS32 Embedded Monitor v0.1       \n");
    uart_puts("========================================\n");
    uart_puts("Type 'help' for command list.\n\nthejas32> ");

    while (1) {
        char c = uart_getc();

        if (c == '\r' || c == '\n') {
            buf[idx] = '\0';
            execute_command(buf);
            idx = 0;
            uart_puts("thejas32> ");
        } else if (c == 0x08 || c == 0x7F) { // Backspace / Del
            if (idx > 0) {
                idx--;
                uart_putc('\b');
                uart_putc(' ');
                uart_putc('\b');
            }
        } else if (c >= 32 && c < 127) { // Printable characters
            if (idx < 63) {
                buf[idx++] = c;
                uart_putc(c);
            }
        }
    }
}