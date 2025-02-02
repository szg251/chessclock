use embassy_time::Duration;

use crate::app::Page;

pub struct Effects {
    pub set_clock: Option<bool>,
    pub buzz: Option<Buzz>,
    pub page_change: Option<Page>,
}

pub struct Buzz {
    pub freq: u32,
    pub duration: Duration,
}

impl Effects {
    pub fn new() -> Self {
        Self {
            set_clock: None,
            buzz: None,
            page_change: None,
        }
    }
    pub fn set_clock(&mut self, clock: bool) {
        self.set_clock = Some(self.set_clock.map(|c| c || clock).unwrap_or(clock));
        if let Some(clock2) = self.set_clock {
            self.set_clock = Some(clock || clock2);
        } else {
            self.set_clock = Some(clock)
        }
    }

    pub fn buzz(&mut self, freq: u32, duration: Duration) {
        if let Some(ref buzz) = self.buzz {
            if freq > buzz.freq {
                self.buzz = Some(Buzz { freq, duration })
            }
        } else {
            self.buzz = Some(Buzz { freq, duration })
        }
    }

    pub fn page_change(&mut self, page: Page) {
        if let None = self.page_change {
            self.page_change = Some(page);
        }
    }
}
