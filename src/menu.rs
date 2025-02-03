use embassy_time::Duration;
use heapless::Vec;

use crate::{
    app::{Button, Event, PressType},
    aux::format_duration,
    error::Error,
    Outputs,
};

#[derive(Clone, PartialEq, Eq)]
pub enum MenuItem {
    Preset,
    LeftTime,
    RightTime,
    IncrementType,
    LeftDelay,
    RightDelay,
}

struct Cursor {
    position: u8,
    multiplier: u64,
}

impl Cursor {
    fn new(position: u8, multiplier: u64) -> Cursor {
        Cursor {
            position,
            multiplier,
        }
    }
}

impl MenuItem {
    /// Returns a vector of columns with their respective cursor position on the display
    fn cols(&self) -> Vec<Cursor, 2> {
        let mut columns = Vec::new();
        match self {
            MenuItem::Preset => {
                let _ = columns.push(Cursor::new(0, 1));
            }
            MenuItem::LeftTime => {
                let _ = columns.push(Cursor::new(1, 60));
                let _ = columns.push(Cursor::new(4, 1));
            }
            MenuItem::RightTime => {
                let _ = columns.push(Cursor::new(1, 60));
                let _ = columns.push(Cursor::new(4, 1));
            }
            MenuItem::IncrementType => {
                let _ = columns.push(Cursor::new(0, 1));
            }
            MenuItem::LeftDelay => {
                let _ = columns.push(Cursor::new(1, 60));
                let _ = columns.push(Cursor::new(4, 1));
            }
            MenuItem::RightDelay => {
                let _ = columns.push(Cursor::new(1, 60));
                let _ = columns.push(Cursor::new(4, 1));
            }
        }
        columns
    }

    /// Returns the maximum value for the menu item
    fn max_val(&self) -> u64 {
        match self {
            MenuItem::Preset => PRESETS.len() as u64 - 1,
            MenuItem::LeftTime => 3599,
            MenuItem::RightTime => 3599,
            MenuItem::IncrementType => INCREMENT_TYPES.len() as u64 - 1,
            MenuItem::LeftDelay => 59,
            MenuItem::RightDelay => 59,
        }
    }

    fn edit(&self, game_config: &mut GameConfig, edit_fn: impl Fn(u64) -> u64) {
        match self {
            MenuItem::Preset => {
                let idx = PRESETS.iter().enumerate().find_map(|(idx, (_, preset))| {
                    if preset == game_config {
                        Some(idx)
                    } else {
                        None
                    }
                });

                match idx {
                    None => *game_config = PRESETS[0].1.clone(),
                    Some(idx) => *game_config = PRESETS[edit_fn(idx as u64) as usize].1.clone(),
                }
            }
            MenuItem::LeftTime => {
                game_config.left_time =
                    Duration::from_secs(edit_fn(game_config.left_time.as_secs()));
            }
            MenuItem::RightTime => {
                game_config.right_time =
                    Duration::from_secs(edit_fn(game_config.right_time.as_secs()));
            }
            MenuItem::IncrementType => {
                let idx = match game_config.increment_type {
                    IncrementType::SuddenDeath => 0,
                    IncrementType::Increment { .. } => 1,
                    IncrementType::Delay { .. } => 2,
                    IncrementType::Bronstein { .. } => 3,
                };
                game_config.increment_type = INCREMENT_TYPES[edit_fn(idx) as usize].clone();
            }
            MenuItem::LeftDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment {
                    ref mut left_increment,
                    ..
                } => {
                    *left_increment = Duration::from_secs(edit_fn(left_increment.as_secs()));
                }
                IncrementType::Delay {
                    ref mut left_delay, ..
                } => {
                    *left_delay = Duration::from_secs(edit_fn(left_delay.as_secs()));
                }
                IncrementType::Bronstein {
                    ref mut left_delay, ..
                } => {
                    *left_delay = Duration::from_secs(edit_fn(left_delay.as_secs()));
                }
            },
            MenuItem::RightDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment {
                    ref mut right_increment,
                    ..
                } => {
                    *right_increment = Duration::from_secs(edit_fn(right_increment.as_secs()));
                }
                IncrementType::Delay {
                    ref mut right_delay,
                    ..
                } => {
                    *right_delay = Duration::from_secs(edit_fn(right_delay.as_secs()));
                }
                IncrementType::Bronstein {
                    ref mut right_delay,
                    ..
                } => {
                    *right_delay = Duration::from_secs(edit_fn(right_delay.as_secs()));
                }
            },
        }
    }
}

const PRESETS: [(&str, GameConfig); 2] = [
    (
        "Right handicap",
        GameConfig {
            left_time: Duration::from_secs(600),
            right_time: Duration::from_secs(15),
            increment_type: IncrementType::Bronstein {
                left_delay: Duration::from_secs(15),
                right_delay: Duration::from_secs(15),
            },
        },
    ),
    (
        "Left handicap",
        GameConfig {
            left_time: Duration::from_secs(15),
            right_time: Duration::from_secs(600),
            increment_type: IncrementType::Bronstein {
                left_delay: Duration::from_secs(15),
                right_delay: Duration::from_secs(15),
            },
        },
    ),
];

const MENU_ITEMS: [MenuItem; 6] = [
    MenuItem::Preset,
    MenuItem::LeftTime,
    MenuItem::RightTime,
    MenuItem::IncrementType,
    MenuItem::LeftDelay,
    MenuItem::RightDelay,
];

const INCREMENT_TYPES: [IncrementType; 4] = [
    IncrementType::SuddenDeath,
    IncrementType::Increment {
        left_increment: Duration::from_secs(10),
        right_increment: Duration::from_secs(10),
    },
    IncrementType::Delay {
        left_delay: Duration::from_secs(10),
        right_delay: Duration::from_secs(10),
    },
    IncrementType::Bronstein {
        left_delay: Duration::from_secs(10),
        right_delay: Duration::from_secs(10),
    },
];

#[derive(Clone, PartialEq, Eq)]
pub struct MenuState {
    item_index: usize,
    edit_mode: EditState,
}

#[derive(Clone, PartialEq, Eq)]
enum EditState {
    NotEditing,
    Cursor(usize),
    Editing(usize),
}

impl MenuState {
    pub fn new() -> MenuState {
        MenuState {
            item_index: 0,
            edit_mode: EditState::NotEditing,
        }
    }

    pub fn handle_event(&mut self, game_config: &mut GameConfig, event: &Event) {
        let mut disabled: Vec<MenuItem, 5> = Vec::new();
        if matches!(game_config.increment_type, IncrementType::SuddenDeath) {
            let _ = disabled.push(MenuItem::LeftDelay);
            let _ = disabled.push(MenuItem::RightDelay);
        };
        match self.edit_mode {
            EditState::NotEditing => match event {
                Event::ButtonPushed(Button::Left, _) => loop {
                    self.item_index = match self.item_index {
                        0 => MENU_ITEMS.len() - 1,
                        _ => self.item_index - 1,
                    };

                    if disabled
                        .iter()
                        .all(|disabled| &MENU_ITEMS[self.item_index] != disabled)
                    {
                        break;
                    }
                },
                Event::ButtonPushed(Button::Right, _) => loop {
                    self.item_index = (self.item_index + 1) % MENU_ITEMS.len();
                    if disabled
                        .iter()
                        .all(|disabled| &MENU_ITEMS[self.item_index] != disabled)
                    {
                        break;
                    }
                },
                Event::ButtonPushed(Button::Control, PressType::Single) => {
                    if MENU_ITEMS[self.item_index].cols().len() > 1 {
                        self.edit_mode = EditState::Cursor(0)
                    } else {
                        self.edit_mode = EditState::Editing(0)
                    }
                }
                _ => {}
            },
            EditState::Cursor(col) => match event {
                Event::ButtonPushed(Button::Left, _) => {
                    let col = match col {
                        0 => MENU_ITEMS[self.item_index].cols().len() - 1,
                        _ => col - 1,
                    };
                    self.edit_mode = EditState::Cursor(col);
                }
                Event::ButtonPushed(Button::Right, _) => {
                    let col = (col + 1) % MENU_ITEMS[self.item_index].cols().len();
                    self.edit_mode = EditState::Cursor(col);
                }
                Event::ButtonPushed(Button::Control, PressType::Single) => {
                    self.edit_mode = EditState::Editing(col)
                }
                _ => {}
            },
            EditState::Editing(col) => match event {
                Event::ButtonPushed(Button::Left, _) => {
                    MENU_ITEMS[self.item_index].edit(game_config, |x| {
                        if x > 0 {
                            x - MENU_ITEMS[self.item_index].cols()[col].multiplier
                        } else {
                            x
                        }
                    });
                }
                Event::ButtonPushed(Button::Right, _) => {
                    MENU_ITEMS[self.item_index].edit(game_config, |x| {
                        (x + MENU_ITEMS[self.item_index].cols()[col].multiplier)
                            .min(MENU_ITEMS[self.item_index].max_val())
                    });
                }
                Event::ButtonPushed(Button::Control, PressType::Single) => {
                    self.edit_mode = EditState::NotEditing
                }
                _ => {}
            },
        }
    }

    pub fn display_state(
        &self,
        prev_state: Option<&Self>,
        prev_game_config: &GameConfig,
        game_config: &GameConfig,
        outputs: &mut Outputs<'_>,
    ) -> Result<(), Error> {
        if Some(self) != prev_state || prev_game_config != game_config {
            match self.edit_mode {
                EditState::NotEditing => {
                    outputs.lcd.clear()?;
                    outputs.lcd.cursor_on(false)?;
                    outputs.lcd.cursor_blink(false)?;

                    self.print_menu(game_config, outputs)?;
                    self.print_value(game_config, outputs)?;
                }
                EditState::Cursor(col) => {
                    self.print_value(game_config, outputs)?;

                    outputs
                        .lcd
                        .set_cursor(1, MENU_ITEMS[self.item_index].cols()[col].position)?;
                    outputs.lcd.cursor_on(true)?;
                    outputs.lcd.cursor_blink(false)?;
                }
                EditState::Editing(col) => {
                    self.print_value(game_config, outputs)?;

                    outputs
                        .lcd
                        .set_cursor(1, MENU_ITEMS[self.item_index].cols()[col].position)?;
                    outputs.lcd.cursor_on(false)?;
                    outputs.lcd.cursor_blink(true)?;
                }
            }
        }
        Ok(())
    }

    fn print_menu(&self, game_config: &GameConfig, outputs: &mut Outputs<'_>) -> Result<(), Error> {
        outputs.lcd.set_cursor(0, 0)?;
        match MENU_ITEMS[self.item_index] {
            MenuItem::Preset => {
                outputs.lcd.write_str("Preset")?;
            }
            MenuItem::LeftTime => {
                outputs.lcd.write_str("Left time")?;
            }
            MenuItem::RightTime => {
                outputs.lcd.write_str("Right time")?;
            }
            MenuItem::IncrementType => {
                outputs.lcd.write_str("Increment type")?;
            }
            MenuItem::LeftDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment { .. } => {
                    outputs.lcd.write_str("Left increment")?;
                }
                IncrementType::Delay { .. } => {
                    outputs.lcd.write_str("Left delay")?;
                }
                IncrementType::Bronstein { .. } => {
                    outputs.lcd.write_str("Left delay")?;
                }
            },
            MenuItem::RightDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment { .. } => {
                    outputs.lcd.write_str("Right increment")?;
                }
                IncrementType::Delay { .. } => {
                    outputs.lcd.write_str("Right delay")?;
                }
                IncrementType::Bronstein { .. } => {
                    outputs.lcd.write_str("Right delay")?;
                }
            },
        }
        Ok(())
    }

    fn print_value(
        &self,
        game_config: &GameConfig,
        outputs: &mut Outputs<'_>,
    ) -> Result<(), Error> {
        outputs.lcd.set_cursor(1, 0)?;
        match MENU_ITEMS[self.item_index] {
            MenuItem::Preset => {
                let preset_name = PRESETS
                    .into_iter()
                    .find_map(|(name, preset)| {
                        if &preset == game_config {
                            Some(name)
                        } else {
                            None
                        }
                    })
                    .unwrap_or("Unknown");

                outputs.lcd.write_str("                ")?;
                outputs.lcd.set_cursor(1, 0)?;
                outputs.lcd.write_str(preset_name)?;
            }
            MenuItem::LeftTime => {
                outputs
                    .lcd
                    .write_str(&format_duration(game_config.left_time)?)?;
            }
            MenuItem::RightTime => {
                outputs
                    .lcd
                    .write_str(&format_duration(game_config.right_time)?)?;
            }
            MenuItem::IncrementType => {
                outputs.lcd.set_cursor(1, 0)?;
                outputs.lcd.write_str("                ")?;
                outputs.lcd.set_cursor(1, 0)?;
                match game_config.increment_type {
                    IncrementType::SuddenDeath => {
                        outputs.lcd.write_str("Sudden death")?;
                    }
                    IncrementType::Increment { .. } => {
                        outputs.lcd.write_str("Increment")?;
                    }
                    IncrementType::Delay { .. } => {
                        outputs.lcd.write_str("Delay")?;
                    }
                    IncrementType::Bronstein { .. } => {
                        outputs.lcd.write_str("Bronstein delay")?;
                    }
                }
            }
            MenuItem::LeftDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment { left_increment, .. } => {
                    outputs.lcd.write_str(&format_duration(left_increment)?)?;
                }
                IncrementType::Delay { left_delay, .. } => {
                    outputs.lcd.write_str(&format_duration(left_delay)?)?;
                }
                IncrementType::Bronstein { left_delay, .. } => {
                    outputs.lcd.write_str(&format_duration(left_delay)?)?;
                }
            },
            MenuItem::RightDelay => match game_config.increment_type {
                IncrementType::SuddenDeath => {}
                IncrementType::Increment {
                    right_increment, ..
                } => {
                    outputs.lcd.write_str(&format_duration(right_increment)?)?;
                }
                IncrementType::Delay { right_delay, .. } => {
                    outputs.lcd.write_str(&format_duration(right_delay)?)?;
                }
                IncrementType::Bronstein { right_delay, .. } => {
                    outputs.lcd.write_str(&format_duration(right_delay)?)?;
                }
            },
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GameConfig {
    pub left_time: Duration,
    pub right_time: Duration,
    pub increment_type: IncrementType,
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            left_time: Duration::from_secs(600),
            right_time: Duration::from_secs(15),
            increment_type: IncrementType::Bronstein {
                left_delay: Duration::from_secs(15),
                right_delay: Duration::from_secs(15),
            },
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum IncrementType {
    SuddenDeath,
    Increment {
        left_increment: Duration,
        right_increment: Duration,
    },
    Delay {
        left_delay: Duration,
        right_delay: Duration,
    },
    Bronstein {
        left_delay: Duration,
        right_delay: Duration,
    },
}
