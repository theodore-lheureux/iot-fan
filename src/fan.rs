use log::info;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Speed {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl Speed {
    pub fn to_string(&self) -> String {
        match self {
            Speed::Low => "low_key".to_string(),
            Speed::Medium => "med_key".to_string(),
            Speed::High => "high_key".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Fan {
    pub speed: Speed,
    on: bool,
}

impl Fan {
    pub fn new() -> Fan {
        let speed = Speed::Low;
        info!("Fan initialized at : {:?}", speed);
        Fan { speed, on: false }
    }
    pub fn set_speed(&mut self, speed: Speed) {
        self.speed = speed;
        info!("Fan speed: {:?}", self.get_speed());
    }
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        info!("Fan on: {:?}", self.is_on());
    }
    pub fn toggle(&mut self) {
        self.on = !self.on;
        info!("Fan on: {:?}", self.is_on());
    }
    pub fn get_speed(&self) -> Speed {
        self.speed
    }
    pub fn next_speed(&mut self) {
        match self.speed {
            Speed::Low => {
                if self.is_on() {
                    self.set_speed(Speed::Medium);
                } else {
                    self.on = true;
                }
            },
            Speed::Medium => self.set_speed(Speed::High),
            Speed::High => { 
                self.set_speed(Speed::Low);
                self.on = false
            },
        }
    }
    pub fn is_on(&self) -> bool {
        self.on
    }
}
