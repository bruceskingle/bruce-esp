#![no_std]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]



use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::time::Rate;
use esp_hal::spi::master::Config;
use mipidsi::{models::ILI9341Rgb565, options::{Orientation}};
use esp_hal::spi::master::Spi;

/* ******************************************************************************************************************************************************
 * Board Support Package for the Cheap Yellow Display (CYD) or ESP32-2432S028R
 *  
 * The CYD uses an ILI9341 display connected via SPI. This uses GPIO pins:
    let miso = peripherals.GPIO12;
    let mosi = peripherals.GPIO13;
    let sclk = peripherals.GPIO14;
    let cs   = peripherals.GPIO15;
    let dc   = peripherals.GPIO2;
    let bl   = peripherals.GPIO21;  // Backlight pin
 ****************************************************************************************************************************************************** */

#[derive(Debug)]
pub enum CydError {
    DisplayInit,
}


pub struct CydResult<'a> {
    pub cyd: Cyd<'a>,
    pub remainder: CydRemainder<'a>,
}

pub struct CydRemainder<'a> {
    pub gpio22: esp_hal::peripherals::GPIO22<'a>,
    pub gpio27: esp_hal::peripherals::GPIO27<'a>,
    pub gpio35: esp_hal::peripherals::GPIO35<'a>,
    pub lpwr: esp_hal::peripherals::LPWR<'a>,
    pub rmt: esp_hal::peripherals::RMT<'a>,
}

pub struct Cyd<'a> {
    pub display: mipidsi::Display<EspDi<'a>, ILI9341Rgb565, mipidsi::NoResetPin>,
    pub backlight_pin: Output<'a>,
    pub led_red_pin: Output<'a>,
    pub led_green_pin: Output<'a>,
    pub led_blue_pin: Output<'a>,
}

impl<'a> Cyd<'a> {
    pub fn backlight(&mut self, on: bool)  {
        match on {
            true => self.backlight_pin.set_high(),
            false => self.backlight_pin.set_low(),
        };
    }

    pub fn led_red(&mut self, on: bool)  {
        match on {
            true => self.led_red_pin.set_low(),
            false => self.led_red_pin.set_high(),
        };
    }

    pub fn led_green(&mut self, on: bool)  {
        match on {
            true => self.led_green_pin.set_low(),
            false => self.led_green_pin.set_high(),
        };
    }

    pub fn led_blue(&mut self, on: bool)  {
        match on {
            true => self.led_blue_pin.set_low(),
            false => self.led_blue_pin.set_high(),
        };
    }
}



// Local display interface wrapper (concrete types) that implements `mipidsi::interface::Interface`.
pub struct EspDi<'a> {
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


pub struct Builder {
    orientation: Option<Orientation>,
}


impl Builder {
    pub fn new() -> Self {
        Self {
            orientation: None,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn init<'a>(
        self,
        peripherals: esp_hal::peripherals::Peripherals,
        mut delay_source: &mut dyn embedded_hal::delay::DelayNs
    ) -> Result<CydResult<'a>, CydError>
    {
        // SPI pins (adjust if needed)
        let miso: esp_hal::peripherals::GPIO12<'_> = peripherals.GPIO12;
        let mosi: esp_hal::peripherals::GPIO13<'_> = peripherals.GPIO13;
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


        // Build the interface instance (moves `spi_bus` into the wrapper)
        let di = EspDi {
            spi: spi_bus,
            cs: cs_out,
            dc: dc_out,
        };

        // Initialize the display via the generic Builder using our local interface.
        // let mut display: mipidsi::Display<EspDi<'_>, ILI9341Rgb565, mipidsi::NoResetPin> = 
        // let mut delay_source; = Delay::new();
        let display = match  mipidsi::Builder::new(ILI9341Rgb565, di)
            .display_size(240, 320)
            .orientation(Orientation::new()
                .flip_horizontal()
                // .rotate(Rotation::Deg180)
            )
            .init(&mut delay_source) {
                Err(_e) => return Err(CydError::DisplayInit),
                Ok(display) => display
            }
            ;

   
        let backlight_pin = Output::new(bl, Level::High, OutputConfig::default());
        
        Ok(CydResult{
            cyd: Cyd {
                display,
                backlight_pin,
                led_red_pin: Output::new(peripherals.GPIO4, Level::High, OutputConfig::default()),
                led_green_pin: Output::new(peripherals.GPIO16, Level::High, OutputConfig::default()),
                led_blue_pin: Output::new(peripherals.GPIO17, Level::High, OutputConfig::default()),
            }, 
            remainder: CydRemainder {
                gpio22: peripherals.GPIO22,
                gpio27: peripherals.GPIO27,
                gpio35: peripherals.GPIO35,
                lpwr: peripherals.LPWR,
                rmt: peripherals.RMT,
            }
        })
    }
}
