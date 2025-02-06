#![no_std]
#![no_main]

use defmt::info;
use effect::{Buzz, Effects};
use embassy_executor::Spawner;
use embassy_futures::join::{join3, join4};
use embassy_rp::{
    bind_interrupts,
    gpio::{Input, Level, Output},
    i2c::{self, I2c, InterruptHandler as I2cInterruptHandler},
    peripherals::I2C1,
    pwm::{self, Pwm, SetDutyCycle},
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, Receiver, Sender},
    signal::Signal,
};
use embassy_time::{Delay, Duration, Timer, WithTimeout};
use lcd_lcm1602_i2c::{sync_lcd::Lcd, Backlight};
use {defmt_rtt as _, panic_probe as _};

use crate::app::{AppState, Button, Event, Page, PressType};
use crate::aux::pwm_freq_config;
use crate::error::Error;
use crate::menu::GameConfig;

mod app;
mod aux;
mod effect;
mod error;
mod game;
mod menu;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => I2cInterruptHandler<I2C1>;
});

static CLOCK: Signal<ThreadModeRawMutex, bool> = Signal::new();
static BUZZ: Signal<ThreadModeRawMutex, Buzz> = Signal::new();

const SLEEP_TIME: u64 = 60;

struct Outputs<'a> {
    left_led: Output<'a>,
    right_led: Output<'a>,
    lcd: Lcd<'a, I2c<'a, I2C1, i2c::Async>, embassy_time::Delay>,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let config = embassy_rp::i2c::Config::default();
    let scl = p.PIN_27;
    let sda = p.PIN_26;
    let mut i2c = I2c::new_async(p.I2C1, scl, sda, Irqs, config);
    let mut delay = Delay;
    let lcd = Lcd::new(&mut i2c, &mut delay)
        .with_address(0x27)
        .with_cursor_on(false)
        .with_rows(2)
        .init()
        .unwrap();

    let left_led = Output::new(p.PIN_1, Level::Low);
    let right_led = Output::new(p.PIN_2, Level::Low);
    let mut buzzer = Pwm::new_output_a(p.PWM_SLICE6, p.PIN_12, pwm::Config::default());

    let mut left_button = Input::new(p.PIN_3, embassy_rp::gpio::Pull::Up);
    let mut right_button = Input::new(p.PIN_4, embassy_rp::gpio::Pull::Up);
    let mut control_button = Input::new(p.PIN_5, embassy_rp::gpio::Pull::Up);

    let event_channel: Channel<ThreadModeRawMutex, Event, 3> = Channel::new();
    let tx = event_channel.sender();
    let rx = event_channel.receiver();

    let mut outputs = Outputs {
        left_led,
        right_led,
        lcd,
    };

    let _ = join4(
        main_loop(rx, &mut outputs),
        emit_clock(tx),
        join3(
            handle_button(tx, &mut left_button, Button::Left),
            handle_button(tx, &mut right_button, Button::Right),
            handle_button(tx, &mut control_button, Button::Control),
        ),
        handle_buzz(&mut buzzer),
    )
    .await;

    // unwrap!(resA);
}

async fn handle_button(
    tx: Sender<'_, ThreadModeRawMutex, Event, 3>,
    input: &mut Input<'_>,
    button: Button,
) {
    loop {
        input.wait_for_low().await;
        let instant = embassy_time::Instant::now();
        Timer::after_millis(200).await;

        input.wait_for_high().await;
        let press_type = if instant.elapsed() > Duration::from_millis(300) {
            PressType::Long
        } else {
            PressType::Single
        };

        tx.send(Event::ButtonPushed(button, press_type)).await;
        Timer::after_millis(100).await;
    }
}

async fn emit_clock(tx: Sender<'_, ThreadModeRawMutex, Event, 3>) {
    loop {
        if CLOCK.wait().await {
            loop {
                let duration = Duration::from_millis(1000);
                let clock = CLOCK.wait().with_timeout(duration).await;

                if let Ok(false) = clock {
                    break;
                }

                tx.send(Event::Clock(duration)).await;
            }
        }
    }
}

async fn main_loop(
    rx: Receiver<'_, ThreadModeRawMutex, Event, 3>,
    outputs: &mut Outputs<'_>,
) -> Result<(), Error> {
    info!("Init");

    let test_duration = Duration::from_millis(300);
    outputs.left_led.set_high();
    outputs.right_led.set_high();
    BUZZ.signal(Buzz {
        freq: 440,
        duration: test_duration,
    });

    Timer::after(test_duration).await;

    outputs.left_led.set_low();
    outputs.right_led.set_low();

    let init_state = AppState {
        game_config: GameConfig::default(),
        page: Page::Init,
    };
    let mut state = AppState {
        game_config: GameConfig::default(),
        page: Page::Welcome,
    };
    state.display_state(&init_state, outputs)?;
    loop {
        let event = receive_event_or_sleep(rx, outputs, &state).await?;

        let prev_state = state.clone();

        let mut effects = Effects::new();
        state.handle_event(&mut effects, event)?;

        if let Some(buzz) = effects.buzz {
            info!("Buzz effect");
            BUZZ.signal(buzz);
        }

        if let Some(page) = effects.page_change {
            state.page = page
        }

        if let Some(clock) = effects.set_clock {
            CLOCK.signal(clock);
        }

        state.display_state(&prev_state, outputs)?;
    }
}

async fn receive_event_or_sleep(
    rx: Receiver<'_, ThreadModeRawMutex, Event, 3>,
    outputs: &mut Outputs<'_>,
    state: &AppState,
) -> Result<Event, Error> {
    let time_until_sleep = Duration::from_secs(SLEEP_TIME);
    let mut event;
    loop {
        event = rx.receive().with_timeout(time_until_sleep).await;

        // Sleep after 1 minute of inactivity
        match event {
            Ok(event) => {
                return Ok(event);
            }
            Err(_) => {
                outputs.lcd.backlight(Backlight::Off)?;
                outputs.left_led.set_low();
                outputs.right_led.set_low();
                let _ = rx.receive().await;

                state.display_state(state, outputs)?;
                outputs.lcd.backlight(Backlight::On)?;
            }
        }
    }
}

async fn handle_buzz(buzzer: &mut Pwm<'_>) -> Result<(), Error> {
    loop {
        let buzz = BUZZ.wait().await;
        buzzer.set_config(&pwm_freq_config(buzz.freq));
        buzzer.set_duty_cycle_fully_on().unwrap();

        Timer::after(buzz.duration).await;
        buzzer.set_duty_cycle_fully_off().unwrap();
    }
}
