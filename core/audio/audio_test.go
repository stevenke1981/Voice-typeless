package audio_test

import (
	"testing"

	"github.com/vtl/core/audio"
)

func TestNewRecorder_NotNil(t *testing.T) {
	rec := audio.NewRecorder()
	if rec == nil {
		t.Fatal("NewRecorder returned nil")
	}
}

func TestNewEnumerator_ListDevices(t *testing.T) {
	enum := audio.NewEnumerator()
	devices, err := enum.ListInputDevices()
	if err != nil {
		t.Fatal(err)
	}
	if len(devices) == 0 {
		t.Error("expected at least one device")
	}
}

func TestIsSpeech_Silence(t *testing.T) {
	cfg := audio.DefaultVADConfig()
	silence := make([]float32, 1024)
	if audio.IsSpeech(silence, cfg) {
		t.Error("expected silence to not be classified as speech")
	}
}

func TestIsSpeech_Signal(t *testing.T) {
	cfg := audio.DefaultVADConfig()
	signal := make([]float32, 1024)
	for i := range signal {
		signal[i] = 0.5
	}
	if !audio.IsSpeech(signal, cfg) {
		t.Error("expected loud signal to be classified as speech")
	}
}

func TestSampleRateConstant(t *testing.T) {
	if audio.SampleRate != 16000 {
		t.Errorf("expected SampleRate=16000, got %d", audio.SampleRate)
	}
}
