use core::fmt::Write;

use embassy_rp::pwm;
use embassy_time::Duration;
use heapless::String;

use crate::error::Error;

pub fn format_duration(duration: Duration) -> Result<String<5>, Error> {
    format_secs(duration.as_secs())
}

pub fn format_secs(secs: u64) -> Result<String<5>, Error> {
    let mut out = String::new();
    write!(&mut out, "{:02}:{:02}", secs / 60, secs % 60)?;
    Ok(out)
}

pub trait CeilTime {
    fn ceil_secs(&self) -> u64;
}

impl CeilTime for Duration {
    fn ceil_secs(&self) -> u64 {
        let subsec_micros = self.as_micros() % 1_000_000;
        self.as_secs() + if subsec_micros > 0 { 1 } else { 0 }
    }
}

pub fn pwm_freq_config(freq: u32) -> pwm::Config {
    let clock_freq_hz = embassy_rp::clocks::clk_sys_freq();
    let divider = 16u8;
    let period = (clock_freq_hz / (freq * divider as u32)) as u16 - 1;
    let mut c = pwm::Config::default();
    c.top = period;
    c.divider = divider.into();
    c
}
