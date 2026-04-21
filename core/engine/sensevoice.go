package engine

import (
	"io"
	"runtime"
)

// senseVoiceEngine is the sherpa-onnx SenseVoice implementation.
// It satisfies the Engine interface.
//
// Production use requires github.com/k2-fsa/sherpa-onnx-go and the
// corresponding native sherpa-onnx shared libraries.
type senseVoiceEngine struct {
	cfg   ModelConfig
	ready bool
	// recognizer *sherpaonnx.OnlineRecognizer  // TODO: uncomment with sherpa-onnx-go
}

// NewSenseVoiceEngine returns a new Engine backed by SenseVoice.
func NewSenseVoiceEngine() Engine {
	return &senseVoiceEngine{}
}

func (e *senseVoiceEngine) LoadModel(cfg ModelConfig) error {
	// 1. Select device if "auto".
	if cfg.Device == DeviceAuto {
		cfg.Device = ProbeDevice()
	}
	// 2. Default thread count to half the logical CPUs.
	if cfg.NumThreads <= 0 {
		n := runtime.NumCPU() / 2
		if n < 1 {
			n = 1
		}
		cfg.NumThreads = n
	}
	// TODO: validate cfg.ModelPath and cfg.TokensPath exist on disk.
	// TODO: create sherpa-onnx SenseVoice recognizer with cfg.
	e.cfg = cfg
	e.ready = true
	return nil
}

func (e *senseVoiceEngine) Recognize(audio []float32, sampleRate int) (*RecognitionResult, error) {
	if !e.ready {
		return nil, ErrModelNotLoaded
	}
	// TODO: run sherpa-onnx inference on audio buffer.
	return &RecognitionResult{
		Text:       "",
		Language:   e.cfg.Language,
		Confidence: 0,
	}, nil
}

func (e *senseVoiceEngine) RecognizeStream(r io.Reader, sampleRate int) (<-chan *RecognitionResult, <-chan error) {
	results := make(chan *RecognitionResult)
	errs := make(chan error, 1)
	go func() {
		defer close(results)
		defer close(errs)
		if !e.ready {
			errs <- ErrModelNotLoaded
			return
		}
		// TODO: stream PCM from r and push partial RecognitionResults to results.
		_ = r
	}()
	return results, errs
}

func (e *senseVoiceEngine) ModelInfo() ModelInfo {
	return ModelInfo{
		ID:        "sensevoice-small",
		Type:      ModelSenseVoice,
		Name:      "SenseVoice Small",
		Languages: []string{"zh", "en", "ja", "ko"},
		Device:    e.cfg.Device,
	}
}

func (e *senseVoiceEngine) Close() error {
	e.ready = false
	return nil
}
