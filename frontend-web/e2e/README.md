# E2E Tests

This directory contains end-to-end tests using Playwright.

## Running Tests

```bash
# Run all E2E tests
npm run test:e2e

# Run with UI
npm run test:e2e:ui

# Debug mode
npm run test:e2e:debug
```

## Test Structure

- `app.spec.ts` - Application-level tests (navigation, responsive design)
- Add more test files for specific features:
  - `sessions.spec.ts` - Session management
  - `chat.spec.ts` - Chat interface
  - `auth.spec.ts` - Authentication flow
