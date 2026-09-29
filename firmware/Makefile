# VEGA Pilot Firmware Makefile
# Builds for RV32 (target) and x86-64 (host tests)

# Toolchains
RV_CROSS_COMPILE ?= riscv-none-elf-
HOST_CC ?= gcc

RV_CC      := $(RV_CROSS_COMPILE)gcc
RV_OBJDUMP := $(RV_CROSS_COMPILE)objdump
RV_OBJCOPY := $(RV_CROSS_COMPILE)objcopy
RV_SIZE    := $(RV_CROSS_COMPILE)size

HOST_CC   := $(HOST_CC)
HOST_SIZE := size

# Architecture flags
RV_CFLAGS  := -march=rv32imac_zicsr -mabi=ilp32
HOST_CFLAGS := -m64

# Common flags
COMMON_CFLAGS := -std=c11 -Wall -Wextra -Werror \
                 -ffunction-sections -fdata-sections \
                 -Os -g \
                 -Iinclude \
                 -DSRAM_BUDGET_BYTES=262144

RV_CFLAGS  += $(COMMON_CFLAGS) -ffreestanding -nostdlib -fno-builtin
HOST_CFLAGS += $(COMMON_CFLAGS)

# Directories
BUILD_DIR := build
RV_BUILD_DIR  := $(BUILD_DIR)/rv32
HOST_BUILD_DIR := $(BUILD_DIR)/host

# Linker flags
LIBGCC := /opt/riscv/bin/../lib/gcc/riscv-none-elf/15.2.0/libgcc.a
RV_LDFLAGS  := -Wl,--gc-sections -Wl,-Map=$(RV_BUILD_DIR)/firmware.map -T link.ld
HOST_LDFLAGS := -Wl,--gc-sections

SRC_DIR := src
INC_DIR := include

# Source files (will expand as we add modules)
RV_SRCS := $(SRC_DIR)/main.c \
           $(SRC_DIR)/sched.c \
           $(SRC_DIR)/hal_emu/time.c \
           $(SRC_DIR)/hal_emu/uart.c \
           $(SRC_DIR)/hal_emu/gpio.c \
           $(SRC_DIR)/hal_emu/spi.c \
           $(SRC_DIR)/hal_emu/i2c.c \
           $(SRC_DIR)/hal_emu/adc.c \
           $(SRC_DIR)/hal_emu/pwm.c \
           $(SRC_DIR)/hal_emu/gnss.c \
           $(SRC_DIR)/hal_emu/rc.c \
           $(SRC_DIR)/libc/string.c \
           $(SRC_DIR)/state/state.c \
           $(SRC_DIR)/ctrl_stub/ctrl_stub.c

HOST_BASE := $(filter-out $(SRC_DIR)/hal_emu/%,$(RV_SRCS))
HOST_BASE := $(filter-out $(SRC_DIR)/main.c,$(HOST_BASE))
HOST_SRCS := $(HOST_BASE) \
             $(SRC_DIR)/hal_emu/time_host.c \
             $(SRC_DIR)/hal_emu/hal_host_stubs.c \
             tests/host/test_main.c

# Object files
RV_OBJS  := $(patsubst %.c,$(RV_BUILD_DIR)/%.o,$(RV_SRCS))
HOST_OBJS := $(patsubst %.c,$(HOST_BUILD_DIR)/%.o,$(HOST_SRCS))

# Targets
RV_ELF  := $(RV_BUILD_DIR)/firmware.elf
HOST_ELF := $(HOST_BUILD_DIR)/test_runner

# Default target
all: rv32 host-test

# RV32 target
rv32: $(RV_ELF)

$(RV_ELF): $(RV_OBJS) link.ld crt0.s | $(RV_BUILD_DIR)
	$(RV_CC) $(RV_CFLAGS) $(RV_LDFLAGS) crt0.s $(RV_OBJS) $(LIBGCC) -o $@
	@echo "=== RV32 Build Complete ==="
	@$(RV_SIZE) $@
	@$(RV_OBJDUMP) -h $@ | head -30

$(RV_BUILD_DIR)/%.o: %.c
	@mkdir -p $(dir $@)
	$(RV_CC) $(RV_CFLAGS) -c $< -o $@

$(RV_BUILD_DIR) $(HOST_BUILD_DIR):
	@mkdir -p $@

# Host test target
host-test: $(HOST_ELF)

$(HOST_ELF): $(HOST_OBJS) | $(HOST_BUILD_DIR)
	$(HOST_CC) $(HOST_CFLAGS) $(HOST_LDFLAGS) $(HOST_OBJS) -o $@
	@echo "=== Host Test Build Complete ==="

$(HOST_BUILD_DIR)/%.o: %.c | $(HOST_BUILD_DIR)
	@mkdir -p $(dir $@)
	$(HOST_CC) $(HOST_CFLAGS) -c $< -o $@

# Run host tests
test: host-test
	@echo "=== Running Host Tests ==="
	@$(HOST_ELF)

# Emulator integration test
emu-test: rv32
	@echo "=== Running Emulator Test ==="
	@./target/release/Thejas32rs $(RV_ELF) --max-cycles 1000000

# Check RAM usage from map file
check-ram: rv32
	@echo "=== RAM Usage Check ==="
	@awk '$$2 == "__text_start" {text_start=strtonum($$1)} \
	    $$2 == "__text_end" {text_end=strtonum($$1)} \
	    $$2 == "__data_start" {data_start=strtonum($$1)} \
	    $$2 == "__data_end" {data_end=strtonum($$1)} \
	    $$2 == "__bss_start" {bss_start=strtonum($$1)} \
	    $$2 == "__bss_end" {bss_end=strtonum($$1)} \
	    END { \
	        text_used = (text_end > text_start) ? text_end - text_start : 0; \
	        data_used = (data_end > data_start) ? data_end - data_start : 0; \
	        bss_used = (bss_end > bss_start) ? bss_end - bss_start : 0; \
	        total_ram = data_used + bss_used; \
	        copy_to_ram = total_ram + text_used; \
	        budget = 262144; \
	        stack_reserve = 32768; \
	        available = budget - stack_reserve; \
	        printf "XIP mode (data+bss): %d bytes (%d KB)\n", total_ram, total_ram/1024; \
	        printf "Copy-to-RAM mode (text+data+bss): %d bytes (%d KB)\n", copy_to_ram, copy_to_ram/1024; \
	        printf "Available for data+bss (256KB - 32KB stack): %d bytes (%d KB)\n", available, available/1024; \
	        if (total_ram > available) { \
	            printf "ERROR: XIP data+bss exceeds %d bytes\n", available; exit 1; \
	        } \
	        if (copy_to_ram > budget) { \
	            printf "ERROR: Copy-to-RAM exceeds %d bytes\n", budget; exit 1; \
	        } \
	    }' $(RV_BUILD_DIR)/firmware.map
	@echo "=== Stack High-Water Mark (run firmware first) ==="
	@echo "TODO: Implement runtime stack high-water reporting"

# Clean
clean:
	rm -rf $(BUILD_DIR)

.PHONY: all rv32 host-test test emu-test check-ram clean