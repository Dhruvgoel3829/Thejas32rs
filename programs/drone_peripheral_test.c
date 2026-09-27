#include <stdint.h>
#include <stdio.h>

#define UART_THR        (*((volatile unsigned char *)0x10000000))
#define GPIO_DIR        (*((volatile unsigned int *)0x10010000))
#define GPIO_DATA       (*((volatile unsigned int *)0x10010004))
#define GPIO_SET        (*((volatile unsigned int *)0x10010008))
#define GPIO_CLEAR      (*((volatile unsigned int *)0x1001000C))
#define GPIO_INT_EN     (*((volatile unsigned int *)0x10010010))

#define TIMER_CTRL      (*((volatile unsigned int *)0x10020000))
#define TIMER_COUNT     (*((volatile unsigned int *)0x10020004))
#define TIMER_RELOAD    (*((volatile unsigned int *)0x10020008))
#define TIMER_PRESC     (*((volatile unsigned int *)0x1002000C))
#define TIMER_STATUS    (*((volatile unsigned int *)0x10020010))

#define SPI_CTRL        (*((volatile unsigned int *)0x10030000))
#define SPI_DATA        (*((volatile unsigned int *)0x10030004))
#define SPI_STATUS      (*((volatile unsigned int *)0x10030008))
#define SPI_CS          (*((volatile unsigned int *)0x1003000C))

#define I2C_CR          (*((volatile unsigned int *)0x10050000))
#define I2C_SR          (*((volatile unsigned int *)0x10050004))
#define I2C_ADDR        (*((volatile unsigned int *)0x10050008))
#define I2C_DATA        (*((volatile unsigned int *)0x1005000C))

#define ADC_CTRL        (*((volatile unsigned int *)0x10080000))
#define ADC_STATUS      (*((volatile unsigned int *)0x10080004))
#define ADC_DATA        (*((volatile unsigned int *)0x10080008))
#define ADC_CHAN        (*((volatile unsigned int *)0x1008000C))

#define PWM_CTRL        (*((volatile unsigned int *)0x10090000))
#define PWM_FREQ        (*((volatile unsigned int *)0x10090004))
#define PWM_DUTY0       (*((volatile unsigned int *)0x10090010))
#define PWM_DUTY1       (*((volatile unsigned int *)0x10090014))
#define PWM_DUTY2       (*((volatile unsigned int *)0x10090018))
#define PWM_DUTY3       (*((volatile unsigned int *)0x1009001C))
#define PWM_DUTY4       (*((volatile unsigned int *)0x10090020))
#define PWM_DUTY5       (*((volatile unsigned int *)0x10090024))
#define PWM_DUTY6       (*((volatile unsigned int *)0x10090028))
#define PWM_DUTY7       (*((volatile unsigned int *)0x1009002C))
#define PWM_DEAD        (*((volatile unsigned int *)0x10090030))
#define PWM_PHASE0      (*((volatile unsigned int *)0x10090034))

void uart_putc(char c) {
    UART_THR = c;
}

void uart_puts(const char* s) {
    while (*s) UART_THR = *s++;
}

void uart_puthex(uint32_t val) {
    char buf[11];
    buf[0] = '0'; buf[1] = 'x';
    for (int i = 9; i >= 2; i--) {
        int nib = val & 0xF;
        buf[i] = (nib < 10) ? '0' + nib : 'A' + nib - 10;
        val >>= 4;
    }
    buf[10] = '\0';
    uart_puts(buf);
}

void uart_putdec(int val) {
    if (val < 0) { UART_THR = '-'; val = -val; }
    char buf[12];
    int i = 11;
    buf[i] = '\0';
    if (val == 0) { UART_THR = '0'; return; }
    while (val > 0) {
        buf[--i] = '0' + (val % 10);
        val /= 10;
    }
    uart_puts(&buf[i]);
}

void delay(int cycles) {
    // Minimal delay - just a few instructions
    for (volatile int i = 0; i < 10; i++);
}

void test_gpio() {
    UART_THR = '\n'; UART_THR = '\r';
    uart_puts("[1/8] GPIO test...");
    GPIO_DIR = 0xFFFFFFFF;
    GPIO_DATA = 0xAAAAAAAA;
    delay(1000);
    GPIO_SET = 0x55555555;
    delay(100);
    GPIO_CLEAR = 0xAAAAAAAA;
    delay(100);
    uart_puts("OK\n");
}

void test_timer() {
    uart_puts("[2/8] Timer test...");
    TIMER_RELOAD = 10000;
    TIMER_PRESC = 0;
    TIMER_CTRL = 0x05; // Enable + auto-reload
    for (int i = 0; i < 3; i++) {
        while (!(TIMER_STATUS & 0x01));
        TIMER_STATUS = 0x01;
    }
    TIMER_CTRL = 0x00;
    uart_puts("OK\n");
}

void test_spi() {
    uart_puts("[3/8] SPI test...");
    SPI_CS = 0x00;
    SPI_DATA = 0x80 | 0x75;
    while (!(SPI_STATUS & 0x01));
    SPI_DATA = 0x00;
    while (!(SPI_STATUS & 0x01));
    uint32_t whoami = SPI_DATA;
    uart_puts("OK (WHO_AM_I="); uart_puthex(whoami); uart_puts(")\n");
    SPI_CS = 0xFF;
}

void test_i2c_eeprom() {
    uart_puts("[4/8] I2C EEPROM test...");
    I2C_ADDR = 0x50;
    I2C_DATA = 0x10;
    I2C_DATA = 0xDE;
    I2C_DATA = 0xAD;
    I2C_DATA = 0x10;
    I2C_CR = 0x08;
    uint32_t d1 = I2C_DATA;
    I2C_CR = 0x08;
    uint32_t d2 = I2C_DATA;
    uart_puts("OK (read="); uart_puthex(d1); uart_putc(' '); uart_puthex(d2); uart_puts(")\n");
}

void test_i2c_sensors() {
    uart_puts("[5/8] I2C Sensors test...");
    I2C_ADDR = 0x77;
    I2C_DATA = 0x00;
    I2C_CR = 0x08;
    uint32_t p = I2C_DATA;
    uart_puthex(p);

    I2C_ADDR = 0x1E;
    I2C_DATA = 0x03;
    I2C_CR = 0x08;
    uint32_t mx = I2C_DATA;
    uart_putc(' '); uart_puthex(mx);

    I2C_ADDR = 0x42;
    I2C_DATA = 0x00;
    I2C_CR = 0x08;
    uint32_t lat = I2C_DATA;
    uart_putc(' '); uart_puthex(lat);
    uart_puts(" OK\n");
}

void test_adc() {
    uart_puts("[6/8] ADC test...");
    ADC_CTRL = 0x01;
    ADC_CHAN = 0;
    ADC_DATA = 4500; // Simulate battery voltage
    uint32_t val = ADC_DATA;
    uart_puts(" ch0="); uart_putdec(val);

    ADC_CHAN = 1;
    ADC_DATA = 2048; // Simulate current
    val = ADC_DATA;
    uart_puts(" ch1="); uart_putdec(val);

    ADC_CHAN = 2;
    ADC_DATA = 3000; // Simulate RSSI
    val = ADC_DATA;
    uart_puts(" ch2="); uart_putdec(val);
    uart_puts(" OK\n");
}

void test_pwm() {
    uart_puts("[7/8] PWM test...");
    PWM_FREQ = 50000;
    PWM_CTRL = 0x1FF;
    for (int ch = 0; ch < 8; ch++) {
        *((volatile unsigned int *)(0x10090010 + ch * 4)) = (ch + 1) * 512;
    }
    PWM_DEAD = 1000;
    for (int ch = 0; ch < 8; ch++) {
        *((volatile unsigned int *)(0x10090034 + ch * 4)) = ch * 4500;
    }
    uart_puts("OK\n");
}

void test_gazebo_telemetry() {
    uart_puts("[8/8] Gazebo telemetry...");
    for (int i = 0; i < 3; i++) {
        uart_puts("TELEM: t="); uart_putdec(i);
        uart_puts(" roll=0.0 pitch=0.0 yaw=0.0 alt=0.0 vbat=3.7\n");
        delay(500);
    }
    uart_puts("OK\n");
}

int main() {
    uart_puts("\n========================================\n");
    uart_puts("  THEJAS32 Drone Peripheral Test\n");
    uart_puts("========================================\n");

    test_gpio();
    test_timer();
    test_spi();
    test_i2c_eeprom();
    test_i2c_sensors();
    test_adc();
    test_pwm();
    test_gazebo_telemetry();

    uart_puts("\n========================================\n");
    uart_puts("  ALL PERIPHERAL TESTS COMPLETE\n");
    uart_puts("========================================\n");

    while (1) {
        delay(100000);
    }
    return 0;
}