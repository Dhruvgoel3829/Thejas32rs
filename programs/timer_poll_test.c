#include <stdint.h>

#define UART_THR        (*((volatile unsigned char *)0x10000000))
#define TIMER_CTRL      (*((volatile unsigned int *)0x10020000))
#define TIMER_RELOAD    (*((volatile unsigned int *)0x10020008))
#define TIMER_PRESC     (*((volatile unsigned int *)0x1002000C))
#define TIMER_STATUS    (*((volatile unsigned int *)0x10020010))

void uart_putc(char c) { *((volatile unsigned char *)0x10000000) = c; }
void uart_puts(const char* s) { while (*s) *((volatile unsigned char *)0x10000000) = *s++; }

int main() {
    // Timer poll test with auto-reload
    TIMER_RELOAD = 1000;
    TIMER_PRESC = 0;
    TIMER_CTRL = 0x05; // Enable + auto-reload
    
    int count = 0;
    while (count < 5) {
        if (*((volatile unsigned int *)0x10020010) & 0x01) {
            // Timer fired
            TIMER_STATUS = 0x01; // Clear
            uart_puts("Timer fired\n");
            count++;
        }
    }
    
    while (1);
    return 0;
}