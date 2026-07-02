# Rusty Nano Frame

<img width="480" height="632" alt="portalVideo" src="https://github.com/user-attachments/assets/aca6d47d-802d-40e7-a776-d403495ad640" />

This project plays a fully hardware-accelerated, 40-frame animation directly from flash memory on an ST7789 display using hardware SPI at 80 MHz.
It uses "bare-metal" Rust without any operating system to achieve maximum possible performance and a perfectly smooth loop. 

To overcome the ESP32-C3's 4MB flash memory limit, the animation is aggressively compressed using block-based **LZ4 compression** (achieving up to 90% space reduction) and decompressed frame-by-frame on the fly directly into the CPU's SRAM cache before being sent over SPI.

## Requirements

To flash this project to your ESP32-C3, you need to install the Rust toolchain and the `espflash` utility.

### 1. Install Rust Toolchain
If you don't have Rust installed, run the following command in your terminal (works on macOS and Linux):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Add RISC-V Target (ESP32-C3 Architecture)
```bash
rustup target add riscv32imc-unknown-none-elf
```

### 3. Install `espflash` (Flashing utility for ESP32)
On macOS, the fastest way is via Homebrew:
```bash
brew install espflash
```
Alternatively, via cargo:
```bash
cargo install espflash
```

## How to Generate Your Own Animation

If you want to use a different GIF, you can generate a new LZ4 animation payload using the provided Python script.

1. Ensure you have Python installed, activate the virtual environment, and install the dependencies:
```bash
python3 -m venv venv
source venv/bin/activate
pip install Pillow lz4
```

2. Open `generate_lz4.py` and change the path to your source GIF.
3. Run the generator:
```bash
python3 generate_lz4.py
```
This script will resize the GIF to 240x280 (ST7789 size), compress each frame individually with LZ4, and save the result as `anim.lz4` in the project root. The rust compiler will automatically embed this file into the firmware during compilation!

## How to Flash to ESP32

Open a terminal in this directory (where the `Cargo.toml` file is located), connect your ESP32-C3 via USB, and run:

```bash
cargo run --release
```

**What does this command do?**
1. It compiles the code with ultimate optimizations (`--release`, `opt-level = 3`). During compilation, Rust takes the `anim.lz4` file and embeds it directly into the final executable!
2. It automatically detects your ESP32 connected via USB. Thanks to the `.cargo` configuration, it uses `espflash` and our custom `partitions.csv` to flash the board.
3. In about a minute, you'll have a perfect hardware-accelerated, real-time LZ4 decompressed GIF loop running on your display!

## Pre-built Firmware Flashing (No Rust Needed)

If you just want to flash the pre-compiled animation to your ESP32-C3 without installing the Rust toolchain, you can use the generated `firmware.bin` file. This file contains the bootloader, partition table, and the compiled app all merged into a single binary.

You can flash it using the standard `esptool.py` (which works on any OS with Python) to offset `0x0`:
```bash
esptool.py -p /dev/ttyUSB0 -b 460800 write_flash 0x0 firmware.bin
```
*(Note: Replace `/dev/ttyUSB0` with your actual serial port. On macOS it might be `/dev/cu.usbmodem...` or `/dev/cu.usbserial...`)*

Alternatively, you can use a web-based flasher like [Adafruit ESPTool](https://adafruit.github.io/Adafruit_WebSerial_ESPTool/) directly from a Chrome or Edge browser. Just upload `firmware.bin` and flash it to address `0x0`.

## Changing Display Resolution

If you want to use a different ST7789 display (for example, a 240x240 square display instead of 240x280), you only need to change 3 simple things:

1. **Python Script (`generate_lz4.py`)**:
   Update the resize dimensions: `frame_rgb.resize((240, 240), Image.Resampling.LANCZOS)`
2. **Rust Buffer Size (`src/bin/main.rs`)**:
   Change the `FRAME_BUFFER` size to match your total pixels * 2 (e.g. 240 * 240 * 2 = 115200):
   `static mut FRAME_BUFFER: [u8; 115200] = [0; 115200];`
3. **Rust Display Coordinates (`src/bin/main.rs`)**:
   Adjust the bounding box for the display controller in the init block:
   `let y1 = 240u16 - 1;` and `let x1 = 240u16 - 1;`

After making these changes, re-run the python generator and rebuild the Rust project.

## Hardware Wiring (ST7789 Display)

* **VCC:** 3.3V
* **GND:** GND
* **MOSI / DIN / SDA:** GPIO 7
* **CLK / SCLK:** GPIO 6
* **CS / CE0:** GPIO 10
* **DC:** GPIO 4
* **RST:** GPIO 3
* **BL:** GPIO 2 (or connect to 3.3V)
