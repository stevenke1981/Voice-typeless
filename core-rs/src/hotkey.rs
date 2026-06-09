use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Modifier — bitmask newtype (no bitflags crate dep)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifier(u32);

impl Modifier {
    pub const NONE: Modifier = Modifier(0);
    pub const CTRL: Modifier = Modifier(1);
    pub const SHIFT: Modifier = Modifier(2);
    pub const ALT: Modifier = Modifier(4);
    pub const SUPER: Modifier = Modifier(8);

    pub const fn from_bits(bits: u32) -> Modifier {
        Modifier(bits)
    }

    pub fn contains(self, other: Modifier) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn bits(self) -> u32 {
        self.0
    }

    fn names(self) -> Vec<&'static str> {
        let mut v = Vec::with_capacity(4);
        if self.contains(Modifier::CTRL) { v.push("Ctrl"); }
        if self.contains(Modifier::SHIFT) { v.push("Shift"); }
        if self.contains(Modifier::ALT) { v.push("Alt"); }
        if self.contains(Modifier::SUPER) { v.push("Super"); }
        v
    }
}

impl std::ops::BitOr for Modifier {
    type Output = Modifier;
    fn bitor(self, rhs: Modifier) -> Modifier { Modifier(self.0 | rhs.0) }
}

impl std::ops::BitOrAssign for Modifier {
    fn bitor_assign(&mut self, rhs: Modifier) { self.0 |= rhs.0; }
}

impl fmt::Display for Modifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.names().join("|");
        if s.is_empty() { write!(f, "None") } else { write!(f, "{s}") }
    }
}

// ---------------------------------------------------------------------------
// KeyCombo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyCombo {
    pub modifiers: Modifier,
    pub key: String,
}

impl KeyCombo {
    pub fn new(modifiers: Modifier, key: impl Into<String>) -> Self {
        KeyCombo { modifiers, key: key.into() }
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = self.modifiers.names();
        let mut parts: Vec<&str> = Vec::with_capacity(names.len() + 1);
        parts.extend_from_slice(&names);
        parts.push(&self.key);
        write!(f, "{}", parts.join("+"))
    }
}

// ---------------------------------------------------------------------------
// HotkeyAction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    PushToTalk,
    FreeSpeech,
    Cancel,
}

impl HotkeyAction {
    pub fn as_str(self) -> &'static str {
        match self {
            HotkeyAction::PushToTalk => "push_to_talk",
            HotkeyAction::FreeSpeech => "free_speech",
            HotkeyAction::Cancel => "cancel",
        }
    }
}

impl fmt::Display for HotkeyAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// HotkeyEvent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct HotkeyEvent {
    pub action: HotkeyAction,
    pub pressed: bool,
    pub combo: KeyCombo,
}

// ---------------------------------------------------------------------------
// HotkeyConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct HotkeyConfig {
    pub push_to_talk: KeyCombo,
    pub free_speech: KeyCombo,
    pub cancel: KeyCombo,
}

// ---------------------------------------------------------------------------
// HotkeyError
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("empty hotkey combo string")]
    EmptyCombo,

    #[error("invalid modifier \"{0}\" in \"{1}\"")]
    InvalidModifier(String, String),

    #[error("no key specified in combo \"{0}\"")]
    NoKeySpecified(String),

    #[error("hotkey registration failed: {0}")]
    RegistrationFailed(String),
}

// ---------------------------------------------------------------------------
// HotkeyManager trait + StubHotkeyManager
// ---------------------------------------------------------------------------

pub trait HotkeyManager {
    fn register(&mut self, cfg: HotkeyConfig) -> Result<(), HotkeyError>;
    fn unregister(&mut self) -> Result<(), HotkeyError>;
}

#[derive(Debug, Default)]
pub struct StubHotkeyManager;

impl StubHotkeyManager {
    pub fn new() -> Self {
        StubHotkeyManager
    }
}

impl HotkeyManager for StubHotkeyManager {
    fn register(&mut self, _cfg: HotkeyConfig) -> Result<(), HotkeyError> {
        Ok(())
    }

    fn unregister(&mut self) -> Result<(), HotkeyError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// parse_key_combo
// ---------------------------------------------------------------------------

/// Parse a hotkey string like "Ctrl+Shift+V" into a KeyCombo.
///
/// The last token is treated as the key; all preceding tokens must be valid
/// modifier names. Case-insensitive.
pub fn parse_key_combo(s: &str) -> Result<KeyCombo, HotkeyError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(HotkeyError::EmptyCombo);
    }

    let parts: Vec<&str> = trimmed.split('+').map(|p| p.trim()).collect();

    if parts.is_empty() || parts.iter().all(|p| p.is_empty()) {
        return Err(HotkeyError::EmptyCombo);
    }

    // Edge case: "Ctrl+" — trailing plus with empty last token
    if parts.len() >= 2 && parts.last().map_or(true, |p| p.is_empty()) {
        return Err(HotkeyError::NoKeySpecified(trimmed.to_string()));
    }

    let mut modifiers = Modifier::NONE;

    for part in parts[..parts.len() - 1].iter() {
        if part.is_empty() {
            return Err(HotkeyError::InvalidModifier(
                String::new(),
                trimmed.to_string(),
            ));
        }
        let lower = part.to_lowercase();
        let m = match lower.as_str() {
            "ctrl" | "control" => Modifier::CTRL,
            "shift" => Modifier::SHIFT,
            "alt" => Modifier::ALT,
            "super" | "win" | "cmd" | "command" => Modifier::SUPER,
            _ => {
                return Err(HotkeyError::InvalidModifier(
                    (*part).to_string(),
                    trimmed.to_string(),
                ));
            }
        };
        modifiers = modifiers | m;
    }

    let key = parts.last().map(|s| s.to_string()).unwrap_or_default();

    if key.is_empty() {
        return Err(HotkeyError::NoKeySpecified(trimmed.to_string()));
    }

    Ok(KeyCombo { modifiers, key })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_key() {
        let kc = parse_key_combo("V").unwrap();
        assert_eq!(kc.modifiers, Modifier::NONE);
        assert_eq!(kc.key, "V");
    }

    #[test]
    fn test_parse_modifier_key() {
        let kc = parse_key_combo("Ctrl+Shift+V").unwrap();
        assert!(kc.modifiers.contains(Modifier::CTRL));
        assert!(kc.modifiers.contains(Modifier::SHIFT));
        assert!(!kc.modifiers.contains(Modifier::ALT));
        assert!(!kc.modifiers.contains(Modifier::SUPER));
        assert_eq!(kc.key, "V");
    }

    #[test]
    fn test_parse_alt_space() {
        let kc = parse_key_combo("Alt+Space").unwrap();
        assert_eq!(kc.modifiers, Modifier::ALT);
        assert_eq!(kc.key, "Space");
    }

    #[test]
    fn test_parse_super_variants() {
        for s in &["Super+A", "Win+A", "Cmd+A", "Command+A"] {
            let kc = parse_key_combo(s).unwrap();
            assert!(kc.modifiers.contains(Modifier::SUPER), "failed for {s}");
            assert_eq!(kc.key, "A");
        }
    }

    #[test]
    fn test_parse_case_insensitive() {
        let kc = parse_key_combo("CTRL+SHIFT+V").unwrap();
        assert!(kc.modifiers.contains(Modifier::CTRL));
        assert!(kc.modifiers.contains(Modifier::SHIFT));
        assert_eq!(kc.key, "V");
    }

    #[test]
    fn test_parse_empty_error() {
        assert!(matches!(parse_key_combo(""), Err(HotkeyError::EmptyCombo)));
        assert!(matches!(parse_key_combo("   "), Err(HotkeyError::EmptyCombo)));
    }

    #[test]
    fn test_parse_no_key_error() {
        let err = parse_key_combo("Ctrl+").unwrap_err();
        assert!(matches!(err, HotkeyError::NoKeySpecified(_)));
    }

    #[test]
    fn test_parse_unexpected_modifier() {
        let err = parse_key_combo("Ctrl+Shift+Foo+X").unwrap_err();
        assert!(matches!(err, HotkeyError::InvalidModifier(..)));
    }

    #[test]
    fn test_parse_three_modifiers() {
        let kc = parse_key_combo("Ctrl+Shift+Alt+X").unwrap();
        assert!(kc.modifiers.contains(Modifier::CTRL));
        assert!(kc.modifiers.contains(Modifier::SHIFT));
        assert!(kc.modifiers.contains(Modifier::ALT));
        assert_eq!(kc.key, "X");
    }

    #[test]
    fn test_key_combo_display() {
        let kc = KeyCombo {
            modifiers: Modifier::CTRL | Modifier::SHIFT,
            key: "V".to_string(),
        };
        assert_eq!(kc.to_string(), "Ctrl+Shift+V");
    }

    #[test]
    fn test_stub_manager_register_unregister() {
        let mut mgr = StubHotkeyManager::new();
        let cfg = HotkeyConfig {
            push_to_talk: KeyCombo::new(Modifier::CTRL | Modifier::SHIFT, "V"),
            free_speech: KeyCombo::new(Modifier::CTRL | Modifier::SHIFT, "B"),
            cancel: KeyCombo::new(Modifier::CTRL | Modifier::SHIFT, "C"),
        };
        assert!(mgr.register(cfg).is_ok());
        assert!(mgr.unregister().is_ok());
    }

    #[test]
    fn test_hotkey_action_as_str() {
        assert_eq!(HotkeyAction::PushToTalk.as_str(), "push_to_talk");
        assert_eq!(HotkeyAction::FreeSpeech.as_str(), "free_speech");
        assert_eq!(HotkeyAction::Cancel.as_str(), "cancel");
    }
}
