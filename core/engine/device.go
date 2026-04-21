package engine

import (
	"errors"
	"runtime"
)

// ErrModelNotLoaded is returned when Recognize is called before LoadModel.
var ErrModelNotLoaded = errors.New("engine: model not loaded; call LoadModel first")

// ProbeDevice auto-selects the best available inference device.
// Priority: DirectML (Windows 10+) → CUDA → CPU
func ProbeDevice() DeviceType {
	if runtime.GOOS == "windows" && isDirectMLAvailable() {
		return DeviceDirectML
	}
	if isCUDAAvailable() {
		return DeviceCUDA
	}
	return DeviceCPU
}

func isDirectMLAvailable() bool {
	// TODO: check Windows version >= 10 and DirectML DLL presence
	return false
}

func isCUDAAvailable() bool {
	// TODO: check CUDA runtime DLL (nvcuda.dll / libcuda.so)
	return false
}
