# VTL Plugin System

Plugins are post-processing scripts that transform recognized text before it is pasted.

## Plugin API (Planned)

```javascript
// my-plugin.js
export function onRecognitionResult(text) {
  // Transform and return modified text
  return text.toUpperCase();
}
```

See `docs/architecture.md` for the full plugin specification.
