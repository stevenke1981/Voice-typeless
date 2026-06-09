//! Post-processing transformations for raw recognition text.
//!
//! This module mirrors `core/processor/` from the Go implementation and
//! provides filler-word removal, mixed-language normalisation, custom
//! dictionary replacement, and sentence capitalisation.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Maps a recognised phrase to a preferred output form.
///
/// Example: `{ input: "a i", output: "AI" }`
#[derive(Debug, Clone)]
pub struct DictionaryEntry {
    pub input: String,
    pub output: String,
}

/// Controls which transformations are applied during processing.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// One of `"auto"`, `"zh"`, `"en"`, `"ja"`, `"ko"`, …
    pub language: String,
    pub filter_filler_words: bool,
    /// Insert spaces at CJK/Latin boundaries.
    pub mixed_language_optimization: bool,
    /// Capitalize first letter of sentences.
    pub capitalize_sentences: bool,
    /// AI-based punctuation restoration (future).
    pub restore_punctuation: bool,
    pub custom_dictionary: Vec<DictionaryEntry>,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            filter_filler_words: true,
            mixed_language_optimization: true,
            capitalize_sentences: true,
            restore_punctuation: false,
            custom_dictionary: vec![],
        }
    }
}

// ---------------------------------------------------------------------------
// Filler word filter
// ---------------------------------------------------------------------------

/// Built-in filler word lists indexed by language code.
fn default_filler_words() -> HashMap<&'static str, &'static [&'static str]> {
    let mut m: HashMap<&'static str, &'static [&'static str]> = HashMap::new();
    m.insert(
        "zh",
        &["嗯", "啊", "那个", "就是", "然后", "其实", "对吧", "你知道"],
    );
    m.insert(
        "en",
        &[
            "uh", "um", "er", "ah", "like", "you know", "i mean", "basically", "literally",
        ],
    );
    m.insert("ja", &["えーと", "あのー", "まあ", "ちょっと"]);
    m.insert("ko", &["어", "음", "그", "저"]);
    m
}

/// Removes spoken filler words from recognised text.
#[derive(Debug, Default)]
pub struct FillerWordFilter {
    custom: HashMap<String, Vec<String>>,
}

impl FillerWordFilter {
    pub fn new() -> Self {
        Self {
            custom: HashMap::new(),
        }
    }

    /// Removes filler words from `text` for the given `language`.
    pub fn filter(&self, text: &str, language: &str) -> String {
        let words = self.words_for(language);
        let mut result = text.to_string();
        for w in words {
            // Remove as middle or trailing word.
            result = result.replace(&format!("{w} "), "");
            result = result.replace(&format!(" {w}"), "");
            // Handle solo filler.
            result = result.replace(w, "");
        }
        result.trim().to_string()
    }

    /// Adds a user-defined filler word for the given language.
    pub fn add_custom(&mut self, word: &str, language: &str) {
        self.custom
            .entry(language.to_string())
            .or_default()
            .push(word.to_string());
    }

    // -- internal -----------------------------------------------------------

    fn words_for(&self, language: &str) -> Vec<&str> {
        let default_map = default_filler_words();
        let base: Vec<&str> = default_map
            .get(language)
            .copied()
            .unwrap_or(default_map.get("en").unwrap())
            .to_vec();

        let custom = self.custom.get(language);
        match custom {
            Some(c) if !c.is_empty() => {
                let mut combined: Vec<&str> = base;
                combined.extend(c.iter().map(String::as_str));
                combined
            }
            _ => base,
        }
    }
}

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

/// Simple heuristic language detection based on Unicode character ranges.
/// Replace with a proper language-ID model for production use.
pub fn detect_language(text: &str) -> &'static str {
    for ch in text.chars() {
        if ch.is_ascii() {
            continue;
        }
        if ch as u32 >= 0x4E00 && ch as u32 <= 0x9FFF {
            return "zh";
        }
        if (ch as u32 >= 0x3040 && ch as u32 <= 0x309F)
            || (ch as u32 >= 0x30A0 && ch as u32 <= 0x30FF)
        {
            return "ja";
        }
        if ch as u32 >= 0xAC00 && ch as u32 <= 0xD7AF {
            return "ko";
        }
    }
    "en"
}

// ---------------------------------------------------------------------------
// Mixed-language normalisation
// ---------------------------------------------------------------------------

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF | // Han
        0x3040..=0x309F | // Hiragana
        0x30A0..=0x30FF | // Katakana
        0xAC00..=0xD7AF   // Hangul
    )
}

fn is_latin(ch: char) -> bool {
    ch.is_alphabetic() && !is_cjk(ch)
}

/// Inserts spaces between CJK and Latin character boundaries.
///
/// Example: `"我loveGo语言"` → `"我 loveGo 语言"`
pub fn normalize_mixed_language(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(chars.len() + 8);
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 {
            let prev = chars[i - 1];
            let cjk_now = is_cjk(ch);
            let cjk_prev = is_cjk(prev);
            let latin_now = is_latin(ch);
            let latin_prev = is_latin(prev);
            if (cjk_prev && latin_now) || (latin_prev && cjk_now) {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

// ---------------------------------------------------------------------------
// Text processor
// ---------------------------------------------------------------------------

/// The main post-processing pipeline.
#[derive(Debug)]
pub struct TextProcessor {
    config: ProcessorConfig,
    filler: FillerWordFilter,
}

impl TextProcessor {
    /// Creates a new `TextProcessor` with the given config.
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            filler: FillerWordFilter::new(),
            config,
        }
    }

    /// Creates a `TextProcessor` with default config.
    pub fn new_default() -> Self {
        Self::new(ProcessorConfig::default())
    }

    /// Updates the processor config without re-creating the instance.
    pub fn configure(&mut self, config: ProcessorConfig) {
        self.config = config;
    }

    /// Applies the full configured pipeline to `raw` recognised text.
    ///
    /// `config_override` overrides the instance config for this single call.
    pub fn process(&self, raw: &str, config_override: &ProcessorConfig) -> Result<String, ()> {
        let mut text = raw.trim().to_string();
        if text.is_empty() {
            return Ok(String::new());
        }

        let lang = if config_override.language == "auto" {
            detect_language(&text)
        } else {
            config_override.language.as_str()
        };

        if config_override.filter_filler_words {
            text = self.filler.filter(&text, lang);
        }

        if config_override.mixed_language_optimization {
            text = normalize_mixed_language(&text);
        }

        // Apply custom dictionary replacements.
        for entry in &config_override.custom_dictionary {
            text = text.replace(&entry.input, &entry.output);
        }

        if config_override.capitalize_sentences && !text.is_empty() {
            // Capitalize first character
            let mut chars: Vec<char> = text.chars().collect();
            chars[0] = chars[0].to_uppercase().next().unwrap_or(chars[0]);
            text = chars.into_iter().collect();
        }

        Ok(text)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- FillerWordFilter tests --------------------------------------------

    #[test]
    fn test_filler_filter_removes_words() {
        let f = FillerWordFilter::new();
        let result = f.filter("um hello uh world", "en");
        assert_ne!(result, "um hello uh world");
        assert!(!result.contains("um"), "result: {result:?}");
        assert!(!result.contains("uh"), "result: {result:?}");
    }

    #[test]
    fn test_filler_filter_custom_words() {
        let mut f = FillerWordFilter::new();
        f.add_custom("customfiller", "en");
        let result = f.filter("hello customfiller world", "en");
        assert!(!result.contains("customfiller"), "result: {result:?}");
    }

    // -- detect_language tests ---------------------------------------------

    #[test]
    fn test_detect_language_english() {
        assert_eq!(detect_language("hello world"), "en");
    }

    #[test]
    fn test_detect_language_chinese() {
        assert_eq!(detect_language("你好世界"), "zh");
    }

    #[test]
    fn test_detect_language_japanese() {
        assert_eq!(detect_language("こんにちは"), "ja");
        assert_eq!(detect_language("カタカナ"), "ja");
    }

    #[test]
    fn test_detect_language_korean() {
        assert_eq!(detect_language("안녕하세요"), "ko");
    }

    // -- normalize_mixed_language tests ------------------------------------

    #[test]
    fn test_mixed_language_spaces() {
        let result = normalize_mixed_language("我loveGo");
        // Should insert a space between 我 (CJK) and l (Latin)
        assert!(
            result.contains(' '),
            "expected spaces inserted between CJK/Latin, got: {result:?}"
        );
    }

    #[test]
    fn test_no_spaces_for_pure_latin() {
        let result = normalize_mixed_language("hello world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_no_spaces_for_pure_cjk() {
        let result = normalize_mixed_language("你好世界");
        assert_eq!(result, "你好世界");
    }

    // -- TextProcessor tests -----------------------------------------------

    #[test]
    fn test_process_filler_word_removal() {
        let p = TextProcessor::new_default();
        let cfg = ProcessorConfig {
            filter_filler_words: true,
            mixed_language_optimization: false,
            capitalize_sentences: false,
            language: "en".into(),
            ..Default::default()
        };
        let got = p.process("um hello uh world", &cfg).unwrap();
        assert_ne!(got, "um hello uh world");
    }

    #[test]
    fn test_process_auto_capitalize() {
        let p = TextProcessor::new_default();
        let cfg = ProcessorConfig {
            language: "en".into(),
            ..Default::default()
        };
        let got = p.process("hello world", &cfg).unwrap();
        assert!(!got.is_empty());
        assert!(got.starts_with('H'), "expected capitalized first letter, got: {got:?}");
    }

    #[test]
    fn test_process_empty() {
        let p = TextProcessor::new_default();
        let got = p.process("", &ProcessorConfig::default()).unwrap();
        assert_eq!(got, "");
    }

    #[test]
    fn test_process_mixed_language() {
        let p = TextProcessor::new_default();
        let cfg = ProcessorConfig {
            language: "zh".into(),
            capitalize_sentences: false,
            ..Default::default()
        };
        let got = p.process("我loveGo", &cfg).unwrap();
        assert!(
            got.contains(' '),
            "expected spaces inserted between CJK/Latin, got: {got:?}"
        );
    }

    #[test]
    fn test_custom_dictionary() {
        let p = TextProcessor::new_default();
        let cfg = ProcessorConfig {
            custom_dictionary: vec![DictionaryEntry {
                input: "a i".into(),
                output: "AI".into(),
            }],
            capitalize_sentences: false,
            ..Default::default()
        };
        let got = p.process("hello a i world", &cfg).unwrap();
        assert!(got.contains("AI"), "got: {got:?}");
    }

    #[test]
    fn test_configure() {
        let mut p = TextProcessor::new_default();
        let cfg = ProcessorConfig {
            language: "en".into(),
            ..Default::default()
        };
        p.configure(cfg); // must not panic
    }
}
