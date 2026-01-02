use ratatui::style::Color;

pub struct Theme {
    pub primary: Color,
    pub accent: Color,
    pub background: Color,
    pub text: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn seamless() -> Self {
        Self {
            primary: Color::Rgb(33, 105, 168), // #2169A8
            accent: Color::Rgb(180, 220, 255), // complementary soft highlight
            background: Color::Black,
            text: Color::White,
            highlight: Color::Rgb(33, 105, 168),
        }
    }
}
