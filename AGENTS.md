# AGENTS.md - LLM Agent Instructions

## Project Overview

**Educartable Sync** is a Tauri v2 desktop application that synchronizes photos and activity content from the Educartable educational platform to parents' computers.

### Critical Context

- **Single integration**: Educartable is the **only** third-party service. No other platforms will ever be integrated. Do not design for extensibility to other services.
- **French only**: No internationalization (i18n) is needed or planned. All UI text is hardcoded in French.
- **Non-technical users**: Parents using this app are not tech-savvy. Prioritize simplicity, clear error messages, and foolproof UX over advanced features.

## Architecture

```
educartable-sync/
├── src/                    # Frontend (vanilla JS, HTML, CSS)
│   ├── index.html          # Main UI
│   ├── main.js             # App logic & Tauri IPC
│   ├── notifications.js    # Toast notification system
│   ├── updater.js          # Auto-update UI handler
│   ├── styles.css          # Custom styles (Pico CSS framework)
│   └── __tests__/          # Vitest frontend tests
│
└── src-tauri/              # Rust backend
    ├── src/
    │   ├── lib.rs          # Tauri app setup, command registration
    │   ├── auth.rs         # OAuth via Educartable Keycloak
    │   ├── config.rs       # User preferences (sync folder path)
    │   ├── api.rs          # Educartable API client
    │   ├── sync.rs         # File download & organization
    │   ├── models.rs       # Data structures
    │   └── updater.rs      # Auto-update logic
    └── tauri.conf.json     # Tauri configuration
```

## Technology Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| Framework | Tauri v2 | Desktop app with web frontend |
| Backend | Rust | Async via Tokio |
| Frontend | Vanilla JS | No framework, ES6 modules |
| CSS | Pico CSS | Minimal framework, local bundle |
| HTTP | reqwest | API calls to Educartable |
| Auth | OAuth 2.0 | Keycloak via embedded webview |
| Credentials | keyring | System-native secure storage |
| Testing | Vitest + cargo test | Frontend & backend unit tests |

## Development Commands

```bash
# Run development build
cargo tauri dev

# Run with debug logging
RUST_LOG=debug cargo tauri dev

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run frontend tests
npm run test:run

# Production build
cargo tauri build

# Using mise task runner
mise run dev              # Development mode
mise run test             # Rust tests
mise run front            # Frontend tests
mise run check            # Lint & format
```

## Tauri Commands (IPC Interface)

All frontend-backend communication goes through these commands:

| Command | Purpose |
|---------|---------|
| `authenticate()` | Open OAuth login webview |
| `logout()` | Clear stored credentials |
| `is_authenticated()` | Check auth status |
| `load_config()` | Get sync folder path |
| `save_config(config)` | Store sync folder path |
| `select_sync_directory()` | Open native folder picker |
| `start_sync()` | Download activities & media |
| `open_logs_directory()` | Open log folder in file manager |
| `check_for_updates()` | Query update server |
| `download_and_install_update()` | Install available update |

## Key Design Principles

### 1. Simplicity Over Flexibility

```rust
// BAD: Generic, over-engineered
pub struct SyncEngine<S: Service, C: Cache, F: FileSystem> { ... }

// GOOD: Direct, single-purpose
pub async fn start_sync(app_handle: AppHandle) -> Result<SyncStats, String>
```

The app does one thing: sync from Educartable. No plugins, no service abstraction layers.

### 2. User-Friendly Error Messages

```rust
// BAD: Technical error
Err(format!("HTTP 401: {}", e))

// GOOD: Actionable message in French
Err("Votre session a expiré. Veuillez vous reconnecter.".to_string())
```

All errors should be understandable by a non-technical parent.

### 3. Defensive UX

- Disable buttons during operations to prevent double-clicks
- Show clear loading states during sync
- Auto-save configuration changes
- Provide visual feedback for every action

### 4. No Configuration Complexity

The only user setting is the sync folder path. Do not add:
- Advanced sync options
- Filtering capabilities
- Scheduling features
- Performance tuning knobs

## Code Patterns

### Adding a Tauri Command

1. **Define in Rust** (e.g., `src-tauri/src/sync.rs`):
```rust
#[tauri::command]
pub async fn my_command(app_handle: AppHandle) -> Result<MyResult, String> {
    // Implementation
}
```

2. **Register in lib.rs**:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    sync::my_command,
])
```

3. **Call from frontend** (`src/main.js`):
```javascript
const result = await invoke('my_command');
```

### Error Handling Pattern

```rust
// Backend: Convert errors to user-friendly strings
pub async fn start_sync(...) -> Result<SyncStats, String> {
    client.fetch_activities().await.map_err(|e| {
        log::error!("Failed to fetch activities: {e}");
        "Impossible de récupérer les activités. Vérifiez votre connexion.".to_string()
    })?;
}
```

```javascript
// Frontend: Display errors via notification system
try {
    await invoke('start_sync');
} catch (error) {
    showError('Erreur de synchronisation', String(error));
}
```

### Testing Pattern

**Rust tests** (inline in source files):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_folder_name() {
        let activity = Activity { title: "Test / Activity".into(), ... };
        let folder = get_activity_folder(&activity);
        assert!(!folder.contains('/'));  // Sanitized
    }
}
```

**Frontend tests** (`src/__tests__/*.test.ts`):
```typescript
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';

describe('Sync', () => {
    beforeEach(() => clearMocks());

    it('should handle sync completion', async () => {
        mockIPC((cmd) => {
            if (cmd === 'start_sync') {
                return { downloaded: 10, skipped: 2, failed: 0 };
            }
        });
        // Test UI updates
    });
});
```

## File Organization Rules

| Location | Content |
|----------|---------|
| `src-tauri/src/auth.rs` | OAuth flow, token storage, credential management |
| `src-tauri/src/api.rs` | Educartable API client, HTTP requests |
| `src-tauri/src/sync.rs` | Download logic, file organization, progress tracking |
| `src-tauri/src/config.rs` | Config file I/O, directory dialogs |
| `src-tauri/src/models.rs` | All data structures (Activity, Media, etc.) |
| `src/main.js` | UI logic, event handlers, state management |
| `src/notifications.js` | Toast notification system |

## What NOT to Do

### Do Not Add

- [ ] Support for other educational platforms
- [ ] Multi-language support / i18n
- [ ] User accounts or cloud sync
- [ ] Advanced configuration options
- [ ] Plugin or extension system
- [ ] Theming beyond basic Pico CSS
- [ ] Analytics or telemetry
- [ ] Social features

### Do Not Over-Engineer

- [ ] Abstract factories or dependency injection
- [ ] Event bus patterns
- [ ] State management libraries
- [ ] Complex caching strategies
- [ ] Microservice-style architecture

### Avoid Technical Jargon in UI

```
// BAD
"Erreur HTTP 403: Forbidden"
"Token JWT expiré"
"Timeout de connexion API"

// GOOD
"Connexion refusée. Veuillez vous reconnecter."
"Votre session a expiré."
"La connexion prend trop de temps. Vérifiez votre internet."
```

## Sync Folder Structure

When files are downloaded, they're organized as:
```
{sync_folder}/
└── {YYYY-MM-DD}_{activity_title}/
    ├── article.md          # Activity text content
    ├── photo_001.jpg       # Media files
    ├── photo_002.jpg
    └── document.pdf
```

- Dates use ISO format for natural sorting
- Activity titles are sanitized (no `/`, `\`, special chars)
- Duplicate files are detected by size comparison

## Security Notes

- OAuth tokens stored in **system keyring** (not files)
- Large tokens are chunked (keyring has size limits)
- Updates are **signed** with Ed25519 keys
- No sensitive data in logs (tokens are redacted)

## CI/CD Workflow

| Workflow | Trigger | Action |
|----------|---------|--------|
| `ci.yml` | Push/PR | Clippy lint, rustfmt check |
| `unit_tests.yml` | Push/PR | Rust tests |
| `frontend_tests.yml` | Push/PR with label | Vitest |
| `release.yml` | Tag `v*` | Build & release all platforms |

## Quick Reference

### Common Tasks

**Fix a bug in sync logic**: Edit `src-tauri/src/sync.rs`, add test, run `cargo test`

**Update UI text**: Edit `src/index.html` (static text) or `src/main.js` (dynamic text)

**Add new error message**: Update Rust command to return French string, handle in JS with `showError()`

**Modify download behavior**: Edit `start_sync()` in `sync.rs`, test with `cargo test`

### Key Files to Read First

1. `src-tauri/src/lib.rs` - Entry point, see all registered commands
2. `src/main.js` - Frontend logic, UI flow
3. `src-tauri/src/models.rs` - All data structures
4. `src-tauri/tauri.conf.json` - App configuration

### Log Locations

- **Linux**: `~/.config/educartable-sync/logs/`
- **macOS**: `~/Library/Logs/educartable-sync/`
- **Windows**: `%APPDATA%\educartable-sync\logs\`
