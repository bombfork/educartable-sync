/**
 * Educartable Sync - Main JavaScript
 *
 * This module handles the frontend logic for authentication, configuration,
 * and synchronization with the Educartable platform.
 */

// Import Tauri API
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Import notification system
import { showError, showSuccess, showLoading, showInfo, handleError, parseError } from './notifications.js';

// Application state
let isAuthenticated = false;
let config = null;

/**
 * Initialize the application when DOM is ready
 */
async function init() {
    console.log('Initializing Educartable Sync...');

    // Set up progress listener for sync events
    await setupProgressListener();

    // Load configuration
    await loadConfig();

    // Check authentication status on load
    await checkAuthStatus();

    // Set up event listeners
    setupEventListeners();

    console.log('Initialization complete');
}

/**
 * Set up event listeners for UI interactions
 */
function setupEventListeners() {
    // Authentication buttons
    document.getElementById('login-btn').addEventListener('click', handleLogin);
    document.getElementById('logout-btn').addEventListener('click', handleLogout);

    // Configuration buttons
    document.getElementById('browse-btn').addEventListener('click', handleBrowse);
    document.getElementById('include-videos').addEventListener('change', handleIncludeVideosChange);
    document.getElementById('organize-by-date').addEventListener('change', handleOrganizeByDateChange);

    // Sync button
    document.getElementById('sync-btn').addEventListener('click', handleSync);
}

/**
 * Check authentication status by calling the Tauri backend
 */
async function checkAuthStatus() {
    console.log('Checking authentication status...');

    try {
        isAuthenticated = await invoke('is_authenticated');
        console.log('Authentication status:', isAuthenticated);
        updateAuthUI();
    } catch (error) {
        console.error('Auth check failed:', error);
        isAuthenticated = false;
        updateAuthUI();
    }
}

/**
 * Handle login button click
 * Opens webview for OAuth authentication via Keycloak
 */
async function handleLogin() {
    console.log('Login initiated...');

    const loginBtn = document.getElementById('login-btn');

    // Set button to loading state using Pico CSS aria-busy attribute
    loginBtn.disabled = true;
    loginBtn.setAttribute('aria-busy', 'true');
    const originalText = loginBtn.textContent;
    loginBtn.textContent = 'Logging in...';

    // Show loading notification
    const loadingNotification = showLoading('Opening login window...');

    try {
        // Call the Tauri authenticate command (opens webview)
        await invoke('authenticate');

        // Dismiss loading notification
        loadingNotification.dismiss();

        console.log('Login successful');
        isAuthenticated = true;
        updateAuthUI();

        // Show success notification
        showSuccess('Successfully logged in to Educartable!');
    } catch (error) {
        console.error('Login failed:', error);

        // Dismiss loading notification
        loadingNotification.dismiss();

        // Show user-friendly error
        handleError(error, 'login');

        isAuthenticated = false;
        updateAuthUI();
    } finally {
        // Reset button state
        loginBtn.disabled = false;
        loginBtn.removeAttribute('aria-busy');
        loginBtn.textContent = originalText;
    }
}

/**
 * Handle logout button click
 * Clears authentication tokens from storage
 */
async function handleLogout() {
    console.log('Logout initiated...');

    const logoutBtn = document.getElementById('logout-btn');

    // Set button to loading state
    logoutBtn.disabled = true;
    logoutBtn.setAttribute('aria-busy', 'true');
    const originalText = logoutBtn.textContent;
    logoutBtn.textContent = 'Logging out...';

    try {
        // Call the Tauri logout command
        await invoke('logout');

        console.log('Logout successful');
        isAuthenticated = false;
        updateAuthUI();

        // Show success notification
        showSuccess('Successfully logged out');
    } catch (error) {
        console.error('Logout failed:', error);

        // Show user-friendly error
        handleError(error, 'logout');
    } finally {
        // Reset button state
        logoutBtn.disabled = false;
        logoutBtn.removeAttribute('aria-busy');
        logoutBtn.textContent = originalText;
    }
}

/**
 * Update UI elements based on authentication state
 */
function updateAuthUI() {
    const authStatus = document.getElementById('auth-status');
    const loginBtn = document.getElementById('login-btn');
    const logoutBtn = document.getElementById('logout-btn');
    const syncBtn = document.getElementById('sync-btn');

    if (isAuthenticated) {
        // Connected state
        authStatus.textContent = '✓ Connected';
        authStatus.className = 'status-connected';

        // Show logout button, hide login button
        loginBtn.classList.add('hidden');
        logoutBtn.classList.remove('hidden');

        // Enable sync functionality
        syncBtn.disabled = false;
    } else {
        // Disconnected state
        authStatus.textContent = 'Not connected';
        authStatus.className = 'status-disconnected';

        // Show login button, hide logout button
        loginBtn.classList.remove('hidden');
        logoutBtn.classList.add('hidden');

        // Disable sync functionality
        syncBtn.disabled = true;
    }
}

/**
 * Load configuration from backend
 */
async function loadConfig() {
    console.log('Loading configuration...');

    try {
        config = await invoke('load_config');
        console.log('Config loaded:', config);
        displayConfig();
    } catch (error) {
        console.error('Failed to load config:', error);

        // Use default config
        config = {
            sync_path: '',
            include_videos: true,
            organize_by_date: true
        };
        displayConfig();
    }
}

/**
 * Display configuration values in the UI
 */
function displayConfig() {
    const syncPathInput = document.getElementById('sync-path');
    const includeVideosCheckbox = document.getElementById('include-videos');
    const organizeByDateCheckbox = document.getElementById('organize-by-date');

    // Display sync path
    if (config.sync_path) {
        syncPathInput.value = config.sync_path;
        syncPathInput.placeholder = '';
    } else {
        syncPathInput.value = '';
        syncPathInput.placeholder = 'No folder selected';
    }

    // Display checkbox values
    includeVideosCheckbox.checked = config.include_videos;
    organizeByDateCheckbox.checked = config.organize_by_date;
}

/**
 * Save configuration to backend
 */
async function saveConfig() {
    try {
        await invoke('save_config', { config });
        console.log('Config saved successfully');
    } catch (error) {
        console.error('Failed to save config:', error);
        handleError(error, 'save configuration');
    }
}

/**
 * Handle browse button click
 * Opens native directory picker dialog
 */
async function handleBrowse() {
    console.log('Opening directory picker...');

    const browseBtn = document.getElementById('browse-btn');

    // Set button to loading state using Pico CSS aria-busy attribute
    browseBtn.disabled = true;
    browseBtn.setAttribute('aria-busy', 'true');

    try {
        // Call the Tauri select_sync_directory command
        const path = await invoke('select_sync_directory');

        if (path) {
            console.log('Directory selected:', path);
            config.sync_path = path;

            // Update UI
            const syncPathInput = document.getElementById('sync-path');
            syncPathInput.value = path;
            syncPathInput.placeholder = '';

            // Save config
            await saveConfig();

            // Show success notification
            showSuccess('Sync folder selected successfully');
        }
    } catch (error) {
        console.error('Failed to select directory:', error);

        // Only show error if it's not a cancellation
        if (error !== 'No folder selected') {
            handleError(error, 'select directory');
        }
    } finally {
        // Reset button state
        browseBtn.disabled = false;
        browseBtn.removeAttribute('aria-busy');
    }
}

/**
 * Handle include videos checkbox change
 */
async function handleIncludeVideosChange(event) {
    console.log('Include videos changed:', event.target.checked);
    config.include_videos = event.target.checked;
    await saveConfig();
}

/**
 * Handle organize by date checkbox change
 */
async function handleOrganizeByDateChange(event) {
    console.log('Organize by date changed:', event.target.checked);
    config.organize_by_date = event.target.checked;
    await saveConfig();
}

/**
 * Set up listener for sync progress events from backend
 */
async function setupProgressListener() {
    await listen('sync-progress', (event) => {
        const progress = event.payload;
        updateProgress(progress);
    });
    console.log('Progress listener registered');
}

/**
 * Update progress UI with current sync status
 */
function updateProgress(progress) {
    const progressBar = document.getElementById('sync-progress');
    const progressText = document.getElementById('progress-text');
    const currentFile = document.getElementById('current-file');

    // Update HTML5 progress bar (Pico CSS styles it automatically)
    progressBar.value = progress.percentage;
    progressBar.max = 100;

    // Update text
    progressText.textContent = `${progress.current} / ${progress.total} files (${Math.round(progress.percentage)}%)`;
    currentFile.textContent = progress.current_file ? `📄 ${progress.current_file}` : '';
}

/**
 * Display sync results after completion
 */
function displaySyncResults(stats) {
    const syncResults = document.getElementById('sync-results');

    document.getElementById('result-total').textContent = stats.total_media;
    document.getElementById('result-downloaded').textContent = stats.downloaded;
    document.getElementById('result-skipped').textContent = stats.skipped;
    document.getElementById('result-failed').textContent = stats.failed;

    syncResults.classList.remove('hidden');
}

/**
 * Handle sync button click
 * Triggers synchronization with the backend
 */
async function handleSync() {
    console.log('Sync initiated...');

    const syncBtn = document.getElementById('sync-btn');
    const progressContainer = document.getElementById('sync-progress-container');
    const syncResults = document.getElementById('sync-results');

    // Validate configuration
    if (!config.sync_path) {
        const parsed = parseError('Sync directory not configured');
        showError(parsed.title, parsed.message, parsed.action);
        return;
    }

    // Disable button and show loading state using Pico CSS aria-busy attribute
    syncBtn.disabled = true;
    syncBtn.setAttribute('aria-busy', 'true');
    const originalText = syncBtn.textContent;
    syncBtn.textContent = 'Syncing...';

    // Show progress, hide results
    progressContainer.classList.remove('hidden');
    syncResults.classList.add('hidden');

    // Reset progress
    document.getElementById('sync-progress').value = 0;
    document.getElementById('progress-text').textContent = 'Starting...';
    document.getElementById('current-file').textContent = '';

    try {
        // Start sync with current configuration
        const stats = await invoke('start_sync', { config });

        console.log('Sync completed successfully:', stats);

        // Show results
        displaySyncResults(stats);

        // Show success notification with stats
        const successMsg = `Downloaded ${stats.downloaded} files, skipped ${stats.skipped}, ${stats.failed > 0 ? `${stats.failed} failed` : 'no failures'}`;
        showSuccess(successMsg);
    } catch (error) {
        console.error('Sync failed:', error);

        // Show user-friendly error
        handleError(error, 'sync');

        // Hide progress on error
        progressContainer.classList.add('hidden');
    } finally {
        // Re-enable button based on auth status
        syncBtn.disabled = !isAuthenticated;
        syncBtn.removeAttribute('aria-busy');
        syncBtn.textContent = originalText;
    }
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', init);
