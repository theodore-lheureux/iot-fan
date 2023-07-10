use anyhow::Result;
use esp_idf_hal::gpio::{Gpio25, Gpio26, Gpio32, Gpio33, Output, PinDriver};
use esp_idf_sys::EspError;

use crate::fan::{Fan, Speed};

pub struct LEDs<'a> {
    pub on_off: PinDriver<'a, Gpio32, Output>,
    pub speed_1: PinDriver<'a, Gpio33, Output>,
    pub speed_2: PinDriver<'a, Gpio25, Output>,
    pub speed_3: PinDriver<'a, Gpio26, Output>,
}

impl LEDs<'_> {
    pub fn new(
        gpio32: Gpio32,
        gpio33: Gpio33,
        gpio25: Gpio25,
        gpio26: Gpio26,
    ) -> Result<LEDs<'static>, EspError> {
        let mut speed_1 = PinDriver::output(gpio33)?;
        let mut speed_2 = PinDriver::output(gpio25)?;
        let mut speed_3 = PinDriver::output(gpio26)?;
        // Must be initialized last to avoid turning on the fan for a brief moment
        let mut on_off = PinDriver::output(gpio32)?;
        
        speed_1.set_low()?;
        speed_2.set_low()?;
        speed_3.set_low()?;
        on_off.set_high()?;

        Ok(LEDs {
            on_off,
            speed_1,
            speed_2,
            speed_3,
        })
    }
    pub fn update_status(&mut self, fan: &Fan) -> Result<()> {
        // avoid flickering by setting all pins low before setting the correct pins high
        if fan.is_on() {
            self.on_off.set_low()?;
            match fan.get_speed() {
                Speed::Low => {
                    self.speed_1.set_high()?;
                    self.speed_2.set_low()?;
                    self.speed_3.set_low()?;
                }
                Speed::Medium => {
                    self.speed_1.set_low()?;
                    self.speed_2.set_high()?;
                    self.speed_3.set_low()?;
                }
                Speed::High => {
                    self.speed_1.set_low()?;
                    self.speed_2.set_low()?;
                    self.speed_3.set_high()?;
                }
            }
        } else {
            self.on_off.set_high()?;
            self.speed_1.set_low()?;
            self.speed_2.set_low()?;
            self.speed_3.set_low()?;
        }
        Ok(())
    }
}
