#![no_std]
#![no_main]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]


use core::fmt::Write;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use heapless::String;

use embedded_graphics::text::Text;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_6X10;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8,SmartLedsWrite};
use log::info;
use esp_hal::spi::master::Config;
use mipidsi::{Builder, models::ILI9341Rgb565, options::{Orientation, Rotation}};
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use esp_hal::spi::master::Spi;

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
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO22,  smart_led_buffer!(3));

    const LEVEL: u8 = 10;
    let mut color = RGB8::default();
    let mut color2 = RGB8::default();
    let mut color3 = RGB8::default();
    
    color.r = LEVEL;
    color2.g = LEVEL;
    color3.b = LEVEL;

    let mut delay = Delay::new();


    // SPI pins (adjust if needed)
    let miso = peripherals.GPIO12;
    let mosi = peripherals.GPIO13;
    let sclk = peripherals.GPIO14;
    let cs   = peripherals.GPIO15;
    let dc   = peripherals.GPIO2;
    let bl   = peripherals.GPIO21;  // Backlight pin

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
        .with_miso(miso);

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
    let mut display = Builder::new(ILI9341Rgb565, di)
        .display_size(240, 320)
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

    // Draw text
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    let text = Text::new("Hello World", Point::new(10, 10), text_style);
    {
        text.draw(&mut display).unwrap();
    }
    


    let mut elapsed_ms: u32 = 0;
    let mut bg = 0;

    display.clear(background[bg]).unwrap();
    bg = (bg + 1) % background.len();

    loop {
        info!("Time: {}ms", elapsed_ms);

                // erase previous text by drawing a filled rectangle behind the text area
        // adjust TXT_W/TXT_H to cover the longest text you'll draw
        const TXT_POS: Point = Point::new(0, 0);
        const TXT_W: u32 = 220;
        const TXT_H: u32 = 12;
        let erase = Rectangle::new(TXT_POS, Size::new(TXT_W, TXT_H))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK));
        erase.draw(&mut display).unwrap();
        
        let mut buf: String<64> = String::new();
        write!(buf, "Time: {}ms", elapsed_ms).unwrap();
        let text = Text::new(buf.as_str(), Point::new(10, 10), text_style);
        text.draw(&mut display).unwrap();

        led.write([color, color2, color3].into_iter()).unwrap();
        delay.delay_millis(1000);
        elapsed_ms += 1000;

        color3 = color2;
        color2 = color;
        let tmp = color.r;
        color.r = color.g;
        color.g = color.b;
        color.b = tmp;

        // display.clear(background[bg]).unwrap();
        // bg = (bg + 1) % background.len();
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
