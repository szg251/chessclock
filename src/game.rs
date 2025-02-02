use defmt::info;
use embassy_time::Duration;

use crate::{
    app::{Button, Event, Page, PressType},
    aux::{format_secs, CeilTime},
    effect::Effects,
    error::Error,
    menu::{GameConfig, IncrementType},
    Outputs,
};

#[derive(Clone)]
pub struct GameState {
    pub turn: Player,
    pub left_time: Duration,
    pub right_time: Duration,
    pub paused: bool,
    pub delay: Duration,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Left,
    Right,
}

impl GameState {
    pub fn new(game_config: &GameConfig, first_player: Player) -> GameState {
        let delay = match game_config.increment_type {
            IncrementType::Delay {
                left_delay,
                right_delay,
            } => match first_player {
                Player::Left => left_delay,
                Player::Right => right_delay,
            },
            IncrementType::Bronstein {
                left_delay,
                right_delay,
            } => match first_player {
                Player::Left => left_delay,
                Player::Right => right_delay,
            },
            _ => Duration::from_ticks(0),
        };
        Self {
            turn: first_player,
            left_time: game_config.left_time,
            right_time: game_config.right_time,
            paused: true,
            delay,
        }
    }

    pub fn handle_event(&mut self, effects: &mut Effects, game_config: &GameConfig, event: &Event) {
        match event {
            Event::ButtonPushed(Button::Left, _) => {
                if self.paused {
                    self.paused = false;
                } else if self.turn == Player::Left {
                    match game_config.increment_type {
                        IncrementType::SuddenDeath { .. } => self.delay = Duration::from_ticks(0),
                        IncrementType::Increment { left_increment, .. } => {
                            self.left_time += left_increment;
                            self.delay = Duration::from_ticks(0)
                        }
                        IncrementType::Delay { right_delay, .. } => {
                            self.delay = right_delay;
                        }
                        IncrementType::Bronstein {
                            left_delay,
                            right_delay,
                        } => {
                            self.left_time += left_delay - self.delay;
                            self.delay = right_delay;
                        }
                    }
                    self.turn = Player::Right;
                    info!("Right's turn")
                }
            }
            Event::ButtonPushed(Button::Right, _) => {
                if self.paused {
                    self.paused = false;
                } else if self.turn == Player::Right {
                    match game_config.increment_type {
                        IncrementType::SuddenDeath { .. } => self.delay = Duration::from_ticks(0),
                        IncrementType::Increment {
                            right_increment, ..
                        } => {
                            self.right_time += right_increment;
                            self.delay = Duration::from_ticks(0)
                        }
                        IncrementType::Delay { left_delay, .. } => {
                            self.delay = left_delay;
                        }
                        IncrementType::Bronstein {
                            left_delay,
                            right_delay,
                        } => {
                            self.right_time += right_delay - self.delay;
                            self.delay = left_delay;
                        }
                    }
                    self.turn = Player::Left;
                    info!("Left's turn")
                }
            }
            Event::ButtonPushed(Button::Control, PressType::Single) => {
                self.paused = !self.paused;
                info!("Pause: {}", self.paused);
            }
            Event::ButtonPushed(Button::Control, PressType::Long) => {}
            Event::Clock(duration) => {
                if !self.paused {
                    if !self.delay.as_ticks() == 0 {
                        match game_config.increment_type {
                            IncrementType::Delay { .. } => {
                                self.delay -= *duration;
                            }
                            IncrementType::Bronstein { .. } => {
                                self.delay -= *duration;
                                self.decrement_time(effects, duration);
                            }
                            _ => {}
                        }
                    } else {
                        self.decrement_time(effects, &duration);
                    }
                }
            }
        }
    }

    fn decrement_time(&mut self, effects: &mut Effects, duration: &Duration) {
        let prev_left_time = self.left_time;
        let prev_right_time = self.right_time;
        match self.turn {
            Player::Left => self.left_time -= *duration,
            Player::Right => self.right_time -= *duration,
        }

        let high_beep = [
            Duration::from_secs(60),
            Duration::from_secs(10),
            Duration::from_secs(5),
            Duration::from_secs(4),
            Duration::from_secs(3),
            Duration::from_secs(2),
            Duration::from_secs(1),
        ]
        .iter()
        .any(|set_time| {
            time_passing(set_time, &prev_left_time, &self.left_time)
                || time_passing(set_time, &prev_right_time, &self.right_time)
        });
        let low_beep = time_passing(&Duration::from_secs(0), &prev_left_time, &self.left_time)
            || time_passing(&Duration::from_secs(0), &prev_right_time, &self.right_time);

        if high_beep {
            effects.buzz(880, Duration::from_millis(100));
        } else if low_beep {
            effects.buzz(440, Duration::from_millis(500));
        }

        if self.left_time.as_ticks() == 0 {
            effects.page_change(Page::GameOver(Player::Left))
        } else if self.right_time.as_ticks() == 0 {
            effects.page_change(Page::GameOver(Player::Right))
        }
    }

    pub fn display_state(
        &self,
        prev_state: Option<&GameState>,
        outputs: &mut Outputs,
    ) -> Result<(), Error> {
        if Some(self.turn) != prev_state.map(|s| s.turn) {
            match self.turn {
                Player::Left => {
                    outputs.left_led.set_high();
                    outputs.right_led.set_low();
                }
                Player::Right => {
                    outputs.left_led.set_low();
                    outputs.right_led.set_high();
                }
            }
        }

        let prev_left_secs = prev_state.map(|s| s.left_time.ceil_secs());
        let left_secs = self.left_time.ceil_secs();

        if prev_left_secs.is_none() || prev_left_secs != Some(left_secs) {
            outputs.lcd.set_cursor(0, 0)?;
            outputs.lcd.write_str(format_secs(left_secs)?.as_str())?;
        }

        let prev_right_secs = prev_state.map(|s| s.right_time.ceil_secs());
        let right_secs = self.right_time.ceil_secs();

        if prev_right_secs.is_none() || prev_right_secs != Some(right_secs) {
            outputs.lcd.set_cursor(0, 11)?;
            outputs.lcd.write_str(format_secs(right_secs)?.as_str())?;
        }

        if prev_state.map(|s| s.paused) != Some(self.paused) {
            outputs.lcd.set_cursor(1, 5)?;
            if self.paused {
                outputs.lcd.write_str("paused")?;
            } else {
                outputs.lcd.write_str("      ")?;
            }
        }
        Ok(())
    }
}

fn time_passing(set_time: &Duration, prev_time: &Duration, time: &Duration) -> bool {
    prev_time > set_time && set_time >= time
}
