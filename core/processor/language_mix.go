package processor

import "unicode"

// normalizeMixedLanguage inserts spaces between CJK and Latin character boundaries.
// Example: "我loveGo语言" → "我 loveGo 语言"
func normalizeMixedLanguage(text string) string {
	runes := []rune(text)
	out := make([]rune, 0, len(runes)+8)
	for i, r := range runes {
		if i > 0 {
			prev := runes[i-1]
			cjkNow := isCJK(r)
			cjkPrev := isCJK(prev)
			latinNow := isLatin(r)
			latinPrev := isLatin(prev)
			if (cjkPrev && latinNow) || (latinPrev && cjkNow) {
				out = append(out, ' ')
			}
		}
		out = append(out, r)
	}
	return string(out)
}

func isCJK(r rune) bool {
	return unicode.In(r, unicode.Han, unicode.Hiragana, unicode.Katakana, unicode.Hangul)
}

func isLatin(r rune) bool {
	return unicode.IsLetter(r) && !isCJK(r)
}
