#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::fmt::Write;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use {esp_backtrace as _, esp_println as _};
use heapless::String;
use esp_println::println;
use chrono::{NaiveTime, Timelike, DateTime};

use embedded_graphics::text::Text;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::{
    primitives::{Circle, PrimitiveStyle, Rectangle},pixelcolor::Rgb565, prelude::*};

use bosch_bme680 as bme680;
// use bme680::{Bme680, Oversampling};

// use aqm_cyd::bsec::Bsec;
// use libalgobsec_sys as bsec;


// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();


use esp_hal::{
    i2c::master::I2c,
    rtc_cntl::Rtc,
};

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.0.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);




    let mut delay = embassy_time::Delay;
    info!("Embassy initialized!");

    let cyd_result = cyd_bsp::Builder::new()
        .init(peripherals, &mut delay)
        .unwrap();

    let rtc = Rtc::new(cyd_result.remainder.lpwr);
    // let start = 1733223472123456789;
    rtc.set_current_time_us(1733223472123456);
        
        
        //((8 * 60) + 5) * 60 * 1000 * 1000); // set to 08:05:00.000


    let timg0 = TimerGroup::new(cyd_result.remainder.timg0);
    esp_rtos::start(timg0.timer0);

    let mut cyd = cyd_result.cyd;
    cyd.backlight(true);


    let fg_color = Rgb565::GREEN;
    let bg_color = Rgb565::BLACK;
    let text_style = MonoTextStyle::new(&FONT_10X20, fg_color);

    cyd.display.clear(bg_color).unwrap();

    let text = Text::new("Hello bosch_bme680", Point::new(0, 30), text_style);
    text.draw(&mut cyd.display).unwrap();

    info!("Trace 1");
    // Create I2C0 driver
    let ic2_config = esp_hal::i2c::master::Config::default();

    info!("Trace 2");
   let mut i2c = I2c::new(cyd_result.remainder.i2c0, ic2_config).unwrap()
    .with_sda(cyd_result.remainder.gpio22)
    .with_scl(cyd_result.remainder.gpio27);
    
    info!("Trace 3");
    let bosch_config = bosch_bme680::Configuration::default();

    info!("Trace 4");
    let mut bme = match  bosch_bme680::Bme680::new(i2c, bosch_bme680::DeviceAddress::Primary, delay, &bosch_config, 20) {
        Ok(bme) => bme,
        Err(e) => {
            defmt::info!("Failed to initialize BME680 sensor");
            panic!("Failed to initialize BME680 sensor: {:?}", e);
        }
    };



    // Example configuration
    let config = bme680::Configuration {
        // temperature_oversampling: Some(bme680::Oversampling::By8),
        // humidity_oversampling: Some(bme680::Oversampling::By2),
        // pressure_oversampling: Some(bme680::Oversampling::By4),
        // filter: Some(bme680::IIRFilter::Coeff3),  // or whatever filter variant
        ..Default::default()
    };

    // // Set heater to 320 °C for 150 ms (recommended for BSEC)
    // config.gas_config.temperature = 320;
    // config.heater_duration = 150;
    // config.gas_measuring = true;  // very important

    bme.set_configuration(&config).unwrap();


    Timer::after(Duration::from_millis(100)).await;

    // // BSEC instance
    // let mut bsec = Bsec::new();
    // let version = bsec.version();
    // println!("BSEC v{}.{}.{}.{}", version.major, version.minor, version.major_bugfix, version.minor_bugfix);

    let mut loop_cnt=0;
    info!("Trace 5");
    loop {
       
        loop_cnt += 1;
        println!("Loop count: {}", loop_cnt);

    info!("Trace 6");
        if let Ok(meas) = bme.measure() {
            let t = meas.temperature;
            let h = meas.humidity;
            let p = (meas.pressure / 100.0) as u32; // convert Pa to hPa
            let g = meas.gas_resistance.expect("Expected gas resistance");

            let mut time_str: String<64> = String::new();
            write!(time_str, "T={:.1}C, H={:.1} %, P={}, G={:.2} Ω", t, h, p, g).unwrap();

            info!("Measurement: {}", time_str.as_str());
            // erase previous text by drawing a filled rectangle behind the text area
            const TXT_POS: Point = Point::new(0, 0);
            const TXT_W: u32 = 220;
            const TXT_H: u32 = 40;
            let erase = Rectangle::new(TXT_POS, Size::new(TXT_W, TXT_H))
                .into_styled(PrimitiveStyle::with_fill(bg_color));
            erase.draw(&mut cyd.display).unwrap();

            let text = Text::new(time_str.as_str(), Point::new(0, 30), text_style);
            text.draw(&mut cyd.display).unwrap();

            // // let now = 0; // TODO: replace with real timestamp (nanoseconds)
            // let now = rtc.current_time_us() as i64;
            // // let secs = (now / 1_000_000) as u32;
            // // let nanos = now % 1_000_000;
            // let time = DateTime::from_timestamp_micros(now).unwrap();
            // println!("Current time: {}", time);
            // let ns = now ; //* 1000;
            // println!("Feeding BSEC with timestamp {} ns", ns);

            // if let Some(out) = bsec.update(ns, t, h, p, g) {
            //     println!(
            //         "IAQ: {:.2} (acc {}), eCO2: {:.2}, bVOC: {:.2}",
            //         out.iaq, out.iaq_accuracy, out.co2_equiv, out.voc_equiv,
            //     );
            // }
            // else {
            //     println!("BSEC no IAQ output");
            // }
        }
        else {
            println!("Measurement error");
        }

       Timer::after(Duration::from_secs(3)).await; // For LP mode
    }

}

