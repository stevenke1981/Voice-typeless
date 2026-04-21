package audio

import (
	"errors"
	"sync"
	"sync/atomic"
	"time"
	// "github.com/gen2brain/malgo"  // TODO: enable when native libs are present
)

// recorder implements AudioRecorder using malgo/miniaudio.
type recorder struct {
	mu        sync.Mutex
	recording atomic.Bool
	startedAt time.Time
	buf       []float32
	subs      []chan AudioChunk
	cfg       RecorderConfig
}

func (r *recorder) Start(cfg RecorderConfig) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.recording.Load() {
		return ErrAlreadyRecording
	}
	r.buf = r.buf[:0]
	r.startedAt = time.Now()
	r.cfg = cfg
	r.recording.Store(true)
	// TODO: open malgo device and stream PCM chunks.
	// On each chunk: append to r.buf and push AudioChunk to r.subs.
	return nil
}

func (r *recorder) Stop() error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if !r.recording.Load() {
		return ErrNotRecording
	}
	r.recording.Store(false)
	r.closeSubs()
	// TODO: flush malgo device buffer.
	return nil
}

func (r *recorder) Cancel() {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.recording.Store(false)
	r.buf = r.buf[:0]
	r.closeSubs()
}

func (r *recorder) Drain() (*AudioChunk, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.recording.Load() {
		return nil, errors.New("audio: Drain called while recording is still active; call Stop first")
	}
	samples := make([]float32, len(r.buf))
	copy(samples, r.buf)
	return &AudioChunk{
		Samples:    samples,
		SampleRate: r.cfg.SampleRate,
		CapturedAt: r.startedAt,
	}, nil
}

func (r *recorder) Subscribe() <-chan AudioChunk {
	ch := make(chan AudioChunk, 32)
	r.mu.Lock()
	r.subs = append(r.subs, ch)
	r.mu.Unlock()
	return ch
}

// closeSubs closes all subscriber channels (must be called with r.mu held).
func (r *recorder) closeSubs() {
	for _, ch := range r.subs {
		close(ch)
	}
	r.subs = r.subs[:0]
}

// player is a stub AudioPlayer implementation.
type player struct {
	mu      sync.Mutex
	enabled bool
	volume  float64
}

func (p *player) PlayStart() error  { return nil /* TODO: play sound */ }
func (p *player) PlayStop() error   { return nil /* TODO: play sound */ }
func (p *player) PlayCancel() error { return nil /* TODO: play sound */ }

func (p *player) SetEnabled(enabled bool) {
	p.mu.Lock()
	p.enabled = enabled
	p.mu.Unlock()
}

func (p *player) SetVolume(volume float64) {
	p.mu.Lock()
	p.volume = volume
	p.mu.Unlock()
}

func (p *player) Close() error { return nil }

// enumerator is a stub DeviceEnumerator implementation.
type enumerator struct{}

func (e *enumerator) ListInputDevices() ([]DeviceInfo, error) {
	// TODO: enumerate via malgo.
	return []DeviceInfo{{ID: "default", Name: "Default Microphone", IsDefault: true, Channels: 1, SampleRates: []int{16000, 44100, 48000}}}, nil
}

func (e *enumerator) DefaultInputDevice() (*DeviceInfo, error) {
	devices, err := e.ListInputDevices()
	if err != nil || len(devices) == 0 {
		return nil, err
	}
	for i := range devices {
		if devices[i].IsDefault {
			return &devices[i], nil
		}
	}
	return &devices[0], nil
}

var (
	// ErrAlreadyRecording is returned by Start when a recording session is already active.
	ErrAlreadyRecording = errors.New("audio: recording already in progress")
	// ErrNotRecording is returned by Stop when no recording session is active.
	ErrNotRecording = errors.New("audio: no active recording session")
)
