#![no_std]
#![no_main]

// CYD BSP — Board Support module for the "CYD (ESP32-2432S028R)" / Waveshare-like board
// This module demonstrates the *idiomatic split-ownership* pattern:
// - `BoardParts::new(...)` consumes `Peripherals` and returns a `Board` (owns device-specific resources)
//   plus a `BoardParts::Leftovers` struct containing peripherals and pins *not* taken by the BSP.
// - The board owns only the things it needs (display driver, backlight pin, optional touch, etc.)
// - Everything else is returned to the caller so they can use them freely.
//
// Important: you will likely need to adapt pin names and some APIs to your `esp-hal` and
// mipidsi/driver crate versions. The code below is intentionally explicit and documented where
// you may need to change things.

// --- Public re-exports and uses ---
use core::convert::Infallible;

use embedded_hal::digital::v2::OutputPin;

// Replace these imports to match your esp-hal version
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::spi::{Config as SpiConfig, Spi};
use esp_hal::gpio::{Output as HalOutput, PinDriver, Unknown, Level, OutputConfig};

// Replace with whatever ILI/driver you're using. Example: mipidsi + ILI9341Rgb565
use mipidsi::Builder as MipiBuilder;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::interface::SpiInterface as MipiSpiInterface;

// NOTE: the exact `Display` type and builder signatures will vary with mipidsi versions.
// The code below follows the pattern used in mipidsi 0.9.x where you supply an interface
// that implements the driver traits.

// ----------------------------- Public types ---------------------------------

/// The high-level Board handle: owns the display driver and peripheral pins the BSP needs.
pub struct CydBoard<DI, RST> {
    /// The mipidsi display driver instance
    pub display: mipidsi::Display<DI, RST>,

    /// Backlight control pin (owned by BSP)
    pub backlight: PinDriver<esp_hal::gpio::Gpio21, HalOutput>,
}

/// The result of initialising the board: the BSP plus leftover peripherals you'll use elsewhere.
pub struct CydBoardParts<LeftoverSPI, LeftoverI2C, PINS> {
    /// The BSP-owned board object
    pub board: CydBoard<LeftoverSPI, mipidsi::NoResetPin>,

    /// The SPI peripheral we didn't take (if you need to use it elsewhere)
    pub spi2: LeftoverSPI,

    /// Example leftover I2C peripheral (if you didn't consume it)
    pub i2c0: LeftoverI2C,

    /// Other pins (raw) that the caller may wish to use
    pub pins: PINS,
}

// ----------------------------- Public API -----------------------------------

/// Initialise the CYD board. Consumes `Peripherals` and returns the BSP plus leftover peripherals.
///
/// Notes:
/// - `Peripherals` is the esp-hal `Peripherals` (chip/peripheral ownership).
/// - `Delay` is used for display init sequences.
/// - You will very likely need to adapt the pin names and types to match your esp-hal version.
pub fn init_cyd_board<LeftoverSPI, LeftoverI2C, PINS>(
    mut peripherals: Peripherals,
    clocks: ClockControl,
    delay: &mut Delay,
) -> Result<CydBoardParts<LeftoverSPI, LeftoverI2C, PINS>, InitError>
where
    // We keep these generic because different esp-hal versions give different concrete types.
    // The caller may downcast / rewrap if required. If you wish, replace `LeftoverSPI` with
    // `esp_hal::spi::Spi<esp_hal::peripherals::SPI2, ...>` concrete type.
    LeftoverSPI: 'static,
    LeftoverI2C: 'static,
    PINS: 'static,
{
    // ---------------------------------------------------------------------
    // 1) Acquire the GPIO pins we need for the display.
    // ---------------------------------------------------------------------
    // The pin identifiers below (GPIO12/13/14/15/2/21) are the example pins used in
    // earlier code; change them to match your board's exposed pins.

    // Create a `Pins` struct from esp-hal to split the GPIO block (API varies across esp-hal)
    // let pins = esp_hal::gpio::Pins::new(peripherals.GPIO);
    // Example (pseudo-code) to convert raw GPIO into push-pull outputs:

    // NOTE: If your esp-hal version uses a different constructor for pins, adapt accordingly.
    // For clarity we show the important intent: we claim ownership of the pins used for
    // SPI (MISO/MOSI/SCLK/CS), DC pin, reset (if any) and backlight.

    // --- Example pin binding (replace with your esp-hal's API) ---
    // let miso = pins.gpio12.into_floating_input();
    // let mosi = pins.gpio13.into_push_pull_output();
    // let sclk = pins.gpio14.into_push_pull_output();
    // let cs   = pins.gpio15.into_push_pull_output();
    // let dc   = pins.gpio2.into_push_pull_output();
    // let bl   = pins.gpio21.into_push_pull_output();

    // ---------------------------------------------------------------------
    // 2) Configure SPI for the display.
    // ---------------------------------------------------------------------
    // Build a spi config — update fields to match your hal's types/function names.

    let spi_config = SpiConfig::default()
        // example: set mode 0 and frequency ~20MHz. Adjust to your hardware.
        .with_mode(esp_hal::spi::Mode::Mode0)
        .with_baudrate(20_000_000u32);

    // Create the SPI bus that we'll hand to the display interface. The exact constructor
    // form depends on your esp-hal version; the pattern is: Spi::new(peripheral, config)
    // .with_sck(sclk).with_mosi(mosi).with_miso(miso)

    // let spi_bus = Spi::new(peripherals.SPI2, spi_config)
    //     .unwrap()
    //     .with_sck(sclk)
    //     .with_mosi(mosi)
    //     .with_miso(miso);

    // ---------------------------------------------------------------------
    // 3) Build the mipidsi transport (a lightweight adapter object)
    // ---------------------------------------------------------------------
    // Here we show the conceptual steps. In practice you will either:
    //  - use an existing `EspDi` adapter that wraps an `esp-hal` Spi + cs/dc pins, or
    //  - implement a tiny adapter that implements the embedded-hal traits expected by mipidsi.

    // Example pseudocode:
    // let cs_out = PinDriver::output(cs, Level::High, OutputConfig::default());
    // let dc_out = PinDriver::output(dc, Level::Low, OutputConfig::default());
    // let di = EspDi::new(spi_bus, cs_out, dc_out);

    // let display = MipiBuilder::new(ILI9341Rgb565, di)
    //     .display_size(240, 320)
    //     .orientation(mipidsi::Orientation::new().flip_horizontal())
    //     .init(delay)?;

    // let backlight_pin = PinDriver::output(bl, Level::High, OutputConfig::default());

    // ---------------------------------------------------------------------
    // 4) Construct Board + Leftovers and return
    // ---------------------------------------------------------------------

    // NOTE: the code below is placeholder to express the return types. Replace the
    // concrete expressions with your initialized values.

    {
        // ===== Fill-in implementation for esp-hal 1.0.0 (ESP32) + mipidsi 0.9 =====

        // --- 1. Construct a Pins struct ---
        let pins = esp_hal::gpio::Pins::new(peripherals.GPIO);

        // CYD wiring (ESP32-2432S028R) uses these external pins:
        let miso = pins.gpio12.into_floating_input();
        let mosi = pins.gpio13.into_push_pull_output();
        let sclk = pins.gpio14.into_push_pull_output();
        let cs   = pins.gpio15.into_push_pull_output();
        let dc   = pins.gpio2.into_push_pull_output();
        let bl   = pins.gpio21.into_push_pull_output();

        // --- 2. SPI2 configuration ---
        let spi = Spi::new(
            peripherals.SPI2,
            sclk,
            mosi,
            miso,
            spi_config,
        );

        // --- 3. Wrap CS + DC pins ---
        let cs_out = PinDriver::output(cs, Level::High);
        let dc_out = PinDriver::output(dc, Level::High);

        // --- 4. Build mipidsi SPI interface ---
        let di = MipiSpiInterface::new(spi, dc_out, cs_out);

        // --- 5. Create ILI9341 driver ---
        let display = MipiBuilder::new(ILI9341Rgb565, di)
            .display_size(240, 320)
            .orientation(mipidsi::Orientation::new().flip_horizontal())
            .init(delay)
            .map_err(|_| InitError::DisplayInitError)?;

        // Backlight control
        let backlight = PinDriver::output(bl, Level::High);

        // Return board + leftovers
        Ok(CydBoardParts {
            board: CydBoard {
                display,
                backlight,
            },
            // We return placeholder leftovers since you didn't list I2C0 or remaining pins.
            spi2: (),
            i2c0: (),
            pins: (),
        })
    }
}

// ----------------------------- Errors ---------------------------------------

/// Error(s) that might occur during BSP initialisation.
#[derive(Debug)]
pub enum InitError {
    /// The hal-specific initialisation code is not filled in (placeholder)
    NotImplemented,

    /// An SPI bus or display init returned an error. We box it because concrete
    /// driver error types vary across versions / interfaces.
    SpiInitError,

    /// The display driver reported an error during init
    DisplayInitError,
}

// ----------------------------- Guidance -------------------------------------

/*
Notes and adaptation checklist for you (read this and then customise the stub above):

1) Pin & Spi APIs
   - Your esp-hal version probably exposes pins via `Pins::new(peripherals.GPIO)` or similar.
   - Convert pins to the right types (push-pull outputs, inputs) using the hal API.
   - Create the SPI peripheral with `Spi::new(..)` and attach pins with `.with_sck(..)` etc.

2) Build an adapter for mipidsi
   - mipidsi expects an interface type that implements the embedded-hal traits.
   - If your Spi type already implements `embedded-hal` traits the wrapper can be tiny.
   - Otherwise implement a small wrapper type that implements the `Write` trait by forwarding
     to `spi.write()` and toggling CS/DC via the pin drivers you created.

3) Choose ownership boundaries
   - Move the display + backlight pins into the `CydBoard` struct
   - Return remaining peripherals (eg: `I2C0`, `SPI1`, free pins) in the `Leftovers` struct.

4) Testing & iteration
   - Start by implementing only the backlight toggle and a simple `display.clear()`.
   - Add more features as you confirm peripheral initialisation works.

5) If pins like GPIO38/39 are not exposed
   - The display may be wired internally on the module (not exposed as `Peripherals.GPIOxx`)
   - In that case you either: (a) use a board-specific HAL that knows how to access the internal
     pads, or (b) write a PAC-level SPI backend that writes the SPI2 registers directly.

6) Example reference projects
   - Look for repos that initialise ILI9341 or mipidsi on ESP32-C6/CYD boards. They often show
     the exact sequence for registers and helper wrappers.
*/
