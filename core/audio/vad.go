package audio

import "math"

// VADConfig configures voice activity detection.
type VADConfig struct {
	// EnergyThreshold is the RMS energy level above which audio is considered speech.
	// Typical range: 0.01–0.05. Default: 0.02.
	EnergyThreshold float32
	// SilenceDurationMs is how long continuous silence triggers auto-stop.
	// Default: 3000 ms.
	SilenceDurationMs int
}

// DefaultVADConfig returns sensible defaults for VAD.
func DefaultVADConfig() VADConfig {
	return VADConfig{
		EnergyThreshold:   0.02,
		SilenceDurationMs: 3000,
	}
}

// IsSpeech returns true if the audio chunk contains speech based on RMS energy.
func IsSpeech(chunk []float32, cfg VADConfig) bool {
	if len(chunk) == 0 {
		return false
	}
	var sum float64
	for _, s := range chunk {
		sum += float64(s) * float64(s)
	}
	rms := math.Sqrt(sum / float64(len(chunk)))
	return float32(rms) > cfg.EnergyThreshold
}
