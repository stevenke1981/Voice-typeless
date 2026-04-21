package processor

import "strings"

// defaultFillerWords is the built-in filler word list indexed by language code.
var defaultFillerWords = map[string][]string{
	"zh": {"嗯", "啊", "那个", "就是", "然后", "其实", "对吧", "你知道"},
	"en": {"uh", "um", "er", "ah", "like", "you know", "i mean", "basically", "literally"},
	"ja": {"えーと", "あのー", "まあ", "ちょっと"},
	"ko": {"어", "음", "그", "저"},
}

// fillerFilter implements FillerWordFilter.
type fillerFilter struct {
	custom map[string][]string
}

func newFillerFilter() *fillerFilter {
	return &fillerFilter{custom: make(map[string][]string)}
}

func (f *fillerFilter) Filter(text string, language string) string {
	words := f.wordsFor(language)
	result := text
	for _, w := range words {
		// Remove as middle or trailing word.
		result = strings.ReplaceAll(result, w+" ", "")
		result = strings.ReplaceAll(result, " "+w, "")
		// Handle solo filler.
		result = strings.ReplaceAll(result, w, "")
	}
	return strings.TrimSpace(result)
}

func (f *fillerFilter) AddCustom(word string, language string) {
	f.custom[language] = append(f.custom[language], word)
}

func (f *fillerFilter) wordsFor(language string) []string {
	base, ok := defaultFillerWords[language]
	if !ok {
		base = defaultFillerWords["en"]
	}
	custom := f.custom[language]
	if len(custom) == 0 {
		return base
	}
	combined := make([]string, len(base)+len(custom))
	copy(combined, base)
	copy(combined[len(base):], custom)
	return combined
}
