# Testing Guide

This document describes the testing setup for the Educartable Sync project.

## Testing Types

### Frontend Unit Tests (Vitest)

Located in `src/__tests__/`, these tests cover frontend JavaScript code with Tauri API mocking support.

**Run frontend unit tests:**
```bash
npm test              # Watch mode
npm run test:run      # Single run
npm run test:ui       # Interactive UI
npm run coverage      # Generate coverage report
```

**Technology Stack:**
- Test Runner: [Vitest](https://vitest.dev/) v2.1.8
- DOM Environment: [happy-dom](https://github.com/capricorn86/happy-dom) v15.11.7
- Tauri Mocking: [@tauri-apps/api](https://v2.tauri.app/develop/tests/mocking/) v2.2.0

**Documentation:** See `src/__tests__/README.md` for detailed usage examples

### Backend Unit Tests (Rust)

Located in `src-tauri/src/`, these tests cover Rust backend code.

**Run backend unit tests:**
```bash
cd src-tauri
cargo test
```

### End-to-End Tests (WebdriverIO)

Located in `e2e-tests/`, these tests cover full application workflows.

**Run E2E tests:**
```bash
cd e2e-tests
npm test
```

## Configuration Files

- `vite.config.ts` - Vitest configuration for frontend tests
- `src/__tests__/setup.ts` - Global test setup and mocks
- `src-tauri/Cargo.toml` - Rust test dependencies
- `e2e-tests/wdio.conf.js` - WebdriverIO configuration

## Quick Start: Writing Your First Test

1. Create a test file in `src/__tests__/`:
   ```typescript
   // src/__tests__/myFeature.test.ts
   import { describe, it, expect } from 'vitest';

   describe('My Feature', () => {
     it('should work correctly', () => {
       expect(1 + 1).toBe(2);
     });
   });
   ```

2. Run the test:
   ```bash
   npm test
   ```

3. See `src/__tests__/sample.test.ts` for more examples including Tauri API mocking.

## CI/CD

Tests are automatically run on GitHub Actions for pull requests. See `.github/workflows/` for workflow configurations.

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [Tauri v2 Testing Guide](https://v2.tauri.app/develop/tests/mocking/)
- [WebdriverIO Documentation](https://webdriver.io/)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
