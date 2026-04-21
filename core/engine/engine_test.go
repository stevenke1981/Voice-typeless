package engine_test

import (
	"testing"
	"time"

	"github.com/vtl/core/engine"
)

func TestRecognitionResult(t *testing.T) {
	r := engine.RecognitionResult{
		Text:       "hello world",
		Language:   "en",
		Confidence: 0.95,
		Duration:   500 * time.Millisecond,
	}
	if r.Text == "" {
		t.Fatal("expected non-empty Text")
	}
	if r.Confidence < 0 || r.Confidence > 1 {
		t.Errorf("confidence %v out of [0,1]", r.Confidence)
	}
}

func TestSenseVoiceEngine_NotLoaded(t *testing.T) {
	eng := engine.NewSenseVoiceEngine()
	_, err := eng.Recognize([]float32{0, 0, 0}, 16000)
	if err == nil {
		t.Fatal("expected error when recognizing before LoadModel")
	}
}

func TestModelTypeConstants(t *testing.T) {
	types := []engine.ModelType{
		engine.ModelSenseVoice,
		engine.ModelWhisperTiny,
		engine.ModelCustomONNX,
	}
	for _, mt := range types {
		if mt == "" {
			t.Error("ModelType must not be empty")
		}
	}
}

func TestNew_UnknownModelType(t *testing.T) {
	_, err := engine.New("unknown-model")
	if err == nil {
		t.Fatal("expected error for unknown model type")
	}
}

func TestNew_SenseVoice(t *testing.T) {
	eng, err := engine.New(engine.ModelSenseVoice)
	if err != nil {
		t.Fatal(err)
	}
	if eng == nil {
		t.Fatal("expected non-nil engine")
	}
}

func TestSegment(t *testing.T) {
	seg := engine.Segment{
		Text:  "hello",
		Start: 0,
		End:   500 * time.Millisecond,
	}
	if seg.End <= seg.Start {
		t.Error("Segment End must be after Start")
	}
}
