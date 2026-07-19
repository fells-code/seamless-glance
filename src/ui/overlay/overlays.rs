use crate::{resources::ssh::SshContext, ui::overlay::scroll::ScrollableOverlay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescribeViewMode {
    Structured,
    Json,
}

pub struct DescribeOverlayState {
    pub title: String,
    pub structured_content: String,
    pub json_content: String,
    pub mode: DescribeViewMode,
    pub scroll: u16,
}

pub struct ConfirmCommandState {
    pub title: String,
    pub command: String,
    pub scroll: u16,
}

pub struct SelectSshKeyState {
    pub title: String,
    pub context: SshContext,
}

pub struct SelectProfileState {
    pub profiles: Vec<String>,
    pub selected: usize,
}

impl SelectProfileState {
    fn move_up(&mut self, count: usize) {
        self.selected = self.selected.saturating_sub(count);
    }

    fn move_down(&mut self, count: usize) {
        if self.profiles.is_empty() {
            return;
        }

        self.selected = (self.selected + count).min(self.profiles.len() - 1);
    }
}

pub enum OverlayState {
    Describe(DescribeOverlayState),
    ConfirmCommand(ConfirmCommandState),
    SelectSshKey(SelectSshKeyState),
    SelectProfile(SelectProfileState),
}

impl DescribeOverlayState {
    pub fn new(title: String, raw_content: String) -> Self {
        let json_content = build_json_view(&raw_content);
        let structured_content = build_structured_view(&json_content);

        Self {
            title,
            structured_content,
            json_content,
            mode: DescribeViewMode::Structured,
            scroll: 0,
        }
    }

    pub fn active_content(&self) -> &str {
        match self.mode {
            DescribeViewMode::Structured => &self.structured_content,
            DescribeViewMode::Json => &self.json_content,
        }
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            DescribeViewMode::Structured => "Structured",
            DescribeViewMode::Json => "JSON",
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            DescribeViewMode::Structured => DescribeViewMode::Json,
            DescribeViewMode::Json => DescribeViewMode::Structured,
        };
        self.scroll = 0;
    }
}

impl OverlayState {
    pub fn scroll_up(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_up(),
            OverlayState::ConfirmCommand(o) => o.scroll_up(),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => o.move_up(1),
        }
    }

    pub fn scroll_down(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_down(),
            OverlayState::ConfirmCommand(o) => o.scroll_down(),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => o.move_down(1),
        }
    }

    pub fn page_up(&mut self, lines: u16) {
        match self {
            OverlayState::Describe(o) => o.page_up(lines),
            OverlayState::ConfirmCommand(o) => o.page_up(lines),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => o.move_up(lines as usize),
        }
    }

    pub fn page_down(&mut self, lines: u16) {
        match self {
            OverlayState::Describe(o) => o.page_down(lines),
            OverlayState::ConfirmCommand(o) => o.page_down(lines),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => o.move_down(lines as usize),
        }
    }

    pub fn scroll_to_top(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_to_top(),
            OverlayState::ConfirmCommand(o) => o.scroll_to_top(),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => o.selected = 0,
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        match self {
            OverlayState::Describe(o) => o.scroll_to_bottom(),
            OverlayState::ConfirmCommand(o) => o.scroll_to_bottom(),
            OverlayState::SelectSshKey(_) => {}
            OverlayState::SelectProfile(o) => {
                o.selected = o.profiles.len().saturating_sub(1);
            }
        }
    }

    pub fn toggle_describe_mode(&mut self) -> bool {
        match self {
            OverlayState::Describe(state) => {
                state.toggle_mode();
                true
            }
            _ => false,
        }
    }
}

fn build_json_view(raw_content: &str) -> String {
    let lines = raw_content
        .lines()
        .filter_map(transform_json_line)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        raw_content.to_string()
    } else {
        lines.join("\n")
    }
}

fn build_structured_view(json_content: &str) -> String {
    let lines = json_content
        .lines()
        .filter_map(transform_structured_line)
        .collect::<Vec<_>>();

    if lines.is_empty() {
        json_content.to_string()
    } else {
        lines.join("\n")
    }
}

fn transform_json_line(line: &str) -> Option<String> {
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed == "Some(" || trimmed == ")" || trimmed == ")," {
        return None;
    }

    if let Some((key, value)) = trimmed.split_once(':') {
        let normalized = normalize_json_value(value.trim());
        if normalized.is_empty() {
            return Some(format!(
                "{}\"{}\":",
                " ".repeat(indent),
                key.trim().trim_matches('"')
            ));
        }

        return Some(format!(
            "{}\"{}\": {}",
            " ".repeat(indent),
            key.trim().trim_matches('"'),
            normalized
        ));
    }

    Some(format!(
        "{}{}",
        " ".repeat(indent),
        normalize_json_value(trimmed)
    ))
}

fn normalize_json_value(value: &str) -> String {
    let (without_comma, had_comma) = strip_trailing_comma(value.trim());

    let normalized = if without_comma == "None" {
        "null".to_string()
    } else if without_comma == "Some(" {
        String::new()
    } else if let Some(inner) = strip_some_inline(without_comma) {
        normalize_json_value(inner)
    } else if let Some(stripped) = strip_struct_name(without_comma) {
        stripped.to_string()
    } else if is_scalar_json_like(without_comma) {
        without_comma.to_string()
    } else {
        format!("\"{}\"", without_comma)
    };

    if normalized.is_empty() {
        normalized
    } else if had_comma && !normalized.ends_with(',') {
        format!("{normalized},")
    } else {
        normalized
    }
}

fn transform_structured_line(line: &str) -> Option<String> {
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    let trimmed = line.trim().trim_end_matches(',');

    if trimmed.is_empty() || trimmed == "{" || trimmed == "}" || trimmed == "[" || trimmed == "]" {
        return None;
    }

    if let Some((key, value)) = trimmed.split_once(':') {
        let key = key.trim().trim_matches('"');
        let value = value.trim();

        if value == "{" || value == "[" || value.is_empty() {
            return Some(format!("{}{}", " ".repeat(indent), key));
        }

        return Some(format!(
            "{}{}: {}",
            " ".repeat(indent),
            key,
            clean_structured_value(value)
        ));
    }

    if trimmed == "{" || trimmed == "[" {
        return None;
    }

    Some(format!(
        "{}- {}",
        " ".repeat(indent),
        clean_structured_value(trimmed)
    ))
}

fn clean_structured_value(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches(',');
    if trimmed == "null" {
        "—".to_string()
    } else {
        trimmed.trim_matches('"').to_string()
    }
}

fn strip_trailing_comma(value: &str) -> (&str, bool) {
    if let Some(stripped) = value.strip_suffix(',') {
        (stripped.trim_end(), true)
    } else {
        (value, false)
    }
}

fn strip_some_inline(value: &str) -> Option<&str> {
    value
        .strip_prefix("Some(")
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
}

fn strip_struct_name(value: &str) -> Option<&str> {
    value
        .split_once(" {")
        .and_then(|(prefix, _)| (!prefix.is_empty() && is_identifier(prefix)).then_some("{"))
}

fn is_scalar_json_like(value: &str) -> bool {
    value.starts_with('"')
        || matches!(value, "true" | "false" | "null" | "{" | "}" | "[" | "]")
        || value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
}

fn is_identifier(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
