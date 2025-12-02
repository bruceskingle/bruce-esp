# CYD-BSP

Rust Bare Metal Board Support Package (BSP) for the Cheap Yellow Display (CYD) or ESP32-2432S028R.

## List of Supported Boards
- [Random Nerd Tutorials Getting Started Guid](https://randomnerdtutorials.com/cheap-yellow-display-esp32-2432s028r/)
- [Random Nerd Tutorials Pinout Guid](https://randomnerdtutorials.com/esp32-cheap-yellow-display-cyd-pinout-esp32-2432s028r/)


## Usage

## Adding the BSP to Your Project

To add the ESP-BSP crate to your project:

```
cargo add cyd-bsp
```

### Board Initialization


```rust
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


    let fg_color = Rgb565::GREEN;
    let bg_color = Rgb565::BLACK;
    let text_style = MonoTextStyle::new(&FONT_10X20, fg_color);

    cyd.display.clear(bg_color).unwrap();

    let text = Text::new("Hello World", Point::new(0, 30), text_style);
    text.draw(&mut cyd.display).unwrap();

    loop {
        // Busy loop
    }
}
```
### Usage
The board initialization returns a ```CydResult``` which simply packages a ```cyd``` (the main BSP type) and a
```CydRemainder``` which contains all the unused pins and peripherals:

```rust
pub struct CydRemainder<'a> {
    pub gpio22: esp_hal::peripherals::GPIO22<'a>,
    pub gpio27: esp_hal::peripherals::GPIO27<'a>,
    pub gpio35: esp_hal::peripherals::GPIO35<'a>,
    pub lpwr: esp_hal::peripherals::LPWR<'a>,
    pub rmt: esp_hal::peripherals::RMT<'a>,
}
```

## Examples

- [hello_world.rs](examples/hello_world.rs) - Draw a message on the display
- [blinky.rs](examples/blinky.rs) - Blink the onboard RGB LED
- [clock](examples/clock/main.rs) - Animate a simple analog clock

## Changelog

### 0.1.0

- Initial release 