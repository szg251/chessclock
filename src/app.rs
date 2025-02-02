use embassy_time::Duration;

use crate::{
    effect::Effects,
    error::Error,
    game::{GameState, Player},
    menu::{GameConfig, MenuState},
    Outputs,
};

#[derive(Clone, Copy, Debug)]
pub enum Button {
    Left,
    Right,
    Control,
}

#[derive(Clone, Copy, Debug)]
pub enum PressType {
    Single,
    Long,
}

pub enum Event {
    ButtonPushed(Button, PressType),
    Clock(Duration),
}

#[derive(Clone)]
pub enum Page {
    Init,
    Welcome,
    Menu(MenuState),
    Game(GameState),
    GameOver(Player),
}

impl Page {
    pub fn is_changed(&self, other: &Page) -> bool {
        match self {
            Page::Init => !matches!(other, Page::Init),
            Page::Welcome => !matches!(other, Page::Welcome),
            Page::Menu(_) => !matches!(other, Page::Menu(_)),
            Page::Game(_) => !matches!(other, Page::Game(_)),
            Page::GameOver(_) => !matches!(other, Page::GameOver(_)),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub game_config: GameConfig,
    pub page: Page,
}

impl AppState {
    pub fn handle_event(&mut self, effects: &mut Effects, event: Event) -> Result<(), Error> {
        match event {
            Event::ButtonPushed(Button::Control, PressType::Long) => {
                if matches!(self.page, Page::Menu(_)) {
                    self.page = Page::Welcome
                } else {
                    self.page = Page::Menu(MenuState::new())
                }
            }
            _ => match self.page {
                Page::Init => {}
                Page::Welcome => match event {
                    Event::ButtonPushed(Button::Left, _) => {
                        self.page = Page::Game(GameState::new(&self.game_config, Player::Left));
                    }
                    Event::ButtonPushed(Button::Right, _) => {
                        self.page = Page::Game(GameState::new(&self.game_config, Player::Right));
                    }
                    Event::ButtonPushed(Button::Control, _) => {
                        self.page = Page::Menu(MenuState::new());
                    }
                    Event::Clock(_) => {}
                },
                Page::Menu(ref mut menu_state) => {
                    menu_state.handle_event(&mut self.game_config, &event);
                }
                Page::Game(ref mut game_state) => {
                    game_state.handle_event(effects, &self.game_config, &event)
                }
                Page::GameOver(_) => match event {
                    Event::ButtonPushed(Button::Left, _) => {
                        self.page = Page::Game(GameState::new(&self.game_config, Player::Left))
                    }
                    Event::ButtonPushed(Button::Right, _) => {
                        self.page = Page::Game(GameState::new(&self.game_config, Player::Right))
                    }
                    Event::ButtonPushed(Button::Control, _) => {
                        self.page = Page::Menu(MenuState::new())
                    }
                    Event::Clock(_) => {}
                },
            },
        }
        Ok(())
    }

    pub fn display_state(
        &self,
        prev_state: &AppState,
        outputs: &mut Outputs<'_>,
    ) -> Result<(), Error> {
        if self.page.is_changed(&prev_state.page) {
            outputs.lcd.clear()?;
            outputs.lcd.cursor_blink(false)?;
            outputs.lcd.cursor_on(false)?;
        }
        match self.page {
            Page::Init => {}
            Page::Welcome => {
                if !matches!(prev_state.page, Page::Welcome) {
                    outputs.lcd.set_cursor(0, 3)?;
                    outputs.lcd.write_str("ChessClock")?;
                };
            }
            Page::Menu(ref menu_state) => {
                let prev_game_config = prev_state.game_config.clone();
                let prev_state = if let Page::Menu(ref menu_state) = prev_state.page {
                    Some(menu_state)
                } else {
                    None
                };
                menu_state.display_state(
                    prev_state,
                    &prev_game_config,
                    &self.game_config,
                    outputs,
                )?
            }
            Page::Game(ref game_state) => {
                let prev_state = if let Page::Game(ref game_state) = prev_state.page {
                    Some(game_state)
                } else {
                    None
                };
                game_state.display_state(prev_state, outputs)?
            }
            Page::GameOver(ref loser) => {
                outputs.lcd.set_cursor(0, 0)?;
                match loser {
                    Player::Left => outputs.lcd.write_str("Left player"),
                    Player::Right => outputs.lcd.write_str("Right player"),
                }?;
                outputs.lcd.set_cursor(1, 0)?;
                outputs.lcd.write_str("timeout :(")?;
            }
        }

        Ok(())
    }
}
