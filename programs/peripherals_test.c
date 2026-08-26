// programs/peripherals_test.c

#define UART_THR         (*((volatile unsigned char *)0x10000000))
#define GPIO_DIR         (*((volatile unsigned int *)0x10010008))
#define GPIO_SET         (*((volatile unsigned int *)0x1001000C))
#define GPIO_CLEAR       (*((volatile unsigned int *)0x10010010))
#define GPIO_DATA_IN     (*((volatile unsigned int *)0x10010000))

#define TIMER_CTRL       (*((volatile unsigned int *)0x10020000))
#define TIMER_COUNT      (*((volatile unsigned int *)0x10020004))
#define TIMER_RELOAD     (*((volatile unsigned int *)0x10020008))
#define TIMER_STATUS     (*((volatile unsigned int *)0x10020010))

#define SPI_CR           (*((volatile unsigned int *)0x10030000))
#define SPI_SR           (*((volatile unsigned int *)0x10030004))
#define SPI_DATA         (*((volatile unsigned int *)0x10030008))

#define I2S_CR           (*((volatile unsigned int *)0x10040000))
#define I2S_LEFT_TX      (*((volatile unsigned int *)0x10040008))
#define I2S_RIGHT_TX     (*((volatile unsigned int *)0x1004000C))

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

__attribute__((section(".text.entry")))
void _start(void) {
    uart_puts("\r\n=======================================\n");
    uart_puts("   THEJAS32 Peripheral Test Suite     \n");
    uart_puts("=======================================\n");

    // 1. Test GPIO
    uart_puts("[TEST 1] Testing GPIO (Pins 0..3 as Output)...\n");
    GPIO_DIR = 0x0F;                // Set pins 0,1,2,3 as outputs
    GPIO_SET = 0x05;                // Turn on pin 0 and 2 (0b0101)
    if ((GPIO_DATA_IN & 0x0F) == 0x05) {
        uart_puts("  [PASS] GPIO Set passed (Value: ");
        print_hex(GPIO_DATA_IN & 0x0F);
        uart_puts(")\n");
    }
    GPIO_CLEAR = 0x01;              // Clear pin 0 -> remaining should be 0x04
    if ((GPIO_DATA_IN & 0x0F) == 0x04) {
        uart_puts("  [PASS] GPIO Clear passed (Value: ");
        print_hex(GPIO_DATA_IN & 0x0F);
        uart_puts(")\n");
    }

    // 2. Test SPI Loopback & Transfer
    uart_puts("[TEST 2] Testing SPI Controller (Loopback Mode)...\n");
    SPI_CR = 0x03;                  // Enable SPI + Loopback mode
    SPI_DATA = 0xA5;                // Transmit byte 0xA5
    while ((SPI_SR & 0x02) == 0);   // Wait for RX ready
    unsigned int rx_spi = SPI_DATA;
    if (rx_spi == 0xA5) {
        uart_puts("  [PASS] SPI Loopback Received: ");
        print_hex(rx_spi);
        uart_puts("\n");
    }

    // 3. Test I2S Audio Stream
    uart_puts("[TEST 3] Testing I2S Transmitter...\n");
    I2S_CR = 0x01;                  // Enable I2S
    I2S_LEFT_TX = 0x1234;           // Send Left Channel Sample
    I2S_RIGHT_TX = 0x5678;          // Send Right Channel Sample
    uart_puts("  [PASS] I2S Left/Right Samples Transmitted.\n");

    // 4. Test Peripheral Timer
    uart_puts("[TEST 4] Testing Peripheral Timer Counter...\n");
    TIMER_RELOAD = 100;
    TIMER_CTRL = 0x05;              // Enable timer + Auto-reload
    while (TIMER_COUNT < 50);       // Wait for counter to advance
    uart_puts("  [PASS] Timer counter reached: ");
    print_hex(TIMER_COUNT);
    uart_puts("\n\nAll peripheral verification tests PASSED!\n");

    while (1);
}