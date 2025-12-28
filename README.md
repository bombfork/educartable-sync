# Educartable Sync

Desktop application for batch downloading and synchronizing photos from Educartable to your computer.

## Development

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Testing

#### Unit Tests

Run Rust unit tests:

```bash
cd src-tauri
cargo test
```

#### End-to-End Tests

Run e2e tests with WebdriverIO:

```bash
cd e2e-tests
npm install
npm test
```

See [e2e-tests/README.md](e2e-tests/README.md) for detailed setup and usage instructions.
