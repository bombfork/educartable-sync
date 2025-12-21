/**
 * Educartable Sync - Notification System
 *
 * Provides user-friendly error messages, success notifications, and loading states.
 * Implements toast notifications for better user experience.
 */

/**
 * Show an error notification with user-friendly message and actionable advice
 * @param {string} title - Short error title
 * @param {string} message - Detailed error message
 * @param {string} [action] - Optional actionable advice for the user
 */
export function showError(title, message, action = '') {
    console.error(`Error: ${title} - ${message}`);

    const notification = createNotification('error', title, message, action);
    showNotification(notification);
}

/**
 * Show a success notification
 * @param {string} message - Success message to display
 */
export function showSuccess(message) {
    console.log(`Success: ${message}`);

    const notification = createNotification('success', 'Succès', message);
    showNotification(notification);
}

/**
 * Show a loading notification (returns handle for dismissal)
 * @param {string} message - Loading message to display
 * @returns {object} Handle with dismiss() method
 */
export function showLoading(message) {
    console.log(`Loading: ${message}`);

    const notification = createNotification('loading', 'Veuillez patienter', message);
    showNotification(notification);

    // Return handle for manual dismissal
    return {
        dismiss: () => dismissNotification(notification)
    };
}

/**
 * Show an info notification
 * @param {string} title - Info title
 * @param {string} message - Info message
 */
export function showInfo(title, message) {
    console.log(`Info: ${title} - ${message}`);

    const notification = createNotification('info', title, message);
    showNotification(notification);
}

/**
 * Create a notification element
 * @private
 */
function createNotification(type, title, message, action = '') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.setAttribute('role', 'alert');

    // Choose icon based on type
    let icon;
    switch (type) {
        case 'error':
            icon = '❌';
            break;
        case 'success':
            icon = '✓';
            break;
        case 'loading':
            icon = '⏳';
            break;
        case 'info':
            icon = 'ℹ️';
            break;
        default:
            icon = '📢';
    }

    notification.innerHTML = `
        <div class="notification-icon">${icon}</div>
        <div class="notification-content">
            <div class="notification-title">${escapeHtml(title)}</div>
            <div class="notification-message">${escapeHtml(message)}</div>
            ${action ? `<div class="notification-action">${escapeHtml(action)}</div>` : ''}
        </div>
        <button class="notification-close" aria-label="Close">&times;</button>
    `;

    // Add close button handler (if present)
    const closeBtn = notification.querySelector('.notification-close');
    if (closeBtn) {
        closeBtn.addEventListener('click', () => {
            dismissNotification(notification);
        });
    }

    return notification;
}

/**
 * Show a notification element
 * @private
 */
function showNotification(notification) {
    const container = getOrCreateContainer();
    container.appendChild(notification);

    // Trigger animation
    setTimeout(() => {
        notification.classList.add('notification-show');
    }, 10);
}

/**
 * Dismiss a notification element
 * @private
 */
function dismissNotification(notification) {
    notification.classList.remove('notification-show');
    notification.classList.add('notification-hide');

    // Remove from DOM after animation
    setTimeout(() => {
        if (notification.parentNode) {
            notification.parentNode.removeChild(notification);
        }
    }, 300);
}

/**
 * Get or create the notification container
 * @private
 */
function getOrCreateContainer() {
    let container = document.getElementById('notification-container');

    if (!container) {
        container = document.createElement('div');
        container.id = 'notification-container';
        container.className = 'notification-container';
        document.body.appendChild(container);
    }

    return container;
}

/**
 * Escape HTML to prevent XSS
 * @private
 */
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Parse backend error messages and make them user-friendly
 * @param {string|Error} error - Error from backend
 * @returns {object} Object with title, message, and action
 */
export function parseError(error) {
    const errorStr = typeof error === 'string' ? error : error.toString();
    const errorLower = errorStr.toLowerCase();

    // Authentication errors
    if (errorLower.includes('not authenticated') || errorLower.includes('failed to load tokens')) {
        return {
            title: 'Non connecté',
            message: 'Vous devez d\'abord vous connecter à Educartable.',
            action: 'Cliquez sur le bouton "Se connecter" pour continuer.'
        };
    }

    if (errorLower.includes('login timeout') || errorLower.includes('no response within')) {
        return {
            title: 'Délai de connexion dépassé',
            message: 'La connexion a pris trop de temps.',
            action: 'Veuillez réessayer de vous connecter.'
        };
    }

    if (errorLower.includes('tokens not found') || errorLower.includes('invalid token')) {
        return {
            title: 'Échec de connexion',
            message: 'Impossible de terminer la connexion.',
            action: 'Veuillez vérifier vos identifiants et réessayer.'
        };
    }

    if (errorLower.includes('token refresh failed')) {
        return {
            title: 'Session expirée',
            message: 'Votre session de connexion a expiré.',
            action: 'Veuillez vous reconnecter pour continuer.'
        };
    }

    // Network errors
    if (errorLower.includes('network') || errorLower.includes('connection') ||
        errorLower.includes('timed out') || errorLower.includes('timeout')) {
        return {
            title: 'Erreur de connexion',
            message: 'Impossible de se connecter à Educartable.',
            action: 'Veuillez vérifier votre connexion Internet et réessayer.'
        };
    }

    if (errorLower.includes('rate limit')) {
        return {
            title: 'Trop de requêtes',
            message: 'Vous envoyez des requêtes trop rapidement.',
            action: 'Veuillez patienter un instant et réessayer.'
        };
    }

    // Configuration errors
    if (errorLower.includes('sync directory not configured') || errorLower.includes('dossier de synchronisation non configuré') ||
        errorLower.includes('no folder selected') || errorLower.includes('aucun dossier')) {
        return {
            title: 'Aucun dossier sélectionné',
            message: 'Vous devez choisir où enregistrer vos photos.',
            action: 'Cliquez sur le bouton "Parcourir" pour sélectionner un dossier.'
        };
    }

    if (errorLower.includes('permission denied') || errorLower.includes('access denied')) {
        return {
            title: 'Permission refusée',
            message: 'Impossible d\'écrire dans le dossier sélectionné.',
            action: 'Choisissez un dossier différent pour lequel vous avez les permissions d\'écriture.'
        };
    }

    if (errorLower.includes('no space') || errorLower.includes('disk full') ||
        errorLower.includes('insufficient space')) {
        return {
            title: 'Espace insuffisant',
            message: 'Votre disque est plein ou manque d\'espace.',
            action: 'Libérez de l\'espace et réessayez.'
        };
    }

    // Sync errors
    if (errorLower.includes('failed to get user info')) {
        return {
            title: 'Impossible d\'accéder au compte',
            message: 'Impossible de récupérer les informations de votre compte.',
            action: 'Veuillez réessayer de vous connecter.'
        };
    }

    if (errorLower.includes('failed to fetch activities')) {
        return {
            title: 'Impossible de charger les activités',
            message: 'Impossible de récupérer vos activités depuis Educartable.',
            action: 'Veuillez vérifier votre connexion et réessayer.'
        };
    }

    if (errorLower.includes('server error') || errorLower.includes('500') ||
        errorLower.includes('503') || errorLower.includes('502')) {
        return {
            title: 'Erreur serveur',
            message: 'Les serveurs Educartable rencontrent des problèmes.',
            action: 'Veuillez réessayer plus tard.'
        };
    }

    // Generic errors
    if (errorLower.includes('failed to') || errorLower.includes('error:')) {
        // Try to extract a more specific message
        const colonIndex = errorStr.indexOf(':');
        const specificMsg = colonIndex > 0 ? errorStr.substring(colonIndex + 1).trim() : errorStr;

        return {
            title: 'Échec de l\'opération',
            message: specificMsg,
            action: 'Veuillez réessayer ou contacter le support si le problème persiste.'
        };
    }

    // Unknown error
    return {
        title: 'Erreur inattendue',
        message: errorStr,
        action: 'Veuillez réessayer. Si le problème persiste, contactez le support.'
    };
}

/**
 * Handle errors in a user-friendly way
 * @param {string|Error} error - Error from backend
 * @param {string} [context] - Optional context (e.g., "login", "sync")
 */
export function handleError(error, context = '') {
    const parsed = parseError(error);

    // Log full error for debugging
    console.error(`Error in ${context}:`, error);

    // Show user-friendly message
    showError(parsed.title, parsed.message, parsed.action);
}
