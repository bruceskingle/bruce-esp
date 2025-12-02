//! A simple analog clock example using embedded-graphics.
//! Copied from https://github.com/embedded-graphics/examples/blob/main/eg-0.8/examples/demo-analog-clock.rs

use core::f32::consts::PI;
use embedded_graphics::{
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, PrimitiveStyleBuilder},
};
use micromath::F32Ext;
const MARGIN: u32 = 10;

/// Converts a polar coordinate (angle/distance) into an (X, Y) coordinate centered around the
/// center of the circle.
///
/// The angle is relative to the 12 o'clock position and the radius is relative to the edge of the
/// clock face.
pub fn polar(circle: &Circle, angle: f32, radius_delta: i32) -> Point {
    let radius = circle.diameter as f32 / 2.0 + radius_delta as f32;

    circle.center()
        + Point::new(
            (angle.sin() * radius) as i32,
            -(angle.cos() * radius) as i32,
        )
}

/// Converts an hour into an angle in radians.
pub fn hour_to_angle(hour: u32) -> f32 {
    // Convert from 24 to 12 hour time.
    let hour = hour % 12;

    (hour as f32 / 12.0) * 2.0 * PI
}

/// Converts a sexagesimal (base 60) value into an angle in radians.
pub fn sexagesimal_to_angle(value: u32) -> f32 {
    (value as f32 / 60.0) * 2.0 * PI
}

/// Creates a centered circle for the clock face.
pub fn create_face(target: &impl DrawTarget) -> Circle {
    // The draw target bounding box can be used to determine the size of the display.
    let bounding_box = target.bounding_box();

    let diameter = bounding_box.size.width.min(bounding_box.size.height) - 2 * MARGIN;

    Circle::with_center(bounding_box.center(), diameter)
}

/// Draws a circle and 12 graduations as a simple clock face.
pub fn draw_face<D,C>(target: &mut D, clock_face: &Circle, stroke_color: C) -> Result<(), D::Error>
where
    C: PixelColor,
    D: DrawTarget<Color = C>,
{
    // Draw the outer face.
    let style = PrimitiveStyle::with_stroke(stroke_color, 2);
    (*clock_face)
        .into_styled(style)
        .draw(target)?;

    // Draw 12 graduations.
    for angle in (0..12).map(hour_to_angle) {
        // Start point on circumference.
        let start = polar(clock_face, angle, 0);

        // End point offset by 10 pixels from the edge.
        let end = polar(clock_face, angle, -10);

        Line::new(start, end)
            .into_styled(style)
            .draw(target)?;
    }

    Ok(())
}

/// Draws a clock hand.
pub fn draw_hand<D,C>(
    target: &mut D,
    clock_face: &Circle,
    stroke_color: C,
    angle: f32,
    length_delta: i32,
) -> Result<(), D::Error>
where
    C: PixelColor,
    D: DrawTarget<Color = C>,
{
    let end = polar(clock_face, angle, length_delta);

    Line::new(clock_face.center(), end)
        .into_styled(PrimitiveStyle::with_stroke(stroke_color, 1))
        .draw(target)
}

/// Draws a decorative circle on the second hand.
pub fn draw_second_decoration<D,C>(
    target: &mut D,
    clock_face: &Circle,
    stroke_color: C,
    bg_color: C,
    angle: f32,
    length_delta: i32,
) -> Result<(), D::Error>
where
    C: PixelColor,
    D: DrawTarget<Color = C>,
{
    let decoration_position = polar(clock_face, angle, length_delta);

    let decoration_style = PrimitiveStyleBuilder::new()
        .fill_color(bg_color)
        .stroke_color(stroke_color)
        .stroke_width(1)
        .build();

    // Draw a fancy circle near the end of the second hand.
    Circle::with_center(decoration_position, 11)
        .into_styled(decoration_style)
        .draw(target)
}