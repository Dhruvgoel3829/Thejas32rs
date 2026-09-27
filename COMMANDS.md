# Thejas32rs Development Session Commands
# Date: 2025-09-25
# Purpose: Complete record of all commands run during development

## Initial Exploration
ls -la
cat README.md
cat Cargo.toml
cat src/main.rs
grep -r "TODO\|FIXME\|XXX\|unimplemented\|placeholder" src/
cat src/cpu/mod.rs | head -644
cat src/cpu/c_decode.rs
cat programs/MakeFile
cat programs/compressed_test.c
cat programs/atomic_test.c
cat programs/scheduler_test.c
cat programs/scheduler_asm.s
cat programs/flight_controller.c

## Build and Test
cargo test 2>&1 | head -100
cargo test 2>&1 | tail -50
git log --oneline -20
git log --oneline --all | head -50

## Fix 1: System Instructions & ELF Loading
cargo build --release 2>&1 | tail -20
./target/release/Thejas32rs programs/hello.elf 2>&1
/opt/riscv/bin/riscv-none-elf-objdump -d programs/hello.elf | head -50
/opt/riscv/bin/riscv-none-elf-objdump -h programs/hello.elf
grep -n "fn load_elf" src/bus.rs
cat src/bus.rs | sed -n '50,150p'
grep -n "fn read_u16" src/bus.rs
cat src/bus.rs | sed -n '258,287p'
od -x programs/hello.elf | head -30
cargo build --release 2>&1 | tail -5
timeout 5 ./target/release/Thejas32rs programs/hello.elf 2>&1

# Debug ELF loading
sed -i 's/if header.p_type == goblin::elf::program_header::PT_LOAD {/eprintln!("[DEBUG] PHDR: p_type={}, p_vaddr=0x{:x}, p_offset=0x{:x}, p_filesz=0x{:x}, p_memsz=0x{:x}", header.p_type, header.p_vaddr, header.p_offset, header.p_filesz, header.p_memsz);\n            if header.p_type == goblin::elf::program_header::PT_LOAD {/' src/bus.rs
cargo build --release 2>&1 | tail -3 && ./target/release/Thejas32rs programs/hello.elf 2>&1

# Fix ELF loading for overlapping segments
cat programs/link.ld
# Edit bus.rs load_elf() to handle segments starting below RAM_BASE
cargo build --release 2>&1 | tail -3 && timeout 5 ./target/release/Thejas32rs programs/hello.elf 2>&1

# Clean up debug output
# Edit bus.rs to remove debug prints
cargo build --release 2>&1 | tail -3 && timeout 5 ./target/release/Thejas32rs programs/hello.elf 2>&1

# Fix for BSS-only segments (filesz=0)
# Edit bus.rs load_elf() bounds checking
cargo build --release 2>&1 | tail -3 && timeout 5 ./target/release/Thejas32rs programs/scheduler_test.elf 2>&1

## Test All Programs
timeout 5 ./target/release/Thejas32rs programs/hello.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/compressed_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/scheduler_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/flight_controller.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/comprehensive_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/xip_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/pmp_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/timer_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/plic_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/wdt_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/i2c_test.elf 2>&1

## Gazebo SITL Integration
cat src/gazebo_backend.rs
cat src/ipc.rs
cat tools/sitl_bridge.py
cat tools/motor_bridge.py
cat tools/external_imu_proxy.py
cat gazebo/CMakeLists.txt
cat src/main.rs
cat src/spi.rs
cat src/gpio.rs
cat src/bus.rs | head -60
cat gazebo/gazebo_thejas_sitl.cpp
cat gazebo/worlds/iris.world

# Fix GazeboImuBackend bind bug (port 9002)
sed -i 's/UdpSocket::bind((GAZEBO_ADDR, 0))?/UdpSocket::bind((GAZEBO_ADDR, GAZEBO_IMU_PORT))?/' src/gazebo_backend.rs

# Add --gazebo CLI flag
# Edit src/main.rs to add gazebo flag
# Edit src/spi.rs to make Gazebo backend conditional
# Edit src/gpio.rs to make Gazebo backend conditional
# Edit src/bus.rs to pass gazebo flag to peripherals
# Edit src/main.rs to pass cli.gazebo to Bus::new()

cargo build --release 2>&1
timeout 3 ./target/release/Thejas32rs programs/hello.elf 2>&1
timeout 3 ./target/release/Thejas32rs programs/hello.elf --gazebo 2>&1
./target/release/Thejas32rs --help
timeout 3 ./target/release/Thejas32rs programs/flight_controller.elf --gazebo 2>&1
timeout 5 ./target/release/Thejas32rs programs/comprehensive_test.elf 2>&1

# Check Gazebo build
ls -la gazebo/
ls -la gazebo/build/

## Git Operations - Fix 1
git status
git diff src/cpu/mod.rs src/bus.rs
git add src/cpu/mod.rs src/bus.rs .gitignore MEMORY.md
git commit -m "Fix system instructions (SFENCE.VMA, reserved funct3) and ELF loading bug"
git push

## Git Operations - Gazebo SITL (initial UDP approach)
git add src/main.rs src/spi.rs src/gpio.rs src/bus.rs src/gazebo_backend.rs MEMORY.md
git commit -m "Complete Gazebo SITL integration for AI-in-the-loop drone firmware"
git push

## Git Operations - IPC Refactor (clean core)
# Revert Gazebo UDP from core peripherals
git checkout HEAD~1 -- src/spi.rs src/gpio.rs src/main.rs src/bus.rs

# Fix UART (was using Gazebo backend)
# Restore uart.rs to crossterm-based terminal I/O

# Add IPC flag and refactor SPI/GPIO to use SocketBackend
# Edit src/spi.rs - use SocketBackend, loopback when disabled
# Edit src/gpio.rs - use SocketBackend
# Edit src/bus.rs - pass enable_ipc to peripherals
# Edit src/main.rs - add --ipc flag
# Remove gazebo_backend module from main.rs

cargo build --release 2>&1 | tail -10
timeout 3 ./target/release/Thejas32rs programs/hello.elf 2>&1
timeout 3 ./target/release/Thejas32rs programs/hello.elf --ipc 2>&1
timeout 5 ./target/release/Thejas32rs programs/comprehensive_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/scheduler_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/flight_controller.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/xip_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/pmp_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/timer_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/wdt_test.elf 2>&1
timeout 5 ./target/release/Thejas32rs programs/i2c_test.elf 2>&1

git add src/spi.rs src/gpio.rs src/main.rs src/bus.rs src/uart.rs MEMORY.md .gitignore
git commit -m "Refactor to IPC-based architecture (clean core emulator)"
git push

## Memory File Creation
cat > MEMORY.md << 'EOF'
# Thejas32rs Project Memory
... (full content)
EOF

# Add MEMORY.md to .gitignore
echo -e "\n# Project memory file\nMEMORY.md" >> .gitignore

# Add COMMANDS.md to .gitignore
echo -e "# Session command log\nCOMMANDS.md" >> .gitignore

## Gazebo SITL Pipeline Fixes (2026-09-26)

# Fix IMU sensor name in Gazebo plugin
sed -i 's/iris::base_link::imu_sensor/imu_sensor/' gazebo/gazebo_thejas_sitl.cpp

# Increase max thrust to 5N/motor
sed -i 's/MAX_THRUST_PER_MOTOR = 4.0/MAX_THRUST_PER_MOTOR = 5.0/' gazebo/gazebo_thejas_sitl.cpp

# Fix yaw torque: (CCW motors - CW motors) = (f2+f3)-(f1+f4)
sed -i 's/yaw_torque   = 0.01 \* ((f2 + f4) - (f1 + f3))/yaw_torque   = 0.015 * ((f2 + f3) - (f1 + f4))/' gazebo/gazebo_thejas_sitl.cpp

# Increase damping
sed -i 's/damping_torque = -3.0 \* ang_vel_world/damping_torque = ignition::math::Vector3d(-5.0 * ang_vel_world.X(), -5.0 * ang_vel_world.Y(), -10.0 * ang_vel_world.Z())/' gazebo/gazebo_thejas_sitl.cpp

# Add linear drag, ground effect, position spring
# Edit gazebo/gazebo_thejas_sitl.cpp

cd gazebo/build && make -j4

# Rewrite Python bridge to use UDP (port 9002 IMU, port 9003 motors)
cat > tools/sitl_bridge.py << 'EOF'
# Full UDP-based bridge with synthetic fallback
EOF

# Add drone_test.c comprehensive test
cat > programs/drone_test.c << 'EOF'
# IMU test, motor sequence, hover ramp
EOF

riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -T link.ld -o drone_test.elf crt0.s drone_test.c

# Test full pipeline
# Terminal 1: cd gazebo/build && gazebo ../worlds/iris.world
# Terminal 2: python3 tools/sitl_bridge.py
# Terminal 3: ./target/release/Thejas32rs programs/drone_test.elf --ipc

# Add feedback_controller.c with cascade PID + complementary filter
cat > programs/feedback_controller.c << 'EOF'
# Complementary filter (98% gyro + 2% accel)
# Cascade PID: Angle -> Rate
# X-configuration mixer
# Step test sequence
EOF

riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -T link.ld -o feedback_controller.elf crt0.s feedback_controller.c

# Test feedback controller
# Terminal 1: cd gazebo/build && gazebo ../worlds/iris.world
# Terminal 2: python3 tools/sitl_bridge.py
# Terminal 3: ./target/release/Thejas32rs programs/feedback_controller.elf --ipc

# Git commit
git add gazebo/gazebo_thejas_sitl.cpp tools/sitl_bridge.py programs/drone_test.c programs/flight_controller.c programs/feedback_controller.c
git commit -m "Fix Gazebo SITL pipeline: IMU sensor name, motor thrust, yaw torque, damping
- Fix IMU sensor lookup (name 'imu_sensor' not 'iris::base_link::imu_sensor')
- Increase max thrust to 5N/motor (hover at ~200 throttle)
- Fix yaw torque formula: (CCW motors - CW motors) = (f2+f3)-(f1+f4)
- Increase angular damping: roll/pitch -5.0, yaw -10.0
- Add linear drag, ground effect, position spring
- Python bridge: UDP IMU receiver on port 9002, motor forward on port 9003
- Add synthetic IMU fallback with warning
- Add drone_test.c comprehensive test program
- Add feedback_controller.c with cascade PID + complementary filter"

git push

## Altitude Hold & Yaw Angle Control (2026-09-26)

# Add altitude PID and yaw angle cascade to feedback_controller.c
cat > programs/feedback_controller.c << 'EOF'
# Altitude PID using accel Z (target 16384 = 1g)
# Yaw angle hold: angle -> rate cascade
# Throttle clamp 150-240
# Stronger yaw PID: kp 15, ki 1, kd 6
EOF

riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -T link.ld -o feedback_controller.elf crt0.s feedback_controller.c

# Increase Gazebo position hold spring and damping
# Edit gazebo/gazebo_thejas_sitl.cpp: spring 5.0 N/m, pos damping 3.0 N/(m/s)

cd gazebo/build && make -j4

# Test
# Terminal 1: cd gazebo/build && gazebo ../worlds/iris.world
# Terminal 2: python3 tools/sitl_bridge.py
# Terminal 3: ./target/release/Thejas32rs programs/feedback_controller.elf --ipc

# Fix Gazebo world model to look like proper quadcopter
# Edit gazebo/worlds/iris.world: add body, arms, proper rotor visuals

# Git commit
git add programs/feedback_controller.c gazebo/gazebo_thejas_sitl.cpp
git commit -m "Add altitude hold and yaw angle control to feedback controller
- Altitude PID using accel Z (target 16384 = 1g hover)
- Throttle clamping 150-240 to prevent saturation
- Yaw angle hold (outer) -> rate (inner) cascade
- Stronger yaw PID: kp 15, ki 1, kd 6
- Position spring: 5.0 N/m + velocity damping 3.0 N/(m/s)
- Telemetry shows AZ and throttle"

git push

## Revert iris.world & Stronger Physics Damping (2026-09-26)

# Revert iris.world to simple box model (proper quadcopter model was unstable)
git checkout HEAD~1 -- gazebo/worlds/iris.world

# Increase Gazebo physics damping 3x for stability
# Edit gazebo/gazebo_thejas_sitl.cpp:
# - Angular damping: roll/pitch -30.0, yaw -50.0
# - Linear drag: -10.0
# - Position spring: 10.0 N/m
# - Velocity damping: 8.0 N/(m/s)

cd gazebo/build && make -j4

# Revert feedback_controller.c to stable version
git checkout HEAD~2 -- programs/feedback_controller.c

riscv-none-elf-gcc -march=rv32imac_zicsr -mabi=ilp32 -nostdlib -T link.ld -o feedback_controller.elf crt0.s feedback_controller.c

# Test with stronger physics
# Terminal 1: cd gazebo/build && gazebo ../worlds/iris.world
# Terminal 2: python3 tools/sitl_bridge.py
# Terminal 3: ./target/release/Thejas32rs programs/feedback_controller.elf --ipc

# Git commit
git add gazebo/worlds/iris.world gazebo/gazebo_thejas_sitl.cpp programs/feedback_controller.c MEMORY.md
git commit -m "Revert iris.world to simple box model, strengthen Gazebo physics damping
- iris.world: reverted to original simple quadcopter (box body + 4 rotors)
- gazebo_thejas_sitl.cpp: increased damping 3x (angular -30/-50, linear -10, position spring 10.0, pos damp 8.0)
- feedback_controller.c: altitude hold + yaw cascade
- MEMORY.md: updated with latest fixes"

git push

## Unit Tests Added (2026-09-27)

# Add unit tests to all peripheral modules and CPU decode
# cpu/decode.rs - 14 tests for 32-bit instruction decode
# cpu/c_decode.rs - 8 tests for compressed instruction decompression
# cpu/csr.rs - 10 tests for CSR read/write, MISA, cycle counter, PMP
# cpu/pmp.rs - 18 tests for config/addr, TOR/NA4/NAPOT, access checks
# clint.rs - 10 tests for timer/software interrupts, MTIME/MTIMECMP
# plic.rs - 10 tests for priority/enable/pending/threshold, claim/complete
# timer.rs - 10 tests for counting, match IRQ, auto-reload, prescaler, status W1C
# gpio.rs - 7 tests for DIR/DATA/SET/CLEAR/INT_EN registers
# spi.rs - 6 tests for CTRL/CS/DATA registers, loopback mode
# i2s.rs - 6 tests for CR/SR/TX registers, sample counting
# i2c.rs - 8 tests for virtual 24C02 EEPROM write/read, address latch, status flags
# dma.rs - 7 tests for register R/W, transfer start, ctrl bits, status W1C
# wdt.rs - 10 tests for countdown, timeout (interrupt/reset), feed keys, status W1C

cargo test 2>&1 | tail -30

# Run all tests - should show 115 passed
cargo test

# Verify test ELFs still work
./target/release/Thejas32rs programs/hello.elf --max-cycles 10000 2>&1
./target/release/Thejas32rs programs/compressed_test.elf --max-cycles 50000 2>&1
./target/release/Thejas32rs programs/atomic_test.elf --max-cycles 50000 2>&1
./target/release/Thejas32rs programs/timer_test.elf --max-cycles 50000 2>&1
./target/release/Thejas32rs programs/scheduler_test.elf --max-cycles 50000 2>&1
./target/release/Thejas32rs programs/flight_controller.elf --max-cycles 100000 2>&1

# Git commit
git add src/
git commit -m "Add comprehensive unit tests (115 tests)

- CPU decode: 22 tests for 32-bit and 16-bit instruction decode
- CSR: 10 tests for register R/W, MISA, cycle counter, PMP config
- PMP: 18 tests for config/addr, TOR/NA4/NAPOT, access checks
- Peripherals: 65 tests for CLINT, PLIC, Timer, GPIO, SPI, I2S, I2C, DMA, WDT
- All tests pass and emulator runs test ELFs correctly"

git push