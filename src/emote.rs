use ratatui::style::Color;

#[derive(Debug, Default, Clone, Copy)]
pub enum Emote {
    #[default]
    None,
    Smile,
    Frown,
    Laugh,
    Angry,
    Sad,
    Confused,
    Wink,
}

pub fn get_emote(c: char) -> Option<Emote> {
    match c {
        '🙂' => Some(Emote::Smile),
        '😐' => Some(Emote::Frown),
        '😄' => Some(Emote::Laugh),
        '😡' => Some(Emote::Angry),
        '😞' => Some(Emote::Sad),
        '😕' => Some(Emote::Confused),
        '😊' => Some(Emote::Wink),
        _ => None,
    }
}

pub fn get_color(c: char) -> Option<Color> {
    match c {
        '🟥' => Some(Color::Red),
        '🟩' => Some(Color::Green),
        '🟨' => Some(Color::Yellow),
        '🟦' => Some(Color::Blue),
        '🟪' => Some(Color::Magenta),
        '⬜' => Some(Color::Reset),
        '🔴' => Some(Color::LightRed),
        '🟢' => Some(Color::LightGreen),
        '🟡' => Some(Color::LightYellow),
        '🔵' => Some(Color::LightBlue),
        '🟣' => Some(Color::LightMagenta),
        _ => None,
    }
}
