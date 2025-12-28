# End-to-End Tests

This directory contains end-to-end (e2e) tests for the Educartable Sync application using WebdriverIO and tauri-driver.

## Overview

The e2e testing setup uses:
- **WebdriverIO** - Modern test automation framework
- **tauri-driver** - WebDriver server for Tauri applications
- **Mocha** - Test framework for organizing and running tests
- **WebKitWebDriver** (Linux) / **Microsoft Edge Driver** (Windows) - Platform-specific WebDriver implementations

## Prerequisites

### All Platforms

1. Install `tauri-driver`:
   ```bash
   cargo install tauri-driver --locked
   ```

2. Ensure the application can be built:
   ```bash
   cd ../src-tauri
   cargo build
   ```

### Linux-Specific

Linux uses `WebKitWebDriver` which is usually bundled with the WebKit package. On Debian-based systems, you may need to install it separately:

```bash
sudo apt-get install webkit2gtk-driver
```

Verify it's available:
```bash
which WebKitWebDriver
```

### Windows-Specific

Windows requires Microsoft Edge Driver matching your Edge version. Use the `msedgedriver-tool` to download the correct version:

```bash
cargo install --git https://github.com/chippers/msedgedriver-tool
msedgedriver-tool
```

Ensure `msedgedriver.exe` is in your PATH or use the `--native-driver` flag with `tauri-driver`.

### macOS

**Note:** macOS is not currently supported for Tauri WebDriver testing on desktop due to the lack of a WKWebView driver tool. For mobile (iOS), use Appium 2 (not covered in this setup).

## Installation

Install the test dependencies:

```bash
cd e2e-tests
npm install
```

## Running Tests

To run all e2e tests:

```bash
npm test
```

This will:
1. Build the Tauri application in debug mode (via `onPrepare` hook)
2. Start `tauri-driver` on port 4444 (via `beforeSession` hook)
3. Launch the application and run all test specs
4. Clean up and stop `tauri-driver` (via `afterSession` hook)

## Test Structure

```
e2e-tests/
├── package.json              # Dependencies and scripts
├── wdio.conf.js             # WebdriverIO configuration
├── test/
│   └── specs/
│       └── app.e2e.js       # Test specifications
└── README.md                # This file
```

## Writing Tests

Tests are written using WebdriverIO's API with Mocha's BDD syntax. Example:

```javascript
import { expect } from '@wdio/globals'

describe('My Feature', () => {
  it('should do something', async () => {
    // Find an element
    const button = await $('#my-button')

    // Wait for it to exist
    await button.waitForExist({ timeout: 5000 })

    // Interact with it
    await button.click()

    // Make assertions
    const text = await button.getText()
    expect(text).toContain('Expected Text')
  })
})
```

### Available Commands

- `$('selector')` - Find single element by CSS selector
- `$$('selector')` - Find multiple elements
- `element.click()` - Click an element
- `element.getText()` - Get element text content
- `element.getValue()` - Get input value
- `element.isDisplayed()` - Check if element is visible
- `element.isEnabled()` - Check if element is enabled
- `element.waitForExist()` - Wait for element to exist
- `browser.pause(ms)` - Pause execution

See [WebdriverIO API docs](https://webdriver.io/docs/api) for more commands.

## Configuration

The `wdio.conf.js` file contains:

- **hostname/port**: Connect to tauri-driver at `127.0.0.1:4444`
- **specs**: Test files location (`./test/specs/**/*.js`)
- **capabilities**: Tauri-specific options including binary path
- **framework**: Mocha with 60-second timeout
- **reporters**: Spec reporter for console output
- **hooks**:
  - `onPrepare`: Builds the debug binary
  - `beforeSession`: Starts tauri-driver
  - `afterSession`: Stops tauri-driver

## Troubleshooting

### Tests hang or timeout

- Ensure `tauri-driver` is not already running: `ps aux | grep tauri-driver`
- Kill any orphaned processes: `pkill tauri-driver`
- Check that port 4444 is not in use: `netstat -an | grep 4444`

### Application doesn't launch

- Verify the binary path in `wdio.conf.js` matches your build output
- Ensure the application builds successfully: `cargo build --manifest-path=../src-tauri/Cargo.toml`
- Check for errors in the test output

### Element not found errors

- Increase timeouts in `waitForExist()` calls
- Verify the CSS selector matches the element in your HTML
- Check if the element is in a shadow DOM (requires different selectors)

### Windows-specific issues

- Ensure Edge Driver version matches your Edge browser version
- Verify `msedgedriver.exe` is in your PATH
- Try running as administrator if you encounter permission issues

## CI/CD Integration

See the main `.github/workflows/ci.yml` file for CI integration. The e2e tests require:
- Platform-specific WebDriver installation
- Rust toolchain for building
- Display server on Linux (handled automatically via Xvfb)

## Resources

- [Tauri WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Documentation](https://webdriver.io/docs/gettingstarted)
- [Mocha Documentation](https://mochajs.org/)
