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

// Import updater module
import { checkForUpdatesSilently, handleUpdateButtonClick } from './updater.js';

// Application state
let isAuthenticated = false;
let config = null;
let activitiesData = null; // Cached activities and sync state
let selectedActivityIds = new Set(); // Currently selected activity IDs

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

    // Check for updates silently (non-blocking)
    checkForUpdatesSilently();

    console.log('Initialization complete');
}

/**
 * Set up event listeners for UI interactions
 */
function setupEventListeners() {
    // Authentication buttons
    document.getElementById('login-btn').addEventListener('click', handleLogin);
    document.getElementById('logout-btn').addEventListener('click', handleLogout);

    // Configuration buttons and inputs
    document.getElementById('browse-btn').addEventListener('click', handleBrowse);
    document.getElementById('sync-path').addEventListener('click', handleBrowse);
    document.getElementById('open-logs-btn').addEventListener('click', handleOpenLogs);
    document.getElementById('check-updates-btn').addEventListener('click', handleUpdateButtonClick);

    // Sync button - now opens activity selection dialog
    document.getElementById('sync-btn').addEventListener('click', handleSyncButtonClick);

    // Activity selection dialog buttons
    document.getElementById('close-dialog-btn').addEventListener('click', closeActivityDialog);
    document.getElementById('cancel-selection-btn').addEventListener('click', closeActivityDialog);
    document.getElementById('confirm-sync-btn').addEventListener('click', handleConfirmSync);
    document.getElementById('select-all-btn').addEventListener('click', handleSelectAll);
    document.getElementById('deselect-all-btn').addEventListener('click', handleDeselectAll);

    // Close dialog when clicking outside
    const dialog = document.getElementById('activity-selection-dialog');
    dialog.addEventListener('click', (e) => {
        if (e.target === dialog) {
            closeActivityDialog();
        }
    });
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
    loginBtn.textContent = 'Connexion...';

    // Show loading notification
    const loadingNotification = showLoading('Ouverture de la fenêtre de connexion...');

    try {
        // Call the Tauri authenticate command (opens webview)
        await invoke('authenticate');

        // Dismiss loading notification
        loadingNotification.dismiss();

        console.log('Login successful');
        isAuthenticated = true;
        updateAuthUI();

        // Show success notification
        showSuccess('Connecté à Educartable avec succès !');
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
    logoutBtn.textContent = 'Déconnexion...';

    try {
        // Call the Tauri logout command
        await invoke('logout');

        console.log('Logout successful');
        isAuthenticated = false;
        updateAuthUI();

        // Show success notification
        showSuccess('Déconnecté avec succès');
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
        authStatus.textContent = '✓ Connecté';
        authStatus.className = 'status-connected';

        // Show logout button, hide login button
        loginBtn.classList.add('hidden');
        logoutBtn.classList.remove('hidden');

        // Enable sync functionality
        syncBtn.disabled = false;
    } else {
        // Disconnected state
        authStatus.textContent = 'Non connecté';
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
            sync_path: ''
        };
        displayConfig();
    }
}

/**
 * Display configuration values in the UI
 */
function displayConfig() {
    const syncPathInput = document.getElementById('sync-path');

    // Display sync path
    if (config.sync_path) {
        syncPathInput.value = config.sync_path;
        syncPathInput.placeholder = '';
    } else {
        syncPathInput.value = '';
        syncPathInput.placeholder = 'Sélectionnez un dossier de destination';
    }
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
            showSuccess('Dossier de synchronisation sélectionné avec succès');
        }
    } catch (error) {
        console.error('Failed to select directory:', error);

        // Only show error if it's not a cancellation
        if (error !== 'No folder selected' && error !== 'Aucun dossier sélectionné') {
            handleError(error, 'select directory');
        }
    } finally {
        // Reset button state
        browseBtn.disabled = false;
        browseBtn.removeAttribute('aria-busy');
    }
}

/**
 * Handle open logs button click
 * Opens the logs directory in the system file explorer
 */
async function handleOpenLogs() {
    console.log('Opening logs directory...');

    const openLogsBtn = document.getElementById('open-logs-btn');

    // Set button to loading state
    openLogsBtn.disabled = true;
    openLogsBtn.setAttribute('aria-busy', 'true');
    const originalText = openLogsBtn.textContent;
    openLogsBtn.textContent = 'Ouverture...';

    try {
        // Call the Tauri open_logs_directory command
        await invoke('open_logs_directory');
        console.log('Logs directory opened successfully');

        // Show success notification
        showSuccess('Dossier des logs ouvert avec succès');
    } catch (error) {
        console.error('Failed to open logs directory:', error);
        handleError(error, 'open logs directory');
    } finally {
        // Reset button state
        openLogsBtn.disabled = false;
        openLogsBtn.removeAttribute('aria-busy');
        openLogsBtn.textContent = originalText;
    }
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
    const progressFill = document.getElementById('sync-progress-fill');
    const progressText = document.getElementById('progress-text');
    const currentFile = document.getElementById('current-file');

    // Update custom progress bar fill width
    progressFill.style.width = `${progress.percentage}%`;

    // Update text
    progressText.textContent = `${progress.current} / ${progress.total} fichiers (${Math.round(progress.percentage)}%)`;
    currentFile.textContent = progress.current_file ? `📄 ${progress.current_file}` : '';
}

/**
 * Handle sync button click
 * Opens activity selection dialog instead of syncing directly
 */
async function handleSyncButtonClick() {
    console.log('Sync button clicked, opening activity selection...');

    const syncBtn = document.getElementById('sync-btn');

    // Validate configuration
    if (!config.sync_path) {
        const parsed = parseError('Dossier de synchronisation non configure');
        showError(parsed.title, parsed.message, parsed.action);
        return;
    }

    // Disable button and show loading state
    syncBtn.disabled = true;
    syncBtn.setAttribute('aria-busy', 'true');
    const originalText = syncBtn.textContent;
    syncBtn.textContent = 'Chargement...';

    try {
        // Fetch activities from backend
        console.log('Fetching activities...');
        activitiesData = await invoke('fetch_activities');
        console.log('Fetched activities:', activitiesData);

        // Open the activity selection dialog
        openActivityDialog();
    } catch (error) {
        console.error('Failed to fetch activities:', error);
        handleError(error, 'fetch activities');
    } finally {
        // Re-enable button
        syncBtn.disabled = !isAuthenticated;
        syncBtn.removeAttribute('aria-busy');
        syncBtn.textContent = originalText;
    }
}

/**
 * Open the activity selection dialog and populate with activities
 */
function openActivityDialog() {
    const dialog = document.getElementById('activity-selection-dialog');
    const container = document.getElementById('activity-list-container');
    const loadingText = document.getElementById('loading-activities');

    // Clear previous content
    container.innerHTML = '';

    if (!activitiesData || activitiesData.activities.length === 0) {
        container.innerHTML = '<p class="no-activities">Aucune activite trouvee.</p>';
        dialog.showModal();
        return;
    }

    // Determine pre-selection based on sync state
    const previouslySyncedIds = new Set(activitiesData.previously_synced_ids || []);
    const allActivityIds = new Set(activitiesData.activities.map(a => a.id));

    // First sync (nothing synced yet): select all
    // Subsequent syncs: select previously synced + new activities
    if (previouslySyncedIds.size === 0) {
        // First sync: select all
        selectedActivityIds = new Set(allActivityIds);
    } else {
        // Subsequent syncs: pre-select previously synced + new activities
        selectedActivityIds = new Set();
        for (const activity of activitiesData.activities) {
            // Select if previously synced OR if it's new (not in previously synced)
            if (previouslySyncedIds.has(activity.id)) {
                selectedActivityIds.add(activity.id);
            } else {
                // This is a new activity, also select it by default
                selectedActivityIds.add(activity.id);
            }
        }
    }

    // Render activity list
    renderActivityList();

    // Update selection count
    updateSelectionCount();

    // Show dialog
    dialog.showModal();
}

/**
 * Render the activity list with checkboxes
 */
function renderActivityList() {
    const container = document.getElementById('activity-list-container');
    container.innerHTML = '';

    // Sort activities by date (newest first)
    const sortedActivities = [...activitiesData.activities].sort((a, b) => {
        return new Date(b.date) - new Date(a.date);
    });

    for (const activity of sortedActivities) {
        const isSelected = selectedActivityIds.has(activity.id);
        const isPreviouslySynced = activitiesData.previously_synced_ids?.includes(activity.id);

        // Format date
        const date = activity.date.split('T')[0] || 'Date inconnue';

        // Count media
        const mediaCount = activity.medias?.length || 0;
        const mediaText = mediaCount === 0 ? 'Pas de medias' :
                         mediaCount === 1 ? '1 media' : `${mediaCount} medias`;

        const item = document.createElement('label');
        item.className = `activity-item${isSelected ? ' selected' : ''}${isPreviouslySynced ? ' previously-synced' : ''}`;
        item.innerHTML = `
            <input type="checkbox"
                   value="${activity.id}"
                   ${isSelected ? 'checked' : ''}
                   class="activity-checkbox">
            <div class="activity-info">
                <span class="activity-title">${escapeHtml(activity.title)}</span>
                <span class="activity-meta">
                    <span class="activity-date">${date}</span>
                    <span class="activity-media-count">${mediaText}</span>
                    ${isPreviouslySynced ? '<span class="synced-badge">Deja synchronise</span>' : '<span class="new-badge">Nouveau</span>'}
                </span>
            </div>
        `;

        // Add event listener for checkbox change
        const checkbox = item.querySelector('input');
        checkbox.addEventListener('change', (e) => {
            if (e.target.checked) {
                selectedActivityIds.add(activity.id);
                item.classList.add('selected');
            } else {
                selectedActivityIds.delete(activity.id);
                item.classList.remove('selected');
            }
            updateSelectionCount();
        });

        container.appendChild(item);
    }
}

/**
 * Update the selection count display
 */
function updateSelectionCount() {
    const count = selectedActivityIds.size;
    const countElement = document.getElementById('selection-count');
    countElement.textContent = `${count} activite(s) selectionnee(s)`;

    // Disable confirm button if nothing selected
    const confirmBtn = document.getElementById('confirm-sync-btn');
    confirmBtn.disabled = count === 0;
}

/**
 * Handle select all button click
 */
function handleSelectAll() {
    if (!activitiesData) return;

    for (const activity of activitiesData.activities) {
        selectedActivityIds.add(activity.id);
    }

    // Update checkboxes
    document.querySelectorAll('.activity-checkbox').forEach(cb => {
        cb.checked = true;
        cb.closest('.activity-item')?.classList.add('selected');
    });

    updateSelectionCount();
}

/**
 * Handle deselect all button click
 */
function handleDeselectAll() {
    selectedActivityIds.clear();

    // Update checkboxes
    document.querySelectorAll('.activity-checkbox').forEach(cb => {
        cb.checked = false;
        cb.closest('.activity-item')?.classList.remove('selected');
    });

    updateSelectionCount();
}

/**
 * Close the activity selection dialog
 */
function closeActivityDialog() {
    const dialog = document.getElementById('activity-selection-dialog');
    dialog.close();
}

/**
 * Handle confirm sync button click
 * Starts sync with selected activities
 */
async function handleConfirmSync() {
    console.log('Confirm sync clicked, selected activities:', [...selectedActivityIds]);

    // Close the dialog
    closeActivityDialog();

    // Get selected activity IDs as array
    const selectedIds = [...selectedActivityIds];

    if (selectedIds.length === 0) {
        showInfo('Aucune activite selectionnee', 'Veuillez selectionner au moins une activite a synchroniser.');
        return;
    }

    // Perform the sync with selected activities
    await performSync(selectedIds);
}

/**
 * Perform synchronization with the backend
 * @param {string[]} selectedActivityIds - Array of activity IDs to sync
 */
async function performSync(selectedActivityIds) {
    console.log('Sync initiated with', selectedActivityIds.length, 'activities...');

    const syncBtn = document.getElementById('sync-btn');

    // Disable button and show loading state using Pico CSS aria-busy attribute
    syncBtn.disabled = true;
    syncBtn.setAttribute('aria-busy', 'true');
    const originalText = syncBtn.textContent;
    syncBtn.textContent = 'Synchronisation...';

    // Reset progress
    document.getElementById('sync-progress-fill').style.width = '0%';
    document.getElementById('progress-text').textContent = 'Demarrage...';
    document.getElementById('current-file').textContent = '';

    try {
        // Start sync with current configuration and selected activity IDs
        const stats = await invoke('start_sync', {
            config,
            selectedActivityIds: selectedActivityIds
        });

        console.log('Sync completed successfully:', stats);

        // Show success notification with stats
        const successMsg = `${stats.downloaded} fichiers telecharges, ${stats.skipped} ignores${stats.failed > 0 ? `, ${stats.failed} en echec` : ', aucun echec'}`;
        showSuccess(successMsg);
    } catch (error) {
        console.error('Sync failed:', error);

        // Show user-friendly error
        handleError(error, 'sync');
    } finally {
        // Re-enable button based on auth status
        syncBtn.disabled = !isAuthenticated;
        syncBtn.removeAttribute('aria-busy');
        syncBtn.textContent = originalText;
    }
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', init);
