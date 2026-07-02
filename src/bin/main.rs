#![no_std]
#![no_main]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};

esp_bootloader_esp_idf::esp_app_desc!();

const ANIM_DATA: &[u8] = include_bytes!("../../anim.lz4");

#[allow(clippy::large_stack_frames)]
#[main]
fn main() -> ! {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            core::ptr::addr_of_mut!(HEAP) as *mut u8,
            HEAP_SIZE,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }

    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    let delay = Delay::new();

    let sclk = peripherals.GPIO6;
    let mosi = peripherals.GPIO7;

    let mut cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let mut dc = Output::new(peripherals.GPIO4, Level::High, OutputConfig::default());
    let mut rst = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let mut bl = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    // 80 MHz SPI frequency for maximum hardware bandwidth (ESP32-C3 max)
    let mut spi = Spi::new(
        peripherals.SPI2,
        Config::default()
            .with_frequency(esp_hal::time::Rate::from_mhz(80))
            .with_mode(Mode::_0),
    )
    .expect("spi")
    .with_sck(sclk)
    .with_mosi(mosi);

    // Hardware reset
    cs.set_high();
    rst.set_high();
    delay.delay_millis(10);
    rst.set_low();
    delay.delay_millis(10);
    rst.set_high();
    delay.delay_millis(10);

    // Send command helper function
    let mut send_cmd = |cmd: u8, data: &[u8]| {
        dc.set_low();
        cs.set_low();
        let _ = spi.write(&[cmd]);
        if !data.is_empty() {
            dc.set_high();
            let _ = spi.write(data);
        }
        cs.set_high();
    };

    // ST7789 init sequence
    send_cmd(0x36, &[0x00]);
    send_cmd(0x3A, &[0x05]); // RGB565
    send_cmd(0xB2, &[0x0B, 0x0B, 0x00, 0x33, 0x35]);
    send_cmd(0xB7, &[0x11]);
    send_cmd(0xBB, &[0x35]);
    send_cmd(0xC0, &[0x2C]);
    send_cmd(0xC2, &[0x01]);
    send_cmd(0xC3, &[0x0D]);
    send_cmd(0xC4, &[0x20]);
    send_cmd(0xC6, &[0x13]);
    send_cmd(0xD0, &[0xA4, 0xA1]);
    send_cmd(0xD6, &[0xA1]);
    send_cmd(
        0xE0,
        &[
            0xF0, 0x06, 0x0B, 0x0A, 0x09, 0x26, 0x29, 0x33, 0x41, 0x18, 0x16, 0x15, 0x29, 0x2D,
        ],
    );
    send_cmd(
        0xE1,
        &[
            0xF0, 0x04, 0x08, 0x08, 0x07, 0x03, 0x28, 0x32, 0x40, 0x3B, 0x19, 0x18, 0x2A, 0x2E,
        ],
    );
    send_cmd(0xE4, &[0x25, 0x00, 0x00]);
    send_cmd(0x21, &[]); // invert on
    send_cmd(0x11, &[]); // sleep out
    delay.delay_millis(120);
    send_cmd(0x29, &[]); // display on
    bl.set_high(); // backlight on

    // Set window 0,0 to 240,280 (with y+20 offset for this panel)
    let y0 = 20u16;
    let y1 = 280u16 + 20 - 1;
    let x0 = 0u16;
    let x1 = 240u16 - 1;
    send_cmd(
        0x2A,
        &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8],
    );
    send_cmd(
        0x2B,
        &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8],
    );

    // Send RAMWR command to start pixel stream
    dc.set_low();
    cs.set_low();
    let _ = spi.write(&[0x2C]);
    dc.set_high();

    let chunk_size = 30_000;

    // Static mut buffer for frame decoding (safe since we only have one thread and no interrupts touching it)
    static mut FRAME_BUFFER: [u8; 134400] = [0; 134400];

    loop {
        let mut offset = 0;

        while offset < ANIM_DATA.len() {
            // Read 4-byte compressed chunk size (Little Endian)
            if offset + 4 > ANIM_DATA.len() {
                break;
            }

            let compressed_size = u32::from_le_bytes([
                ANIM_DATA[offset],
                ANIM_DATA[offset + 1],
                ANIM_DATA[offset + 2],
                ANIM_DATA[offset + 3],
            ]) as usize;
            offset += 4;

            let compressed_chunk = &ANIM_DATA[offset..offset + compressed_size];
            offset += compressed_size;

            // Decompress the frame directly into RAM
            unsafe {
                let uncompressed = lz4_flex::decompress_into(
                    compressed_chunk,
                    &mut *core::ptr::addr_of_mut!(FRAME_BUFFER),
                )
                .unwrap_or(0);

                // Write decompressed frame to SPI
                let frame_data = &(&(*core::ptr::addr_of_mut!(FRAME_BUFFER)))[..uncompressed];
                for chunk in frame_data.chunks(chunk_size) {
                    let _ = spi.write(chunk);
                }
            }
        }
    }
}
