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
            // Muted copper
            primary: Color::Rgb(201, 120, 74),

            // Warm sand accent
            accent: Color::Rgb(234, 204, 163),

            // Brown-charcoal background
            background: Color::Rgb(24, 20, 18),

            // Creamy light text
            text: Color::Rgb(236, 231, 224),

            // Warm orange highlight
            highlight: Color::Rgb(223, 146, 92),
        }
    }
}
