package processor_test

import (
	"testing"

	"github.com/vtl/core/processor"
)

func TestProcess_FillerWordRemoval(t *testing.T) {
	p := processor.NewProcessor()
	cfg := processor.ProcessorConfig{
		FilterFillerWords:         true,
		MixedLanguageOptimization: false,
		CapitalizeSentences:       false,
		Language:                  "en",
	}
	got, err := p.Process("um hello uh world", cfg)
	if err != nil {
		t.Fatal(err)
	}
	if got == "um hello uh world" {
		t.Errorf("expected filler words removed, got: %q", got)
	}
}

func TestProcess_AutoCapitalize(t *testing.T) {
	p := processor.NewProcessor()
	cfg := processor.DefaultConfig()
	cfg.Language = "en"
	got, err := p.Process("hello world", cfg)
	if err != nil {
		t.Fatal(err)
	}
	if len(got) == 0 || got[0] != 'H' {
		t.Errorf("expected capitalized first letter, got: %q", got)
	}
}

func TestProcess_Empty(t *testing.T) {
	p := processor.NewProcessor()
	got, err := p.Process("", processor.DefaultConfig())
	if err != nil {
		t.Fatal(err)
	}
	if got != "" {
		t.Errorf("expected empty string for empty input, got: %q", got)
	}
}

func TestProcess_MixedLanguage(t *testing.T) {
	p := processor.NewProcessor()
	cfg := processor.DefaultConfig()
	cfg.Language = "zh"
	cfg.CapitalizeSentences = false
	got, err := p.Process("我loveGo", cfg)
	if err != nil {
		t.Fatal(err)
	}
	// Space should be inserted between CJK and Latin.
	if got == "我loveGo" {
		t.Errorf("expected spaces inserted between CJK/Latin, got: %q", got)
	}
}

func TestFillerWordFilter(t *testing.T) {
	f := processor.NewFillerWordFilter()
	result := f.Filter("um hello uh world", "en")
	if result == "um hello uh world" {
		t.Errorf("filler filter did not remove words, got: %q", result)
	}
}

func TestConfigure(t *testing.T) {
	p := processor.NewProcessor()
	cfg := processor.DefaultConfig()
	p.Configure(cfg) // must not panic
}
