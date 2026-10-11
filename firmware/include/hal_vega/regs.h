#ifndef HAL_VEGA_REGS_H
#define HAL_VEGA_REGS_H

/*
 * VEGA ET1031 / THEJAS32 Peripheral Register Map
 * 
 * NOTE: These addresses are from the Thejas32rs EMULATOR.
 * Real hardware addresses MUST be verified against C-DAC SDK headers.
 * Do not hardcode addresses anywhere else - include this file only.
 */

#ifndef __ASSEMBLER__
#include <stdint.h>
#endif

/* Memory Map */
#define FLASH_BASE        0x40000000  /* QSPI XIP window, 16 MB */
#define SRAM_BASE         0x80000000  /* SRAM, 256 KB (real) / 512 KB (emu) */

#define CLINT_BASE        0x02000000
#define PLIC_BASE         0x0C000000

#define UART0_BASE        0x10000000  /* UART0: Telemetry */
#define UART1_BASE        0x10001000  /* UART1: GNSS */
#define UART2_BASE        0x10002000  /* UART2: RC */
#define UART_SIZE         0x1000
#define GPIO_BASE         0x10010000
#define TIMER_BASE        0x10020000
#define SPI_BASE          0x10030000
#define I2S_BASE          0x10040000
#define I2C_BASE          0x10050000
#define DMA_BASE          0x10060000
#define WDT_BASE          0x10070000
#define ADC_BASE          0x10080000
#define PWM_BASE          0x10090000

/* CLINT Registers */
#define CLINT_MSIP        (CLINT_BASE + 0x0000)
#define CLINT_MTIMECMP    (CLINT_BASE + 0x4000)
#define CLINT_MTIMECMPH   (CLINT_BASE + 0x4004)
#define CLINT_MTIME       (CLINT_BASE + 0xBFF8)
#define CLINT_MTIMEH      (CLINT_BASE + 0xBFFC)

/* PLIC Registers */
#define PLIC_PRIORITY(n)  (PLIC_BASE + 0x000004 + (n) * 4)
#define PLIC_PENDING      (PLIC_BASE + 0x001000)
#define PLIC_ENABLED      (PLIC_BASE + 0x002000)
#define PLIC_THRESHOLD    (PLIC_BASE + 0x200000)
#define PLIC_CLAIM        (PLIC_BASE + 0x200004)

/* UART Registers (16550-style) - per UART instance */
#define UART_THR(base)    ((base) + 0x00)
#define UART_RHR(base)    ((base) + 0x00)
#define UART_LSR(base)    ((base) + 0x05)

/* Convenience macros for each UART */
#define UART0_THR         UART_THR(UART0_BASE)
#define UART0_RHR         UART_RHR(UART0_BASE)
#define UART0_LSR         UART_LSR(UART0_BASE)
#define UART1_THR         UART_THR(UART1_BASE)
#define UART1_RHR         UART_RHR(UART1_BASE)
#define UART1_LSR         UART_LSR(UART1_BASE)
#define UART2_THR         UART_THR(UART2_BASE)
#define UART2_RHR         UART_RHR(UART2_BASE)
#define UART2_LSR         UART_LSR(UART2_BASE)

/* GPIO Registers */
#define GPIO_DIR          (GPIO_BASE + 0x00)
#define GPIO_DATA_OUT     (GPIO_BASE + 0x04)
#define GPIO_SET          (GPIO_BASE + 0x08)
#define GPIO_CLEAR        (GPIO_BASE + 0x0C)
#define GPIO_INT_EN       (GPIO_BASE + 0x10)

/* Timer Registers */
#define TIMER_CTRL        (TIMER_BASE + 0x00)
#define TIMER_COUNT       (TIMER_BASE + 0x04)
#define TIMER_RELOAD      (TIMER_BASE + 0x08)
#define TIMER_PRESC       (TIMER_BASE + 0x0C)
#define TIMER_STATUS      (TIMER_BASE + 0x10)

/* SPI Registers */
#define SPI_CTRL          (SPI_BASE + 0x00)
#define SPI_DATA          (SPI_BASE + 0x08)
#define SPI_CS            (SPI_BASE + 0x0C)

/* I2C Registers */
#define I2C_CR            (I2C_BASE + 0x00)
#define I2C_SR            (I2C_BASE + 0x04)
#define I2C_ADDR          (I2C_BASE + 0x08)
#define I2C_DATA          (I2C_BASE + 0x0C)

/* ADC Registers */
#define ADC_CTRL          (ADC_BASE + 0x00)
#define ADC_STATUS        (ADC_BASE + 0x04)
#define ADC_DATA          (ADC_BASE + 0x08)
#define ADC_CHAN          (ADC_BASE + 0x0C)

/* PWM Registers */
#define PWM_CTRL          (PWM_BASE + 0x00)
#define PWM_FREQ          (PWM_BASE + 0x04)
#define PWM_DUTY0         (PWM_BASE + 0x10)
#define PWM_DUTY(ch)      (PWM_DUTY0 + (ch) * 4)
#define PWM_DEAD          (PWM_BASE + 0x30)
#define PWM_PHASE0        (PWM_BASE + 0x34)
#define PWM_PHASE(ch)     (PWM_PHASE0 + (ch) * 4)

/* DMA Registers */
#define DMA_SRC           (DMA_BASE + 0x00)
#define DMA_DST           (DMA_BASE + 0x04)
#define DMA_LEN           (DMA_BASE + 0x08)
#define DMA_CTRL          (DMA_BASE + 0x0C)
#define DMA_STATUS        (DMA_BASE + 0x10)

/* WDT Registers */
#define WDT_CTRL          (WDT_BASE + 0x00)
#define WDT_COUNT         (WDT_BASE + 0x04)
#define WDT_RELOAD        (WDT_BASE + 0x08)
#define WDT_FEED          (WDT_BASE + 0x0C)
#define WDT_STATUS        (WDT_BASE + 0x10)

/* Bit definitions */
#define UART_LSR_TX_READY 0x60
#define UART_LSR_RX_READY 0x01

#define TIMER_CTRL_EN     0x01
#define TIMER_CTRL_IRQ    0x02
#define TIMER_CTRL_AUTO   0x04
#define TIMER_STATUS_MATCH 0x01

#define SPI_CTRL_EN       0x01
#define SPI_CTRL_LOOPBACK 0x02

#define I2C_CR_START      0x04
#define I2C_CR_READ       0x08
#define I2C_SR_ACK        0x04
#define I2C_SR_RX_READY   0x02

#define ADC_CTRL_EN       0x01
#define ADC_CTRL_CONT     0x02
#define ADC_STATUS_READY  0x01

#define PWM_CTRL_EN       0x01
#define PWM_CTRL_CH_EN(ch) (0x02 << (ch))

#define WDT_CTRL_EN       0x01
#define WDT_CTRL_RESET    0x02
#define WDT_FEED_KEY1     0x5555AAAA
#define WDT_FEED_KEY2     0x76

#endif /* HAL_VEGA_REGS_H */