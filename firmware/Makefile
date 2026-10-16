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
LIBM := $(shell $(RV_CC) -print-file-name=libm.a)
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
           $(SRC_DIR)/hal_emu/flash.c \
           $(SRC_DIR)/libc/string.c \
           $(SRC_DIR)/state/state.c \
           $(SRC_DIR)/ctrl_stub/ctrl_stub.c

HOST_BASE := $(filter-out $(SRC_DIR)/hal_emu/%,$(RV_SRCS))
HOST_BASE := $(filter-out $(SRC_DIR)/main.c,$(HOST_BASE))
HOST_BASE := $(filter-out $(SRC_DIR)/hal_emu/flash.c,$(HOST_BASE))
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

# Test program targets
TEST_PROGRAMS := uart_isolation_test strict_mem_test qspi_persistence_test i2c_sensor_test sram_boundary_test

# Cycle budget benchmarks (linked with libm for sqrtf/sinf/cosf/atan2f)
BENCH_PROGRAMS := cycle_bench

# Default target
all: rv32 host-test test-programs bench

# Test programs (exclude main.c to avoid duplicate main)
TEST_OBJS := $(filter-out $(RV_BUILD_DIR)/src/main.o,$(RV_OBJS))

# Test programs
test-programs: $(addprefix $(RV_BUILD_DIR)/,$(addsuffix .elf,$(TEST_PROGRAMS)))

$(RV_BUILD_DIR)/%.elf: programs/%.c link.ld crt0.s $(TEST_OBJS) | $(RV_BUILD_DIR)
	$(RV_CC) $(RV_CFLAGS) $(RV_LDFLAGS) crt0.s $< $(TEST_OBJS) $(LIBGCC) -o $@
	@echo "=== Test Program: $* ==="
	@$(RV_SIZE) $@

# Benchmark programs (linked with libm)
bench: $(addprefix $(RV_BUILD_DIR)/,$(addsuffix .elf,$(BENCH_PROGRAMS)))

$(RV_BUILD_DIR)/cycle_bench.elf: programs/cycle_bench.c link.ld crt0.s $(TEST_OBJS) | $(RV_BUILD_DIR)
	$(RV_CC) $(RV_CFLAGS) $(RV_LDFLAGS) crt0.s $< $(TEST_OBJS) -Wl,--start-group $(LIBGCC) $(LIBM) -Wl,--end-group -o $@
	@echo "=== Benchmark Program: cycle_bench ==="
	@$(RV_SIZE) $@

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
# --strict-mem: fault on unmapped access, PC=0 is fault (not clean exit)
# --test-finished-reg 4096 (0x1000): absolute byte ADDRESS in emulator space.
#   0x1000 is unmapped low memory the firmware never touches normally, so a
#   write there can only come from an explicit test write. Checked FIRST in
#   bus write paths, so it never trips --strict-mem. Firmware writes the
#   desired exit code (low byte); emulator exits with that code.
# Fails on: non-zero exit code, or "OVERRUNS DETECTED" in UART output.
EMU := ../target/release/Thejas32rs
EMU_SMOKE_CYCLES := 200000000
EMU_TEST_OUT := $(RV_BUILD_DIR)/emu-test.log

# Always rebuild the emulator first: a stale target/release binary silently
# invalidates every run below (learned the hard way).
$(EMU): $(wildcard ../src/*.rs ../src/cpu/*.rs ../Cargo.toml)
	cd .. && cargo build --release

emu-test: rv32 test-programs $(EMU)
	@echo "=== Emulator Smoke Test (strict-mem, sram-kb 256, ~2 firmware-sec) ==="
	@$(EMU) $(RV_ELF) --max-cycles $(EMU_SMOKE_CYCLES) --strict-mem --sram-kb 256 --test-finished-reg 4096 > $(EMU_TEST_OUT) 2>&1; \
	EC=$$?; \
	if [ $$EC -ne 0 ]; then echo "FAIL: firmware smoke exited $$EC"; tail -5 $(EMU_TEST_OUT); exit 1; fi; \
	if grep -q "OVERRUNS DETECTED" $(EMU_TEST_OUT); then echo "FAIL: task overruns detected"; grep "OVERRUNS" $(EMU_TEST_OUT); exit 1; fi; \
	echo "PASS: firmware smoke (exit 0, no overruns)"
	@echo "=== Strict-Mem Negative Test (NULL deref must fault) ==="
	@$(EMU) $(RV_BUILD_DIR)/strict_mem_test.elf --max-cycles 100000000 --strict-mem > $(RV_BUILD_DIR)/strict-mem.log 2>&1; \
	EC=$$?; \
	if [ $$EC -eq 0 ]; then echo "FAIL: strict-mem test exited 0, expected fault"; exit 1; fi; \
	echo "PASS: strict-mem faulted as expected (exit $$EC)"
	@echo "=== SRAM Boundary Test (0x80040000 must fault at 256 KB) ==="
	@$(EMU) $(RV_BUILD_DIR)/sram_boundary_test.elf --max-cycles 100000000 --strict-mem --sram-kb 256 > $(RV_BUILD_DIR)/sram-boundary.log 2>&1; \
	EC=$$?; \
	if [ $$EC -eq 0 ]; then echo "FAIL: sram boundary write exited 0, expected fault"; exit 1; fi; \
	echo "PASS: sram boundary faulted as expected (exit $$EC)"
	@echo "=== QSPI Persistence Test ==="
	@rm -f $(RV_BUILD_DIR)/emu-flash.bin
	@$(EMU) $(RV_BUILD_DIR)/qspi_persistence_test.elf --max-cycles 100000000 --flash-file $(RV_BUILD_DIR)/emu-flash.bin > /dev/null 2>&1; \
	EC=$$?; \
	if [ $$EC -ne 0 ]; then echo "FAIL: qspi test pass 1 exited $$EC"; exit 1; fi; \
	[ -f $(RV_BUILD_DIR)/emu-flash.bin ] || (echo "FAIL: no flash file written"; exit 1); \
	echo "PASS: qspi persistence (16MB file written)"

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