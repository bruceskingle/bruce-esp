#![no_std]
#![no_main]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]


use embedded_graphics::text::Text;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::main;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// Simple Hello World program for the cyd BSP
 


#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: esp_hal::peripherals::Peripherals = esp_hal::init(config);
    let mut delay = Delay::new();
    
     let cyd_result = cyd_bsp::Builder::new()
        .init(peripherals, &mut delay)
        .unwrap();

    let mut cyd = cyd_result.cyd;
    cyd.backlight(true);


    let fg_color = Rgb565::GREEN;
    let bg_color = Rgb565::BLACK;
    let text_style = MonoTextStyle::new(&FONT_10X20, fg_color);

    cyd.display.clear(bg_color).unwrap();

    let text = Text::new("Hello World", Point::new(0, 30), text_style);
    text.draw(&mut cyd.display).unwrap();

    loop {
        // Busy loop
    }
}
