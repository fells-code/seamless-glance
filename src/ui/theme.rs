use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeName {
    Autumn,
    Winter,
    Summer,
    Spring,
    Developer,
}

impl ThemeName {
    pub const ALL: [ThemeName; 5] = [
        ThemeName::Autumn,
        ThemeName::Winter,
        ThemeName::Summer,
        ThemeName::Spring,
        ThemeName::Developer,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeName::Autumn => "autumn",
            ThemeName::Winter => "winter",
            ThemeName::Summer => "summer",
            ThemeName::Spring => "spring",
            ThemeName::Developer => "developer",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ThemeName::Autumn => "Autumn",
            ThemeName::Winter => "Winter",
            ThemeName::Summer => "Summer",
            ThemeName::Spring => "Spring",
            ThemeName::Developer => "Developer",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ThemeName::Autumn => "Warm earth tones and copper accents",
            ThemeName::Winter => "Snowy stone, alpine blue, and glacial slate",
            ThemeName::Summer => "Sunlit sand, bright surf, and deep ocean blues",
            ThemeName::Spring => "Fresh blooms, new grass, and warm morning light",
            ThemeName::Developer => "Low-glare graphite, terminal cyan, and focused contrast",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "autumn" => Some(ThemeName::Autumn),
            "winter" => Some(ThemeName::Winter),
            "summer" => Some(ThemeName::Summer),
            "spring" => Some(ThemeName::Spring),
            "developer" => Some(ThemeName::Developer),
            _ => None,
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|name| *name == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub primary: Color,
    pub accent: Color,
    pub background: Color,
    pub text: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Autumn => Self {
                primary: Color::Rgb(182, 78, 57),
                accent: Color::Rgb(243, 230, 223),
                background: Color::Rgb(33, 25, 22),
                text: Color::Rgb(240, 234, 229),
                highlight: Color::Rgb(201, 120, 74),
            },
            ThemeName::Winter => Self {
                primary: Color::Rgb(94, 137, 166),
                accent: Color::Rgb(239, 245, 248),
                background: Color::Rgb(26, 35, 43),
                text: Color::Rgb(233, 240, 244),
                highlight: Color::Rgb(143, 178, 201),
            },
            ThemeName::Summer => Self {
                primary: Color::Rgb(242, 191, 99),
                accent: Color::Rgb(51, 166, 201),
                background: Color::Rgb(19, 38, 51),
                text: Color::Rgb(248, 237, 210),
                highlight: Color::Rgb(255, 212, 122),
            },
            ThemeName::Spring => Self {
                primary: Color::Rgb(232, 143, 176),
                accent: Color::Rgb(127, 191, 116),
                background: Color::Rgb(29, 35, 28),
                text: Color::Rgb(247, 241, 223),
                highlight: Color::Rgb(244, 217, 108),
            },
            ThemeName::Developer => Self {
                primary: Color::Rgb(58, 174, 216),
                accent: Color::Rgb(122, 211, 239),
                background: Color::Rgb(31, 41, 51),
                text: Color::Rgb(222, 230, 236),
                highlight: Color::Rgb(87, 198, 234),
            },
        }
    }

    pub fn autumn() -> Self {
        Self::from_name(ThemeName::Autumn)
    }
}
