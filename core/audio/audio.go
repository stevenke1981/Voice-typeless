// Package audio handles microphone capture and audio playback for Voice-typeless.
// Uses malgo (miniaudio) for cross-platform, zero-dependency audio I/O.
// Production use requires github.com/gen2brain/malgo and the libminiaudio native library.
package audio

import "time"

// SampleRate is the canonical sample rate required by sherpa-onnx.
const SampleRate = 16000

// DeviceInfo describes a physical audio input device.
type DeviceInfo struct {
	ID          string
	Name        string
	IsDefault   bool
	Channels    int
	SampleRates []int
}

// RecorderConfig configures an AudioRecorder session.
type RecorderConfig struct {
	DeviceID   string // "" or "default" → system default
	SampleRate int    // Must be 16000 for direct engine use
	Channels   int    // 1 = mono (required); 2 = stereo (downmixed internally)
	BufferSize int    // Ring buffer size in samples (0 = 16000*30 = 30 s)
}

// AudioChunk is a slice of PCM samples with metadata.
type AudioChunk struct {
	Samples    []float32
	SampleRate int
	CapturedAt time.Time
}

// AudioRecorder captures microphone input.
// All implementations use malgo (miniaudio) internally.
type AudioRecorder interface {
	// Start begins audio capture using cfg.
	// Returns an error if already recording or the device cannot be opened.
	Start(cfg RecorderConfig) error

	// Stop ends capture and flushes the ring buffer.
	Stop() error

	// Cancel ends capture and discards buffered audio.
	Cancel()

	// Drain returns all buffered samples since the last Start.
	// Safe to call only after Stop.
	Drain() (*AudioChunk, error)

	// Subscribe returns a channel that receives audio chunks in real time.
	// Must be called before Start. The channel is closed when Stop/Cancel is called.
	Subscribe() <-chan AudioChunk
}

// AudioPlayer plays short notification sounds.
type AudioPlayer interface {
	// PlayStart plays the recording-start sound (non-blocking).
	PlayStart() error

	// PlayStop plays the recording-stop / success sound (non-blocking).
	PlayStop() error

	// PlayCancel plays the cancel sound (non-blocking).
	PlayCancel() error

	// SetEnabled enables or disables all sounds.
	SetEnabled(enabled bool)

	// SetVolume sets master volume in the range [0.0, 1.0].
	SetVolume(volume float64)

	// Close releases audio output resources.
	Close() error
}

// DeviceEnumerator lists available audio input devices.
type DeviceEnumerator interface {
	// ListInputDevices returns all available microphone devices.
	ListInputDevices() ([]DeviceInfo, error)

	// DefaultInputDevice returns the system-default microphone.
	DefaultInputDevice() (*DeviceInfo, error)
}

// NewRecorder creates a new AudioRecorder backed by malgo.
func NewRecorder() AudioRecorder { return &recorder{} }

// NewPlayer creates a new AudioPlayer backed by malgo.
func NewPlayer() AudioPlayer { return &player{enabled: true, volume: 1.0} }

// NewEnumerator creates a new DeviceEnumerator backed by malgo.
func NewEnumerator() DeviceEnumerator { return &enumerator{} }
