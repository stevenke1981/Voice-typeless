# Tests

| Layer | Tool | Location |
|-------|------|----------|
| Core unit and integration tests | Cargo | `core-rs/src/**` and `core-rs/tests/**` |
| Frontend validation | Svelte Check | `frontend/src/**` |
| E2E | Playwright + Tauri | `tests/e2e/` |

## Running Tests

```powershell
# Core unit tests
Push-Location core-rs
cargo test
Pop-Location

# Frontend type and Svelte validation
Push-Location frontend
npm run check
Pop-Location

# E2E tests (requires built app)
Push-Location tests/e2e
npx playwright test
Pop-Location
```

Coverage target: >92% for core library.
