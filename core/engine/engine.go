// Package engine provides the abstract speech recognition interface for Voice-typeless.
// It is designed as a standalone library—importable independently of the Tauri application.
//
// Concrete implementations (SenseVoice, Whisper-tiny) live in separate files.
// Use New(ModelType) to obtain an Engine instance.
package engine

import (
	"fmt"
	"io"
	"time"
)

// ModelType identifies the speech recognition model variant.
type ModelType string

const (
	ModelSenseVoice  ModelType = "sensevoice"
	ModelWhisperTiny ModelType = "whisper-tiny"
	ModelCustomONNX  ModelType = "custom-onnx"
)

// DeviceType specifies the inference hardware target.
type DeviceType string

const (
	DeviceAuto     DeviceType = "auto"     // Probe DirectML → CUDA → CPU
	DeviceDirectML DeviceType = "directml" // Windows DirectML (GPU)
	DeviceCUDA     DeviceType = "cuda"     // NVIDIA CUDA
	DeviceCPU      DeviceType = "cpu"      // CPU-only fallback
)

// ModelConfig carries all parameters needed to initialise a model.
type ModelConfig struct {
	Type       ModelType
	ModelPath  string     // Absolute path to .onnx file
	TokensPath string     // Path to tokens.txt (required by sherpa-onnx)
	Device     DeviceType
	Language   string // "auto", "zh", "en", "ja", "ko", …
	NumThreads int    // 0 = use runtime.NumCPU() / 2
}

// Segment represents a timed word or phrase within a recognition result.
type Segment struct {
	Text  string
	Start time.Duration
	End   time.Duration
}

// RecognitionResult is the output of a single inference pass.
type RecognitionResult struct {
	Text       string
	Language   string        // Detected language code
	Confidence float64       // 0.0–1.0
	Duration   time.Duration // Audio duration processed
	Segments   []Segment     // Word-level timestamps (if available)
}

// ModelInfo describes a loaded or available model.
type ModelInfo struct {
	ID          string
	Type        ModelType
	Name        string
	Description string
	SizeBytes   int64
	Languages   []string
	Device      DeviceType // Actual device in use
}

// Engine is the primary interface for speech recognition.
// Implementations must be safe for concurrent use after LoadModel returns.
type Engine interface {
	// LoadModel initialises the model and warms up the inference session.
	// Must be called exactly once before Recognize.
	LoadModel(cfg ModelConfig) error

	// Recognize performs inference on a complete audio buffer.
	// audio must be 16 kHz, mono, normalised float32 in [-1.0, 1.0].
	Recognize(audio []float32, sampleRate int) (*RecognitionResult, error)

	// RecognizeStream performs inference on a streaming audio source.
	// The reader must produce raw float32 LE samples at 16 kHz mono.
	RecognizeStream(r io.Reader, sampleRate int) (<-chan *RecognitionResult, <-chan error)

	// ModelInfo returns metadata about the currently loaded model.
	ModelInfo() ModelInfo

	// Close releases all resources held by the engine.
	// The engine must not be used after Close returns.
	Close() error
}

// New creates an Engine for the given model type.
// Returns an error if the model type is unknown.
func New(modelType ModelType) (Engine, error) {
	switch modelType {
	case ModelSenseVoice:
		return NewSenseVoiceEngine(), nil
	default:
		return nil, fmt.Errorf("engine: unknown model type %q", modelType)
	}
}
