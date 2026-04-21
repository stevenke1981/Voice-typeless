// Package processor applies post-processing transformations to raw recognition text.
// This includes filler-word removal, punctuation insertion, and mixed-language normalisation.
package processor

import (
	"strings"
	"unicode"
)

// DictionaryEntry maps a recognised phrase to a preferred output form.
type DictionaryEntry struct {
	Input  string // What the model may output, e.g. "a i"
	Output string // What to replace it with, e.g. "AI"
}

// ProcessorConfig controls which transformations are applied.
type ProcessorConfig struct {
	Language                  string // "auto", "zh", "en", "ja", "ko", …
	FilterFillerWords         bool
	MixedLanguageOptimization bool  // Insert spaces at CJK/Latin boundaries
	CapitalizeSentences       bool  // Capitalize first letter of sentences
	RestorePunctuation        bool  // AI-based punctuation restoration (future)
	CustomDictionary          []DictionaryEntry
}

// DefaultConfig returns sensible defaults.
func DefaultConfig() ProcessorConfig {
	return ProcessorConfig{
		Language:                  "auto",
		FilterFillerWords:         true,
		MixedLanguageOptimization: true,
		CapitalizeSentences:       true,
	}
}

// TextProcessor is the main post-processing pipeline.
type TextProcessor interface {
	// Process applies the full configured pipeline to raw recognised text.
	// cfg overrides the instance config for this single call.
	Process(raw string, cfg ProcessorConfig) (string, error)

	// Configure updates the processor config without re-creating the instance.
	Configure(cfg ProcessorConfig)
}

// FillerWordFilter removes spoken filler words from recognised text.
// Built-in lists cover: zh, en, ja, ko.
type FillerWordFilter interface {
	// Filter removes filler words from text.
	Filter(text string, language string) string

	// AddCustom adds a user-defined filler word for the given language.
	AddCustom(word string, language string)
}

// NewTextProcessor creates a TextProcessor with the given config.
func NewTextProcessor(cfg ProcessorConfig) TextProcessor {
	return &defaultProcessor{cfg: cfg, filler: newFillerFilter()}
}

// NewProcessor is an alias for NewTextProcessor with default config.
// Kept for backward compatibility with existing call sites.
func NewProcessor() TextProcessor {
	return NewTextProcessor(DefaultConfig())
}

// NewFillerWordFilter creates a FillerWordFilter with the built-in word lists.
func NewFillerWordFilter() FillerWordFilter { return newFillerFilter() }

// defaultProcessor implements TextProcessor.
type defaultProcessor struct {
	cfg    ProcessorConfig
	filler *fillerFilter
}

func (p *defaultProcessor) Configure(cfg ProcessorConfig) { p.cfg = cfg }

func (p *defaultProcessor) Process(raw string, cfg ProcessorConfig) (string, error) {
	text := strings.TrimSpace(raw)
	if text == "" {
		return "", nil
	}

	lang := cfg.Language
	if lang == "auto" {
		lang = detectLanguage(text)
	}

	if cfg.FilterFillerWords {
		text = p.filler.Filter(text, lang)
	}

	if cfg.MixedLanguageOptimization {
		text = normalizeMixedLanguage(text)
	}

	// Apply custom dictionary replacements.
	for _, entry := range cfg.CustomDictionary {
		text = strings.ReplaceAll(text, entry.Input, entry.Output)
	}

	if cfg.CapitalizeSentences && len(text) > 0 {
		runes := []rune(text)
		runes[0] = unicode.ToUpper(runes[0])
		text = string(runes)
	}

	return text, nil
}

// detectLanguage uses simple heuristics; replace with a proper LangID model.
func detectLanguage(text string) string {
	for _, r := range text {
		if unicode.In(r, unicode.Han) {
			return "zh"
		}
		if unicode.In(r, unicode.Hiragana, unicode.Katakana) {
			return "ja"
		}
		if unicode.In(r, unicode.Hangul) {
			return "ko"
		}
	}
	return "en"
}
