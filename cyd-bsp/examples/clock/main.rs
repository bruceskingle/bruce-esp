#![no_std]
#![no_main]

#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

mod clock;

use core::fmt::Write;

use defmt::info;
use chrono::{NaiveTime, Timelike};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::String;

use {esp_backtrace as _, esp_println as _};
use esp_hal::{
    main,
    clock::CpuClock,
    delay::Delay,
    rtc_cntl::Rtc,
};

use crate::clock::*;



// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

// Simple clock program for the cyd BSP, based on
// https://github.com/embedded-graphics/examples/blob/main/eg-0.8/examples/demo-analog-clock.rs
//
// The CYD's RTC peripheral is used to keep track of time, but has no means of finding the real time of day so initializes
// the time to 8:05am at start.   
 


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


    let rtc = Rtc::new(cyd_result.remainder.lpwr);
    


    rtc.set_current_time_us(((8 * 60) + 5) * 60 * 1000 * 1000); // set to 08:05:00.000
    let mut elapsed_ms: u32;

    let clock_color = Rgb565::GREEN;
    let bg_color = Rgb565::BLACK;
    let text_style = MonoTextStyle::new(&FONT_10X20, clock_color);

    cyd.display.clear(bg_color).unwrap();

    let clock_face = create_face(&cyd.display);
    draw_face(&mut cyd.display, &clock_face, clock_color).unwrap();

    let mut prev_hour = 0;
    let mut prev_minute = 0;
    let mut prev_second = 0;

    loop {

        let now = rtc.current_time_us() as i64;
        info!("now: {}", now);
        elapsed_ms = (now / 1000) as u32;
        info!("elapsed_ms: {}", elapsed_ms);
        let secs = elapsed_ms / 1000;
        let bedtime = 1000 * (secs + 1) - elapsed_ms;
        let nanos = now % 1_000_000;
        let time = NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos as u32).unwrap();

        let mut time_str: String<64> = String::new();
        write!(time_str, "Time: {:02}:{:02}:{:02}", time.hour(), time.minute(), time.second()).unwrap();

        info!("Time: {}", time_str.as_str());
        

        // erase previous text by drawing a filled rectangle behind the text area
        const TXT_POS: Point = Point::new(0, 0);
        const TXT_W: u32 = 220;
        const TXT_H: u32 = 40;
        let erase = Rectangle::new(TXT_POS, Size::new(TXT_W, TXT_H))
            .into_styled(PrimitiveStyle::with_fill(bg_color));
        erase.draw(&mut cyd.display).unwrap();

        let text = Text::new(time_str.as_str(), Point::new(0, 30), text_style);
        text.draw(&mut cyd.display).unwrap();

        let hour = time.hour();
        let minute = time.minute();
        let second = time.second();

        if hour != prev_hour {
             draw_hand(&mut cyd.display, &clock_face, bg_color, hour_to_angle(prev_hour), -60).unwrap();
        }
        if minute != prev_minute {
             draw_hand(&mut cyd.display, &clock_face, bg_color, sexagesimal_to_angle(prev_minute), -30).unwrap();
        }

        if second != prev_second {
            let seconds_radians = sexagesimal_to_angle(prev_second);
            draw_hand(&mut cyd.display, &clock_face, bg_color, seconds_radians, 0).unwrap();
            draw_second_decoration(&mut cyd.display, &clock_face, bg_color, bg_color, seconds_radians, -20).unwrap();
        }

        prev_hour = hour;
        prev_minute = minute;
        prev_second = second;

        draw_hand(&mut cyd.display, &clock_face, clock_color, hour_to_angle(hour), -60).unwrap();
        draw_hand(&mut cyd.display, &clock_face, clock_color, sexagesimal_to_angle(minute), -30).unwrap();

        let seconds_radians = sexagesimal_to_angle(second);
        draw_hand(&mut cyd.display, &clock_face, clock_color, seconds_radians, 0).unwrap();
        draw_second_decoration(&mut cyd.display, &clock_face, clock_color, bg_color, seconds_radians, -20).unwrap();

        // Draw a small circle over the hands in the center of the clock face.
        // This has to happen after the hands are drawn so they're covered up.
        Circle::with_center(clock_face.center(), 9)
            .into_styled(PrimitiveStyle::with_fill(clock_color))
            .draw(&mut cyd.display).unwrap();


        delay.delay_millis(bedtime);
    }
}
