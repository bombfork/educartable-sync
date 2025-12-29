# Frontend Unit Tests

This directory contains unit tests for the Educartable Sync frontend.

## Setup

Vitest is configured for testing with:
- **Test Runner**: Vitest
- **DOM Environment**: happy-dom (provides browser-like APIs)
- **Tauri Mocking**: @tauri-apps/api/mocks

## Running Tests

```bash
# Run tests in watch mode (interactive)
npm test

# Run tests once
npm run test:run

# Run tests with UI
npm run test:ui

# Generate coverage report
npm run coverage
```

## Writing Tests

### Basic Test Structure

```typescript
import { describe, it, expect } from 'vitest';

describe('Feature Name', () => {
  it('should do something', () => {
    expect(true).toBe(true);
  });
});
```

### Mocking Tauri APIs

To mock Tauri IPC calls in your tests:

```typescript
import { mockIPC } from '@tauri-apps/api/mocks';
import { invoke } from '@tauri-apps/api/core';

describe('My Feature', () => {
  it('should call Tauri command', async () => {
    // Setup mock
    mockIPC((cmd, args) => {
      if (cmd === 'my_command') {
        return { result: 'success' };
      }
    });

    // Call the Tauri command
    const result = await invoke('my_command');

    // Assert
    expect(result).toEqual({ result: 'success' });
  });
});
```

### Testing DOM Manipulation

```typescript
import { describe, it, expect } from 'vitest';

describe('DOM Tests', () => {
  it('should manipulate DOM', () => {
    document.body.innerHTML = '<button id="myBtn">Click</button>';
    const button = document.querySelector('#myBtn');
    expect(button?.textContent).toBe('Click');
  });
});
```

### Using Spies and Mocks

```typescript
import { describe, it, expect, vi } from 'vitest';

describe('Function Mocking', () => {
  it('should mock a function', () => {
    const mockFn = vi.fn();
    mockFn('hello');

    expect(mockFn).toHaveBeenCalledWith('hello');
    expect(mockFn).toHaveBeenCalledTimes(1);
  });
});
```

## Test File Location

Place test files adjacent to the code they test:
- `src/main.js` → `src/__tests__/main.test.js`
- `src/utils/helper.js` → `src/__tests__/utils/helper.test.js`

Or use the naming convention:
- `src/main.test.js` (next to `src/main.js`)

## Configuration

The test configuration is in `/vite.config.ts` and can be customized for:
- Test environment settings
- Coverage thresholds
- Setup files
- Include/exclude patterns

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [Tauri v2 Testing Guide](https://v2.tauri.app/develop/tests/mocking/)
- [Happy DOM Documentation](https://github.com/capricorn86/happy-dom)
