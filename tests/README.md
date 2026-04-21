# Tests

| Layer | Tool | Location |
|-------|------|----------|
| Core unit tests | `go test ./...` | `core/**/*_test.go` |
| Frontend unit | Vitest | `frontend/src/**/*.test.ts` |
| E2E | Playwright + Tauri | `tests/e2e/` |

## Running Tests

```bash
# Core unit tests
cd core && go test ./...

# E2E tests (requires built app)
cd tests/e2e && npx playwright test
```

Coverage target: >92% for core library.
