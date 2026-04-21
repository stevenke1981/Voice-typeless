# VTL Tauri Command Reference

> **Version**: 0.2.0  
> **Updated**: 2026-04-21  
> **Source**: `src-tauri/src/commands.rs`

All commands are invoked from the Svelte frontend using the Tauri `invoke()` API:

```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke<ReturnType>("command_name", { param: value });
```

Errors are thrown as `string` values (Tauri v2 convention). Always wrap calls in `try/catch`.

---

## Table of Contents

1. [Config Commands](#1-config-commands)
2. [History Commands](#2-history-commands)
3. [Statistics](#3-statistics)
4. [Demo Mode](#4-demo-mode)
5. [Autostart](#5-autostart)
6. [Recording Commands](#6-recording-commands)
7. [Device Commands](#7-device-commands)
8. [Types Reference](#8-types-reference)
9. [Events Reference](#9-events-reference)
10. [Error Handling](#10-error-handling)

---

## 1. Config Commands

### `get_config`

Returns the full application configuration. Reads from
`%APPDATA%\Roaming\VoiceTypeless\config.json`. If the file does not exist, returns the
built-in default configuration.

**Parameters**: none

**Returns**: [`AppConfig`](#appconfig)

```typescript
const config = await invoke<AppConfig>("get_config");
console.log(config.ui.theme);     // "dark" | "light" | "system"
console.log(config.system.autoStart); // false
```

---

### `set_config`

Persists a partial or full configuration update to disk. Merges the provided object into the
existing config — unspecified fields are left unchanged.

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config` | `Partial<AppConfig>` | ✅ | Fields to update. Supports nested partial objects. |

**Returns**: `void`

```typescript
// Change theme only
await invoke("set_config", {
  config: { ui: { theme: "dark" } },
});

// Enable autostart and change log level in one call
await invoke("set_config", {
  config: {
    system: { autoStart: true, logLevel: "debug" },
  },
});
```

**Errors**:

| Error String | Cause |
|--------------|-------|
| `"CONFIG_WRITE_FAILED: {detail}"` | Disk write failed (permissions or disk full) |
| `"CONFIG_INVALID: {detail}"` | Payload contains an unrecognised field or invalid type |

---

## 2. History Commands

### `get_history`

Returns recognition history items sorted newest-first.

**Parameters**:

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `limit` | `number` | ❌ | `50` | Maximum items to return. Pass `0` to return all items. |

**Returns**: [`HistoryItem[]`](#historyitem)

```typescript
// Last 20 items
const recent = await invoke<HistoryItem[]>("get_history", { limit: 20 });

// All items (use with caution — may be large)
const all = await invoke<HistoryItem[]>("get_history", { limit: 0 });
```

---

### `delete_history_item`

Deletes a single history entry by its ID.

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `id` | `string` | ✅ | The `id` field from a [`HistoryItem`](#historyitem). |

**Returns**: `void`

```typescript
await invoke("delete_history_item", { id: "item_abc123" });
```

**Errors**:

| Error String | Cause |
|--------------|-------|
| `"HISTORY_ITEM_NOT_FOUND: {id}"` | No item exists with the given ID |
| `"HISTORY_WRITE_FAILED: {detail}"` | Disk write failed after deletion |

---

### `clear_history`

Deletes **all** history items. This operation is permanent and cannot be undone.

**Parameters**: none

**Returns**: `void`

```typescript
// Always ask for confirmation before calling this
if (await confirm("Clear all history? This cannot be undone.")) {
  await invoke("clear_history");
}
```

---

### `export_history_text`

Returns all history items formatted as plain text. Each line contains a timestamp followed by
the transcription result. Use this to copy history to the clipboard or save it to a file.

**Parameters**: none

**Returns**: `string`

**Output format** (one entry per line):

```
[2026-04-21 14:32:01] Hello, this is a test recording.
[2026-04-21 14:35:44] Another transcription result here.
```

```typescript
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const text = await invoke<string>("export_history_text");
await writeText(text); // Writes to system clipboard
```

---

## 3. Statistics

### `get_stats`

Returns aggregated statistics computed from the history store. Values are re-computed on
every call (no caching).

**Parameters**: none

**Returns**: [`AppStats`](#appstats)

```typescript
const stats = await invoke<AppStats>("get_stats");
console.log(`Total recordings: ${stats.total_recordings}`);
console.log(`Characters typed:  ${stats.total_characters}`);
// Language breakdown:
Object.entries(stats.languages).forEach(([lang, count]) => {
  console.log(`  ${lang}: ${count} recordings`);
});
```

---

## 4. Demo Mode

### `run_demo`

Simulates a complete recording and transcription cycle **without** requiring a microphone or a
loaded speech model. Useful for UI development, onboarding, and testing the history and
statistics features.

**Parameters**: none

**Returns**: [`RecognitionResult`](#recognitionresult)

**Simulated sequence**:
1. Emits `recording-started` event immediately
2. Waits ~1.5 s (simulating audio capture)
3. Emits `recording-stopped` event
4. Returns a hardcoded `RecognitionResult`
5. Appends the result to `history.json` (persisted to disk)

```typescript
const result = await invoke<RecognitionResult>("run_demo");
console.log(result.text);
// "This is a demo transcription result."
```

> **Note**: Demo results are indistinguishable from real results in the history store. They
> count toward statistics. Delete them with `clear_history` or individually via
> `delete_history_item` if needed.

---

## 5. Autostart

### `get_autostart_enabled`

Returns whether VTL is configured to launch at Windows startup. Reads the registry key
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run\VoiceTypeless`.

**Parameters**: none

**Returns**: `boolean`

```typescript
const enabled = await invoke<boolean>("get_autostart_enabled");
console.log(enabled ? "Autostart is ON" : "Autostart is OFF");
```

---

### `set_autostart_enabled`

Enables or disables Windows autostart by adding or removing the registry entry.

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `enabled` | `boolean` | ✅ | `true` to enable; `false` to disable. |

**Returns**: `void`

```typescript
// Enable autostart
await invoke("set_autostart_enabled", { enabled: true });

// Keep config.json in sync
await invoke("set_config", { config: { system: { autoStart: true } } });
```

**Errors**:

| Error String | Cause |
|--------------|-------|
| `"AUTOSTART_REGISTRY_ERROR: {detail}"` | Registry access failed |
| `"AUTOSTART_NOT_SUPPORTED"` | Non-Windows platform |

> **Important**: `set_autostart_enabled` only modifies the Windows registry. Call
> `set_config` in parallel to keep `config.json` consistent with the registry state.

---

## 6. Recording Commands

> ⚠️ **Stub status in v0.2.0** — These commands require Go Core integration (speech model
> loaded) to produce real results. All stubs return placeholder values. Use
> [`run_demo`](#run_demo) for UI testing until the Go Core sidecar is integrated.

### `start_recording`

Begins audio capture using the configured microphone.

**Parameters**:

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `mode` | `"push_to_talk" \| "free_speech"` | ✅ | `push_to_talk`: records until `stop_recording` is called. `free_speech`: auto-stops after configurable silence. |

**Returns**: `void`

Emits: [`recording-started`](#recording-started) on success.

```typescript
await invoke("start_recording", { mode: "push_to_talk" });
// Now wait for user to release the hotkey, then call stop_recording()
```

**Errors**:

| Error String | Cause |
|--------------|-------|
| `"AUDIO_DEVICE_ERROR"` | Microphone unavailable or permission denied |
| `"MODEL_NOT_LOADED"` | No speech model is loaded — call `switch_model` first |
| `"ALREADY_RECORDING"` | A recording session is already active |

---

### `stop_recording`

Stops audio capture, runs speech inference, and returns the transcription result. The result
is also persisted to `history.json` automatically.

**Parameters**: none

**Returns**: [`RecognitionResult`](#recognitionresult)

Emits (in order):
1. [`recording-stopped`](#recording-stopped) — immediately when capture ends
2. [`recognition-result`](#recognition-result) — when inference completes

```typescript
const result = await invoke<RecognitionResult>("stop_recording");
console.log(`Transcribed: "${result.text}" (${result.language}, ${result.confidence * 100}%)`);
```

**Errors**:

| Error String | Cause |
|--------------|-------|
| `"NOT_RECORDING"` | No active recording session to stop |
| `"INFERENCE_TIMEOUT"` | Inference exceeded 10 seconds |
| `"AUDIO_DEVICE_ERROR"` | Microphone failed during capture |

---

### `cancel_recording`

Cancels the active recording session and discards all captured audio. No inference is run and
nothing is written to history.

**Parameters**: none

**Returns**: `void`

Emits: `recording-cancelled`

```typescript
await invoke("cancel_recording");
```

---

## 7. Device Commands

### `get_devices`

Returns all available audio input devices and the currently active device ID.

**Parameters**: none

**Returns**: [`DeviceList`](#devicelist)

```typescript
const { devices, active_device_id } = await invoke<DeviceList>("get_devices");

devices.forEach((d) => {
  const marker = d.id === active_device_id ? " ← active" : "";
  const def = d.is_default ? " (system default)" : "";
  console.log(`${d.name}${def}${marker}`);
});
```

---

## 8. Types Reference

### `AppConfig`

```typescript
interface AppConfig {
  version: number;          // Config schema version — used for migrations
  hotkey: HotkeyConfig;
  audio: AudioConfig;
  model: ModelConfig;
  text: TextConfig;
  ui: UIConfig;
  system: SystemConfig;
}

interface UIConfig {
  theme: "dark" | "light" | "system";
  language: "zh" | "en";              // UI display language
  showFloatingIndicator: boolean;
  indicatorPosition: { x: number; y: number; displayId?: string };
  historyRetentionDays: number;       // 0 = keep forever
  maxHistoryItems: number;            // Default: 50
}

interface SystemConfig {
  autoStart: boolean;
  minimizeToTray: boolean;
  checkUpdates: boolean;
  logLevel: "debug" | "info" | "warn" | "error";
}
```

For the full `AppConfig` definition including `HotkeyConfig`, `AudioConfig`, `ModelConfig`,
and `TextConfig`, see [`architecture.md §6`](architecture.md#6-configuration-schema).

---

### `HistoryItem`

```typescript
interface HistoryItem {
  id: string;           // UUID — stable identifier for delete operations
  text: string;         // Transcribed and post-processed text
  language: string;     // Detected language code: "en", "zh", "ja", etc.
  confidence: number;   // Model confidence score: 0.0–1.0
  duration_ms: number;  // Length of the recorded audio in milliseconds
  created_at: number;   // Unix timestamp in milliseconds
}
```

---

### `AppStats`

```typescript
interface AppStats {
  total_recordings: number;           // Total number of history entries
  total_characters: number;           // Sum of text.length across all entries
  languages: Record<string, number>;  // e.g. { "en": 42, "zh": 17, "ja": 3 }
  total_duration_ms: number;          // Sum of all recording durations in ms
}
```

---

### `RecognitionResult`

```typescript
interface RecognitionResult {
  text: string;         // Final transcribed and post-processed text
  language: string;     // Detected language code
  confidence: number;   // 0.0–1.0
  duration_ms: number;  // Length of the recorded audio in milliseconds
  segments?: Segment[]; // Word-level timestamps (future — when streaming enabled)
}

interface Segment {
  text: string;
  start_ms: number;
  end_ms: number;
}
```

---

### `DeviceList`

```typescript
interface DeviceList {
  devices: DeviceInfo[];
  active_device_id: string;
}

interface DeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}
```

---

## 9. Events Reference

The Go Core sidecar emits events over the IPC channel; the Rust layer re-emits them as Tauri
global events. Subscribe using `@tauri-apps/api/event`:

```typescript
import { listen } from "@tauri-apps/api/event";
```

### `recording-started`

Emitted by `start_recording` and `run_demo` when capture begins.

```typescript
interface RecordingStartedPayload { timestamp: number; } // Unix ms

const unlisten = await listen<RecordingStartedPayload>(
  "recording-started",
  (e) => showFloatingIndicator(e.payload.timestamp)
);
```

---

### `recording-stopped`

Emitted when capture ends (before inference completes).

```typescript
interface RecordingStoppedPayload { duration_ms: number; }
```

---

### `recognition-result`

Emitted when inference completes and the result is ready.

```typescript
interface RecognitionResultPayload {
  text: string;
  language: string;
  confidence: number;
  segments?: Array<{ text: string; start_ms: number; end_ms: number }>;
}
```

---

### `model-loading`

Emitted during model download or initialisation.

```typescript
interface ModelLoadingPayload {
  progress: number;                     // 0.0–1.0
  stage: "download" | "load" | "warmup";
}
```

---

### `model-ready`

Emitted when the speech model is fully loaded and warmed up.

```typescript
interface ModelReadyPayload {
  modelId: string;
  device: "directml" | "cuda" | "cpu";
}
```

---

## 10. Error Handling

All Tauri commands throw `string` errors when they fail. The error string format is:

```
"ERROR_CODE: human-readable detail"
```

```typescript
try {
  await invoke("set_config", { config: updates });
} catch (err: unknown) {
  // err is a string in Tauri v2
  const message = typeof err === "string" ? err : String(err);
  console.error("Config save failed:", message);
  showToast(`Failed to save settings: ${message}`);
}
```

For the complete error code registry, see
[Appendix A of architecture.md](architecture.md#appendix-a-error-code-reference).
