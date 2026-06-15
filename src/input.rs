use uefi::proto::console::text::{Input, Key, ScanCode};

#[derive(Debug)]
pub enum GameInput {
    None,
    Left,
    Right,
    Select,
    Confirm,
    Bank,
    Roll,
    Quit,
    LangToggle,
    Escape,
}

pub fn poll_input(input: &mut Input) -> GameInput {
    match input.read_key() {
        Ok(Some(key)) => match key {
            Key::Special(sc) => match sc {
                ScanCode::LEFT => GameInput::Left,
                ScanCode::RIGHT => GameInput::Right,
                ScanCode::UP => GameInput::Left,
                ScanCode::DOWN => GameInput::Right,
                ScanCode::ESCAPE => GameInput::Escape,
                _ => GameInput::None,
            },
            Key::Printable(c) => {
                let ch: char = c.into();
                match ch {
                    ' ' => GameInput::Select,
                    '\r' | '\n' => GameInput::Confirm,
                    'r' | 'R' => GameInput::Roll,
                    'b' | 'B' => GameInput::Bank,
                    'q' | 'Q' => GameInput::Quit,
                    'l' | 'L' => GameInput::LangToggle,
                    _ => GameInput::None,
                }
            }
        },
        Ok(None) => GameInput::None,
        Err(_) => GameInput::None,
    }
}
