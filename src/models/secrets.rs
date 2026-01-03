#[derive(Debug, Clone)]
pub struct SecretInfo {
    pub name: String,
    pub last_rotated: Option<String>,
    pub rotation_enabled: bool,
}
