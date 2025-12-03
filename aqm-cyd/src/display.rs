use cyd_bsp::Cyd;

use core::fmt::Write;
use crate::sense;
use heapless::String;



use embedded_graphics::text::Text;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_8X13};
use embedded_graphics::{
    primitives::{PrimitiveStyle, Rectangle},pixelcolor::Rgb565, prelude::*};

/// Swaps RED and BLUE channels for display color correction (BGR vs RGB mismatch)
// fn swap_rb(color: Rgb565) -> Rgb565 {
//     Rgb565::new(color.b(), color.g(), color.r())
// }

    const FG_COLOR: Rgb565 = Rgb565::GREEN;
    const BG_COLOR: Rgb565 = Rgb565::BLACK;
    const LABEL_STYLE: MonoTextStyle<'static, Rgb565> = MonoTextStyle::new(&FONT_8X13, FG_COLOR);
    const VALUE_STYLE: MonoTextStyle<'static, Rgb565> = MonoTextStyle::new(&FONT_10X20, FG_COLOR);

struct DisplayItem {
    position: Point,
    erase: embedded_graphics::primitives::Styled<Rectangle, PrimitiveStyle<Rgb565>>,
    label: &'static str,
    // format: &'static str,
    values: [String<16>; 2],
    current_value: usize,
}

impl DisplayItem {
    fn new(cyd: &mut Cyd<'static>, 
    x: i32,
    y: i32,
    label: &'static str) -> Self {
        let text = Text::new(label, Point::new(x, y + 13), LABEL_STYLE);
        text.draw(&mut cyd.display).unwrap();
        let position = Point::new(x, y + 18 + 20);
        let rect = Rectangle::new( Point::new(x, y + 18), Size::new(100, 50-18));
        let erase = rect
            .into_styled(PrimitiveStyle::with_fill(
                // Rgb565::RED
                BG_COLOR
            ));
        Self {
            position,
            erase,
            label,
            // format,
            values: [String::new(), String::new()],
            current_value: 0,
        }
    }

    fn update_value<T: core::fmt::Display>(&mut self, cyd: &mut Cyd<'static>, val: T) {
        let old_value = if self.current_value == 1 { 0 } else { 1 };

        self.values[old_value].clear();
        write!(self.values[old_value], "{:.1}", val).unwrap();

        if self.values[old_value] != self.values[self.current_value] {
            self.current_value = old_value;
            defmt::info!("display_item {}: {}", self.label, self.values[self.current_value].as_str());
            self.erase.draw(&mut cyd.display).unwrap();
            let text = Text::new(self.values[self.current_value].as_str(), self.position, VALUE_STYLE);
            text.draw(&mut cyd.display).unwrap();
        }
        
    }
}

#[embassy_executor::task]
pub async fn display_task(mut cyd: Cyd<'static>) {
    let mut rx = sense::get_receiver().unwrap();

    let mut temperature = DisplayItem::new(&mut cyd, 10, 50, "Temperature");
    let mut pressure = DisplayItem::new(&mut cyd, 10, 100, "Pressure");
    let mut humidity = DisplayItem::new(&mut cyd, 10, 150, "Humidity");
    loop {
        let sensor_data = rx.changed().await;
        defmt::info!("DISP Temperature: {}", sensor_data.temperature);

        let mut time_str: String<64> = String::new();
        write!(time_str, "T={:.1}C ", sensor_data.temperature).unwrap();

        temperature.update_value(&mut cyd, sensor_data.temperature);
        pressure.update_value(&mut cyd, sensor_data.pressure/100.0);
        humidity.update_value(&mut cyd, sensor_data.humidity);
    }
}