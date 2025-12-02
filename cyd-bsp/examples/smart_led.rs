#![no_std]
#![no_main]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]


use log::info;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::main;

use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8,SmartLedsWrite};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

/* Simple blinky program for the cyd BSP and a WS2812 LED strip. The first three LEDs on the strip blink red, blue and green in sequence.
 *
 * The WS2812 strip is connected to GPIO22, so you need to connect a WS2812 strip to the CYD board's CN1 connector (GND, GPIO22).
 * Since the CYD does not output 5V on CN1 you will need to power the WS2812 strip separately with 5V and connect the grounds together 
 * (by connecting the CN1 GND pin to the strip and then connecting a separate 5V power supply to the WS2812 additional power wires).
 * 
 */
 


#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: esp_hal::peripherals::Peripherals = esp_hal::init(config);
    let mut delay = Delay::new();
    
     let cyd_result = cyd_bsp::Builder::new()
        .init(peripherals, &mut delay)
        .unwrap();

    let mut cyd = cyd_result.cyd;
    cyd.backlight(false); // turn off backlight to save power
    let bg_color = Rgb565::BLACK;
    cyd.display.clear(bg_color).unwrap();

     let rmt = Rmt::new(cyd_result.remainder.rmt, Rate::from_mhz(80)).unwrap();

    let mut led = SmartLedsAdapter::new(rmt.channel0, cyd_result.remainder.gpio22,  smart_led_buffer!(3));

    const LEVEL: u8 = 10;
    let mut color = RGB8::default();
    let mut color2 = RGB8::default();
    let mut color3 = RGB8::default();
    
    color.r = LEVEL;
    color2.g = LEVEL;
    color3.b = LEVEL;
    

    loop {
         info!("Blink!");
        led.write([color, color2, color3].into_iter()).unwrap();
        
        color3 = color2;
        color2 = color;
        let tmp = color.r;
        color.r = color.g;
        color.g = color.b;
        color.b = tmp;

        delay.delay_millis(500u32);
    }
}
