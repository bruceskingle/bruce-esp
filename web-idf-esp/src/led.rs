use std::sync::{Arc, Mutex};
use log::info;

use esp_idf_hal::{gpio::PinDriver, ledc::LedcDriver};
use esp_idf_hal::units::*;

pub struct LedManager<'a> {
    led_timer_driver: esp_idf_hal::ledc::LedcTimerDriver<'a, esp_idf_hal::ledc::LowSpeed>,
    led_channel_red: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
    led_channel_green: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
    led_channel_blue: Arc<Mutex<esp_idf_hal::ledc::LedcDriver<'a>>>,
}

impl<'a> LedManager<'a> {
    pub fn new(
        timer0: esp_idf_hal::ledc::TIMER0<'static>,
        channel0: esp_idf_hal::ledc::CHANNEL0<'static>,
        gpio4: esp_idf_hal::gpio::Gpio4<'static>,
        channel1: esp_idf_hal::ledc::CHANNEL1<'static>,
        gpio16: esp_idf_hal::gpio::Gpio16<'static>,
        channel2: esp_idf_hal::ledc::CHANNEL2<'static>,
        gpio17: esp_idf_hal::gpio::Gpio17<'static>
    ) -> anyhow::Result<Self> {
        let led_timer_driver = esp_idf_hal::ledc::LedcTimerDriver::new(timer0,
            &esp_idf_hal::ledc::config::TimerConfig::new().frequency(1000.Hz()))?;
    
        let led_channel_red = Arc::new(Mutex::new(LedcDriver::new(channel0, &led_timer_driver, gpio4)?));
        let led_channel_green = Arc::new(Mutex::new(LedcDriver::new(channel1, &led_timer_driver, gpio16)?));
        let led_channel_blue = Arc::new(Mutex::new(LedcDriver::new(channel2, &led_timer_driver, gpio17)?));

        Ok(Self { 
            led_timer_driver,
            led_channel_red,
            led_channel_green,
            led_channel_blue,
         })
    }

    pub fn set_color(&self, r: u8, g: u8, b: u8) -> anyhow::Result<()> {
        info!("Set led color r={} g={} b={}", r, g, b);
        self.led_channel_red.lock().unwrap().set_duty(r as u32)?;
        self.led_channel_green.lock().unwrap().set_duty(g as u32)?;
        self.led_channel_blue.lock().unwrap().set_duty(b as u32)?;
        Ok(())
    }
}