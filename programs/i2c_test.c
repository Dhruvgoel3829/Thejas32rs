#define UART_THR  (*((volatile unsigned char *)0x10000000))
#define I2C_CR    (*((volatile unsigned int *)0x10050000))
#define I2C_SR    (*((volatile unsigned int *)0x10050004))
#define I2C_ADDR  (*((volatile unsigned int *)0x10050008))
#define I2C_DATA  (*((volatile unsigned int *)0x1005000C))

void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') UART_THR = '\r';
        UART_THR = *s++;
    }
}

void print_hex(unsigned int val) {
    const char hex[] = "0123456789ABCDEF";
    uart_puts("0x");
    for (int i = 1; i >= 0; i--) {
        UART_THR = hex[(val >> (i * 4)) & 0xF];
    }
}

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("   THEJAS32 I2C EEPROM Test           \n");
    uart_puts("=======================================\n");

    // 1. Write byte 0x42 to EEPROM address 0x10 (Slave address 0x50)
    uart_puts("[I2C] Writing 0x42 to EEPROM register 0x10...\n");
    I2C_ADDR = 0x50;       // Set target slave
    I2C_CR = 0x03;         // Enable + START
    I2C_DATA = 0x10;       // Send memory address 0x10
    I2C_DATA = 0x42;       // Send data byte
    I2C_CR = 0x04;         // STOP

    // 2. Set memory pointer for read
    uart_puts("[I2C] Reading back from EEPROM register 0x10...\n");
    I2C_CR = 0x03;         // START
    I2C_DATA = 0x10;       // Target address

    // 3. Trigger read
    I2C_CR = 0x09;         // Enable + READ mode
    while ((I2C_SR & 0x02) == 0); // Wait for RX Ready
    unsigned int read_val = I2C_DATA;

    uart_puts("[I2C] Read back value: ");
    print_hex(read_val);
    uart_puts("\n");

    if (read_val == 0x42) {
        uart_puts(">>> I2C EEPROM TEST PASSED! <<<\n");
    } else {
        uart_puts(">>> I2C EEPROM TEST FAILED! <<<\n");
    }

    while (1);
}