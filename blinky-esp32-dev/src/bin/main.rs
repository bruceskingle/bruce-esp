#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::rmt::Rmt;
use esp_hal::time::{Duration, Instant, Rate};
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8,SmartLedsWrite};
use log::info;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // generator version: 0.6.0

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO2,  smart_led_buffer!(2));

    const LEVEL: u8 = 10;
    let mut color = RGB8::default();
    let mut color2 = RGB8::default();
    
    color.r = LEVEL;
    color2.g = LEVEL;

    let delay = Delay::new();

    loop {
        info!("Blink!");
        led.write([color, color2].into_iter()).unwrap();
        delay.delay_millis(500);

        color2 = color;
        let tmp = color.r;
        color.r = color.g;
        color.g = color.b;
        color.b = tmp;
    }
     
    // let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    // led.set_high();

    //  let delay = Delay::new();

    // loop {
    //     info!("Hello world!");
    //     led.toggle();
    //     delay.delay_millis(500);
    // }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.1/examples/src/bin
}
