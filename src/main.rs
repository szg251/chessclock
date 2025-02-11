#![no_std]
#![no_main]

use defmt::{info, unwrap};
use effect::{Buzz, Effects};
use embassy_executor::Spawner;
use embassy_futures::{
    join::{join3, join4},
    select::{select, Either},
};
use embassy_stm32::{
    bind_interrupts,
    exti::ExtiInput,
    gpio::{Level, Output, OutputType, Pull, Speed},
    i2c::{ErrorInterruptHandler, EventInterruptHandler, I2c},
    peripherals::{I2C1, TIM1},
    time::Hertz,
    timer::{
        low_level::CountingMode,
        simple_pwm::{PwmPin, SimplePwm},
    },
};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    channel::{Channel, Receiver, Sender},
    signal::Signal,
};
use embassy_time::{Delay, Duration, Timer, WithTimeout};
use lcd_lcm1602_i2c::{async_lcd::Lcd, Backlight};
use {defmt_rtt as _, panic_probe as _};

use crate::app::{AppState, Button, Event, Page, PressType};
use crate::error::Error;
use crate::menu::GameConfig;

mod app;
mod aux;
mod effect;
mod error;
mod game;
mod menu;

bind_interrupts!(struct Irqs {
    I2C1_EV => EventInterruptHandler<I2C1>;
    I2C1_ER => ErrorInterruptHandler<I2C1>;
});

static CLOCK: Signal<ThreadModeRawMutex, bool> = Signal::new();
static BUZZ: Signal<ThreadModeRawMutex, Buzz> = Signal::new();

pub enum SystemEvent {
    SetClock(bool),
    Buzz(Buzz),
    Sleep(bool),
}

const SLEEP_TIME: u64 = 20;

struct Outputs<'a, 'b> {
    left_led: Output<'a>,
    right_led: Output<'a>,
    lcd: Lcd<'a, I2c<'b, embassy_stm32::mode::Async>, embassy_time::Delay>,
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let stm32_config = Default::default();
    let p = embassy_stm32::init(stm32_config);

    let config = embassy_stm32::i2c::Config::default();
    let scl = p.PB6;
    let sda = p.PB7;
    let mut i2c = I2c::new(
        p.I2C1,
        scl,
        sda,
        Irqs,
        p.DMA1_CH6,
        p.DMA1_CH7,
        Hertz::khz(20),
        config,
    );
    let mut delay = Delay;
    let lcd = unwrap!(
        Lcd::new(&mut i2c, &mut delay)
            .with_address(0x27)
            .with_cursor_on(false)
            .with_rows(2)
            .init()
            .await
    );

    let left_led = Output::new(p.PC13, Level::Low, Speed::Low);
    let right_led = Output::new(p.PC15, Level::Low, Speed::Low);
    let mut buzzer = SimplePwm::new(
        p.TIM1,
        Some(PwmPin::new_ch1(p.PA8, OutputType::PushPull)),
        None,
        None,
        None,
        Hertz::hz(440),
        CountingMode::default(),
    );

    let left_button = ExtiInput::new(p.PA0, p.EXTI0, Pull::Up);
    let right_button = ExtiInput::new(p.PA1, p.EXTI1, Pull::Up);
    let control_button = ExtiInput::new(p.PA2, p.EXTI2, Pull::Up);

    let sys_event_channel: Channel<ThreadModeRawMutex, SystemEvent, 3> = Channel::new();
    let sys_tx = sys_event_channel.sender();
    let sys_rx = sys_event_channel.receiver();

    let event_channel: Channel<ThreadModeRawMutex, Event, 3> = Channel::new();
    let tx = event_channel.sender();
    let rx = event_channel.receiver();

    let mut outputs = Outputs {
        left_led,
        right_led,
        lcd,
    };

    let _ = join4(
        main_loop(sys_tx, rx, &mut outputs),
        emit_clock(tx),
        join3(
            handle_button(tx, left_button, Button::Left),
            handle_button(tx, right_button, Button::Right),
            handle_button_with_waker(sys_rx, tx, control_button, Button::Control),
        ),
        handle_buzz(&mut buzzer),
    )
    .await;
}

async fn handle_button(
    tx: Sender<'_, ThreadModeRawMutex, Event, 3>,
    mut input: ExtiInput<'_>,
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

async fn handle_button_with_waker(
    sys_rx: Receiver<'_, ThreadModeRawMutex, SystemEvent, 3>,
    tx: Sender<'_, ThreadModeRawMutex, Event, 3>,
    mut input: ExtiInput<'_>,
    button: Button,
) {
    loop {
        match select(input.wait_for_low(), sys_rx.receive()).await {
            Either::First(_) => {
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
            Either::Second(SystemEvent::Sleep(true)) => {
                // let _waker = input.dormant_wake(DormantWakeConfig {
                //     edge_high: false,
                //     edge_low: true,
                //     level_high: false,
                //     level_low: false,
                // });
                // dormant_sleep();
            }
            Either::Second(_) => {}
        }
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
    sys_tx: Sender<'_, ThreadModeRawMutex, SystemEvent, 3>,
    rx: Receiver<'_, ThreadModeRawMutex, Event, 3>,
    outputs: &mut Outputs<'_, '_>,
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
    state.display_state(&init_state, outputs).await?;
    loop {
        let event = receive_event_or_sleep(sys_tx, rx, outputs, &state).await?;

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

        state.display_state(&prev_state, outputs).await?;
    }
}

async fn receive_event_or_sleep(
    sys_tx: Sender<'_, ThreadModeRawMutex, SystemEvent, 3>,
    rx: Receiver<'_, ThreadModeRawMutex, Event, 3>,
    outputs: &mut Outputs<'_, '_>,
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
                outputs.lcd.clear().await?;
                outputs.lcd.backlight(Backlight::Off).await?;
                outputs.left_led.set_low();
                outputs.right_led.set_low();

                sys_tx.send(SystemEvent::Sleep(true)).await;
                Timer::after_millis(10).await;

                state.display_state(state, outputs).await?;
                outputs.lcd.backlight(Backlight::On).await?;
            }
        }
    }
}

async fn handle_buzz(pwm: &mut SimplePwm<'_, TIM1>) -> Result<(), Error> {
    loop {
        let buzz = BUZZ.wait().await;
        pwm.set_frequency(Hertz::hz(buzz.freq));
        let mut buzzer = pwm.ch1();

        buzzer.set_duty_cycle_fully_on();

        Timer::after(buzz.duration).await;
        buzzer.set_duty_cycle_fully_off();
    }
}
