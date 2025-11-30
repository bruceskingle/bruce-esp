#![no_std]
#![no_main]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]


use core::fmt::Write;
use chrono::NaiveTime;
use chrono::Timelike;
use defmt::info;
use embedded_graphics::primitives::Circle;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use esp_hal::rtc_cntl::Rtc;
use heapless::String;

use embedded_graphics::text::Text;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
// use embedded_graphics::prelude::*;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use mipidsi::models::ST7789;
use esp_hal::spi::master::Config;
use mipidsi::{Builder, models::ILI9341Rgb565, options::{Orientation, Rotation}};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use esp_hal::spi::master::Spi;
use ws_esp32_c6_147_clock::*;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();





#[main]
fn main() -> ! {
    // generator version: 0.6.0

    // esp_println::defmt::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let mut delay = Delay::new();

    /*
    
    If you want to use the default mapping (like Waveshare example) for the LCD on ESP32-C6-LCD-1.47:
SCK — GPIO 38
MOSI — GPIO 39
CS — GPIO 21
DC — GPIO 45
Plus typical VCC, GND, RST (reset), etc per your display’s wiring
If you need full SPI (e.g. with MISO), you can also assign MISO to a spare GPIO, or use the board’s other SPI-capable pins (through the GPIO matrix) — but for a TFT LCD display typically you only need MOSI + SCK + CS + DC + Reset (and back-light if needed).
     */

    // SPI pins (adjust if needed)
    // let miso = peripherals.GPIO39;
    let mosi = peripherals.GPIO39;
    let sclk = peripherals.GPIO38;
    let cs   = peripherals.GPIO21;
    let dc   = peripherals.GPIO45;
    let bl   = peripherals.GPIO22;  // Backlight pin

    // Configure SPI
    let config = Config::default().with_mode( esp_hal::spi::Mode::_0)
        .with_frequency(Rate::from_mhz(20))
        .with_read_bit_order(esp_hal::spi::BitOrder::MsbFirst)
        .with_write_bit_order(esp_hal::spi::BitOrder::MsbFirst);
        // .with_cs_active_high(false);
    
    let spi_bus = Spi::new(
            peripherals.SPI2,
            config,
        )
        .unwrap()
        .with_sck(sclk)
        .with_mosi(mosi)
        // .with_miso(miso)
        ;

    // Re-enable display using `mipidsi`'s `SpiInterface` which targets embedded-hal v1.
    // We'll create a display interface from the already-created `spi_bus` and the DC/CS pins
    // and then initialize the ILI9341 RGB565 driver.
    // NOTE: if the compile fails here it's likely due to a trait-version mismatch in
    // the dependency graph; we'll iterate on that if needed.

    // Create output pins (use esp-hal's Output wrapper) so we can toggle them directly
    // without relying on embedded-hal trait impls from other crates.
    let cs_out = Output::new(cs, Level::High, OutputConfig::default());
    let dc_out = Output::new(dc, Level::High, OutputConfig::default());

    // Local display interface wrapper (concrete types) that implements `mipidsi::interface::Interface`.
    struct EspDi<'a> {
        spi: esp_hal::spi::master::Spi<'a, esp_hal::Blocking>,
        cs: Output<'a>,
        dc: Output<'a>,
    }

    impl<'a> mipidsi::interface::Interface for EspDi<'a> {
        type Word = u8;
        type Error = esp_hal::spi::Error;

        fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
            let _ = self.cs.set_low();
            let _ = self.dc.set_low();
            self.spi.write(&[command])?;
            if !args.is_empty() {
                let _ = self.dc.set_high();
                self.spi.write(args)?;
            }
            let _ = self.cs.set_high();
            Ok(())
        }

        fn send_pixels<const N: usize>(
            &mut self,
            pixels: impl IntoIterator<Item = [Self::Word; N]>,
        ) -> Result<(), Self::Error> {
            let _ = self.cs.set_low();
            let _ = self.dc.set_high();
            for chunk in pixels {
                // chunk is [u8; N]
                self.spi.write(&chunk)?;
            }
            let _ = self.cs.set_high();
            Ok(())
        }

        fn send_repeated_pixel<const N: usize>(
            &mut self,
            pixel: [Self::Word; N],
            mut count: u32,
        ) -> Result<(), Self::Error> {
            let _ = self.cs.set_low();
            let _ = self.dc.set_high();
            let mut buf = [0u8; 64];
            while count > 0 {
                let chunk_count = core::cmp::min(count, (buf.len() / N) as u32);
                let mut idx = 0usize;
                for _ in 0..chunk_count {
                    for &b in &pixel {
                        buf[idx] = b;
                        idx += 1;
                    }
                }
                self.spi.write(&buf[..idx])?;
                count -= chunk_count;
            }
            let _ = self.cs.set_high();
            Ok(())
        }
    }

    // Build the interface instance (moves `spi_bus` into the wrapper)
    let di = EspDi {
        spi: spi_bus,
        cs: cs_out,
        dc: dc_out,
    };

    // Initialize the display via the generic Builder using our local interface.
    let mut display = Builder::new(ST7789, di)
        .display_size(172, 320)
        .orientation(Orientation::new()
            .flip_horizontal()
            // .rotate(Rotation::Deg180)
        )
        .init(&mut delay)
        .unwrap();



    // Turn on the backlight
    let mut backlight = Output::new(bl, Level::High, OutputConfig::default());
    backlight.set_high();


    display.clear(Rgb565::RED).unwrap();

    let background = [Rgb565::GREEN, Rgb565::WHITE, Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE];

    // // Draw text
    // let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    // let text = Text::new("Hello World", Point::new(10, 10), text_style);
    // {
    //     text.draw(&mut display).unwrap();
    // }

    let rtc = Rtc::new(peripherals.LPWR);
    


    rtc.set_current_time_us(((8 * 60) + 5) * 60 * 1000 * 1000); // set to 08:05:00.000
    let mut elapsed_ms: u32;
    let mut bg = 0;

    display.clear(background[bg]).unwrap();
    bg = (bg + 1) % background.len();

    let clock_color = Rgb565::GREEN;
    let bg_color = Rgb565::BLACK;
    let text_style = MonoTextStyle::new(&FONT_10X20, clock_color);

    display.clear(bg_color).unwrap();

    let clock_face = create_face(&display);
    draw_face(&mut display, &clock_face, clock_color).unwrap();

    let mut prev_hour = 0;
    let mut prev_minute = 0;
    let mut prev_second = 0;    

    loop {
        // let now = Timestamp::from_microsecond(rtc.current_time_us() as i64)?;
        let now = rtc.current_time_us() as i64;
        info!("now: {}", now);
        elapsed_ms = (now / 1000) as u32;
        info!("elapsed_ms: {}", elapsed_ms);
        let secs = elapsed_ms / 1000;
        let bedtime = 1000 * (secs + 1) - elapsed_ms;
        let nanos = now % 1_000_000;
        // let duration = chrono::Duration::milliseconds(now);
        // let time = NaiveTime::from_hms_milli_opt(0, 0, secs, ms).unwrap();
        let time = NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos as u32).unwrap();

        let mut time_str: String<64> = String::new();
        write!(time_str, "Time: {:02}:{:02}:{:02}", time.hour(), time.minute(), time.second()).unwrap();

        // info!("Time: {}ms", now);

        info!("Time: {}", time_str.as_str());
        

                // erase previous text by drawing a filled rectangle behind the text area
        // adjust TXT_W/TXT_H to cover the longest text you'll draw
        const TXT_POS: Point = Point::new(0, 0);
        const TXT_W: u32 = 220;
        const TXT_H: u32 = 40;
        let erase = Rectangle::new(TXT_POS, Size::new(TXT_W, TXT_H))
            .into_styled(PrimitiveStyle::with_fill(bg_color));
        erase.draw(&mut display).unwrap();
        
        // let mut buf: String<64> = String::new();
        // write!(buf, "Time: {}ms", elapsed_ms).unwrap();
        let text = Text::new(time_str.as_str(), Point::new(0, 30), text_style);
        text.draw(&mut display).unwrap();

                delay.delay_millis(bedtime);
        

        let hour = time.hour();
        let minute = time.minute();
        let second = time.second();

        if hour != prev_hour {
             draw_hand(&mut display, &clock_face, bg_color, hour_to_angle(prev_hour), -60).unwrap();
        }
        if minute != prev_minute {
             draw_hand(&mut display, &clock_face, bg_color, sexagesimal_to_angle(prev_minute), -30).unwrap();
        }

        if second != prev_second {
            let seconds_radians = sexagesimal_to_angle(prev_second);
            draw_hand(&mut display, &clock_face, bg_color, seconds_radians, 0).unwrap();
            draw_second_decoration(&mut display, &clock_face, bg_color, bg_color, seconds_radians, -20).unwrap();
        }

        prev_hour = hour;
        prev_minute = minute;
        prev_second = second;

        draw_hand(&mut display, &clock_face, clock_color, hour_to_angle(hour), -60).unwrap();
        draw_hand(&mut display, &clock_face, clock_color, sexagesimal_to_angle(minute), -30).unwrap();

        let seconds_radians = sexagesimal_to_angle(second);
        draw_hand(&mut display, &clock_face, clock_color, seconds_radians, 0).unwrap();
        draw_second_decoration(&mut display, &clock_face, clock_color, bg_color, seconds_radians, -20).unwrap();

        // Draw a small circle over the hands in the center of the clock face.
        // This has to happen after the hands are drawn so they're covered up.
        Circle::with_center(clock_face.center(), 9)
            .into_styled(PrimitiveStyle::with_fill(clock_color))
            .draw(&mut display).unwrap();

        // window.update(&display);
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
