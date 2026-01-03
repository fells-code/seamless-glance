use ratatui::style::Color;

pub struct Theme {
    pub primary: Color,    // brand / headings / borders
    pub accent: Color,     // secondary highlights
    pub background: Color, // main background
    pub text: Color,       // primary text
    pub highlight: Color,  // selection / focus
}

impl Theme {
    pub fn seamless() -> Self {
        Self {
            // Burnt orange brand color (#E46B2E)
            primary: Color::Rgb(228, 107, 46),

            // Softer orange accent for subtle emphasis
            accent: Color::Rgb(255, 179, 128),

            // Deep navy-black background (#0F1320)
            background: Color::Rgb(15, 19, 32),

            // Near-white text for readability (#E6E6E6)
            text: Color::Rgb(230, 230, 230),

            // Highlight uses brand orange (not bright yellow)
            highlight: Color::Rgb(228, 107, 46),
        }
    }
}
