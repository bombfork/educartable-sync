/**
 * Notifications Test Suite
 *
 * Tests for the notification system, focusing on:
 * - parseError() - Converting backend errors to user-friendly French messages
 * - escapeHtml() - XSS prevention
 *
 * Note: DOM-based functions (showError, showSuccess, etc.) require JSDOM
 * and are tested separately if needed.
 */

import { describe, it, expect } from 'vitest';
import { parseError } from '../notifications.js';

/**
 * Test Suite: parseError()
 *
 * Critical function that converts backend error messages into user-friendly
 * French messages with actionable advice. Tests cover all error categories.
 */
describe('Notifications - parseError', () => {
  /**
   * Authentication Errors
   */
  describe('Authentication errors', () => {
    it('should parse "not authenticated" error', () => {
      const result = parseError('not authenticated');
      expect(result.title).toBe('Non connecté');
      expect(result.message).toBe('Vous devez d\'abord vous connecter à Educartable.');
      expect(result.action).toContain('Se connecter');
    });

    it('should parse "failed to load tokens" error', () => {
      const result = parseError('failed to load tokens');
      expect(result.title).toBe('Non connecté');
      expect(result.message).toBe('Vous devez d\'abord vous connecter à Educartable.');
      expect(result.action).toContain('Se connecter');
    });

    it('should parse login timeout error', () => {
      const result = parseError('login timeout');
      expect(result.title).toBe('Délai de connexion dépassé');
      expect(result.message).toBe('La connexion a pris trop de temps.');
      expect(result.action).toContain('réessayer');
    });

    it('should parse "no response within" error', () => {
      const result = parseError('no response within 30 seconds');
      expect(result.title).toBe('Délai de connexion dépassé');
      expect(result.message).toBe('La connexion a pris trop de temps.');
      expect(result.action).toContain('réessayer');
    });

    it('should parse "tokens not found" error', () => {
      const result = parseError('tokens not found');
      expect(result.title).toBe('Échec de connexion');
      expect(result.message).toBe('Impossible de terminer la connexion.');
      expect(result.action).toContain('identifiants');
    });

    it('should parse "invalid token" error', () => {
      const result = parseError('invalid token');
      expect(result.title).toBe('Échec de connexion');
      expect(result.message).toBe('Impossible de terminer la connexion.');
      expect(result.action).toContain('identifiants');
    });

    it('should parse "token refresh failed" error', () => {
      const result = parseError('token refresh failed');
      expect(result.title).toBe('Session expirée');
      expect(result.message).toBe('Votre session de connexion a expiré.');
      expect(result.action).toContain('reconnecter');
    });

    it('should handle authentication errors in Error objects', () => {
      const error = new Error('not authenticated');
      const result = parseError(error);
      expect(result.title).toBe('Non connecté');
      expect(result.message).toBe('Vous devez d\'abord vous connecter à Educartable.');
    });

    it('should be case-insensitive for authentication errors', () => {
      const result = parseError('NOT AUTHENTICATED');
      expect(result.title).toBe('Non connecté');
    });
  });

  /**
   * Network Errors
   */
  describe('Network errors', () => {
    it('should parse "network" error', () => {
      const result = parseError('network error occurred');
      expect(result.title).toBe('Erreur de connexion');
      expect(result.message).toBe('Impossible de se connecter à Educartable.');
      expect(result.action).toContain('connexion Internet');
    });

    it('should parse "connection" error', () => {
      const result = parseError('connection failed');
      expect(result.title).toBe('Erreur de connexion');
      expect(result.message).toBe('Impossible de se connecter à Educartable.');
      expect(result.action).toContain('connexion Internet');
    });

    it('should parse "timed out" error', () => {
      const result = parseError('request timed out');
      expect(result.title).toBe('Erreur de connexion');
      expect(result.message).toBe('Impossible de se connecter à Educartable.');
      expect(result.action).toContain('connexion Internet');
    });

    it('should parse "timeout" error', () => {
      const result = parseError('connection timeout');
      expect(result.title).toBe('Erreur de connexion');
      expect(result.message).toBe('Impossible de se connecter à Educartable.');
      expect(result.action).toContain('connexion Internet');
    });

    it('should parse rate limit error', () => {
      const result = parseError('rate limit exceeded');
      expect(result.title).toBe('Trop de requêtes');
      expect(result.message).toBe('Vous envoyez des requêtes trop rapidement.');
      expect(result.action).toContain('patienter');
    });
  });

  /**
   * Configuration Errors
   */
  describe('Configuration errors', () => {
    it('should parse "sync directory not configured" error', () => {
      const result = parseError('sync directory not configured');
      expect(result.title).toBe('Aucun dossier sélectionné');
      expect(result.message).toBe('Vous devez choisir où enregistrer vos photos.');
      expect(result.action).toContain('Parcourir');
    });

    it('should parse French "dossier de synchronisation non configuré" error', () => {
      const result = parseError('dossier de synchronisation non configuré');
      expect(result.title).toBe('Aucun dossier sélectionné');
      expect(result.message).toBe('Vous devez choisir où enregistrer vos photos.');
      expect(result.action).toContain('Parcourir');
    });

    it('should parse "no folder selected" error', () => {
      const result = parseError('no folder selected');
      expect(result.title).toBe('Aucun dossier sélectionné');
      expect(result.message).toBe('Vous devez choisir où enregistrer vos photos.');
      expect(result.action).toContain('Parcourir');
    });

    it('should parse "aucun dossier" error', () => {
      const result = parseError('aucun dossier');
      expect(result.title).toBe('Aucun dossier sélectionné');
      expect(result.message).toBe('Vous devez choisir où enregistrer vos photos.');
      expect(result.action).toContain('Parcourir');
    });
  });

  /**
   * Permission Errors
   */
  describe('Permission errors', () => {
    it('should parse "permission denied" error', () => {
      const result = parseError('permission denied');
      expect(result.title).toBe('Permission refusée');
      expect(result.message).toBe('Impossible d\'écrire dans le dossier sélectionné.');
      expect(result.action).toContain('permissions d\'écriture');
    });

    it('should parse "access denied" error', () => {
      const result = parseError('access denied');
      expect(result.title).toBe('Permission refusée');
      expect(result.message).toBe('Impossible d\'écrire dans le dossier sélectionné.');
      expect(result.action).toContain('permissions d\'écriture');
    });
  });

  /**
   * Disk Space Errors
   */
  describe('Disk space errors', () => {
    it('should parse "no space" error', () => {
      const result = parseError('no space left on device');
      expect(result.title).toBe('Espace insuffisant');
      expect(result.message).toBe('Votre disque est plein ou manque d\'espace.');
      expect(result.action).toContain('Libérez de l\'espace');
    });

    it('should parse "disk full" error', () => {
      const result = parseError('disk full');
      expect(result.title).toBe('Espace insuffisant');
      expect(result.message).toBe('Votre disque est plein ou manque d\'espace.');
      expect(result.action).toContain('Libérez de l\'espace');
    });

    it('should parse "insufficient space" error', () => {
      const result = parseError('insufficient space');
      expect(result.title).toBe('Espace insuffisant');
      expect(result.message).toBe('Votre disque est plein ou manque d\'espace.');
      expect(result.action).toContain('Libérez de l\'espace');
    });
  });

  /**
   * Sync Errors
   */
  describe('Sync errors', () => {
    it('should parse "failed to get user info" error', () => {
      const result = parseError('failed to get user info');
      expect(result.title).toBe('Impossible d\'accéder au compte');
      expect(result.message).toBe('Impossible de récupérer les informations de votre compte.');
      expect(result.action).toContain('réessayer de vous connecter');
    });

    it('should parse "failed to fetch activities" error', () => {
      const result = parseError('failed to fetch activities');
      expect(result.title).toBe('Impossible de charger les activités');
      expect(result.message).toBe('Impossible de récupérer vos activités depuis Educartable.');
      expect(result.action).toContain('connexion');
    });
  });

  /**
   * Server Errors
   */
  describe('Server errors', () => {
    it('should parse "server error" error', () => {
      const result = parseError('server error');
      expect(result.title).toBe('Erreur serveur');
      expect(result.message).toBe('Les serveurs Educartable rencontrent des problèmes.');
      expect(result.action).toContain('réessayer plus tard');
    });

    it('should parse HTTP 500 error', () => {
      const result = parseError('HTTP 500 Internal Server Error');
      expect(result.title).toBe('Erreur serveur');
      expect(result.message).toBe('Les serveurs Educartable rencontrent des problèmes.');
      expect(result.action).toContain('réessayer plus tard');
    });

    it('should parse HTTP 502 error', () => {
      const result = parseError('502 Bad Gateway');
      expect(result.title).toBe('Erreur serveur');
      expect(result.message).toBe('Les serveurs Educartable rencontrent des problèmes.');
      expect(result.action).toContain('réessayer plus tard');
    });

    it('should parse HTTP 503 error', () => {
      const result = parseError('503 Service Unavailable');
      expect(result.title).toBe('Erreur serveur');
      expect(result.message).toBe('Les serveurs Educartable rencontrent des problèmes.');
      expect(result.action).toContain('réessayer plus tard');
    });
  });

  /**
   * Generic Errors
   */
  describe('Generic errors', () => {
    it('should parse "failed to" errors with specific message', () => {
      const result = parseError('Failed to: download image');
      expect(result.title).toBe('Échec de l\'opération');
      expect(result.message).toBe('download image');
      expect(result.action).toContain('réessayer');
    });

    it('should parse errors with colon separator', () => {
      const result = parseError('Error: something specific went wrong');
      expect(result.title).toBe('Échec de l\'opération');
      expect(result.message).toBe('something specific went wrong');
      expect(result.action).toContain('réessayer');
    });

    it('should handle unknown errors with fallback', () => {
      const result = parseError('some unknown error xyz');
      expect(result.title).toBe('Erreur inattendue');
      expect(result.message).toBe('some unknown error xyz');
      expect(result.action).toContain('réessayer');
    });

    it('should handle empty string error', () => {
      const result = parseError('');
      expect(result.title).toBe('Erreur inattendue');
      expect(result.message).toBe('');
      expect(result.action).toContain('réessayer');
    });

    it('should handle Error objects', () => {
      const error = new Error('Test error message');
      const result = parseError(error);
      // Error objects toString() to "Error: <message>", matching the "error:" pattern
      expect(result.title).toBe('Échec de l\'opération');
      expect(result.message).toContain('Test error message');
    });
  });

  /**
   * Priority Order Tests
   *
   * Ensures that more specific error patterns take precedence over generic ones.
   */
  describe('Error priority and specificity', () => {
    it('should prioritize authentication over generic "failed to" errors', () => {
      const result = parseError('Failed to: not authenticated');
      expect(result.title).toBe('Non connecté');
      expect(result.title).not.toBe('Échec de l\'opération');
    });

    it('should prioritize network errors over generic errors', () => {
      const result = parseError('Error: network connection failed');
      expect(result.title).toBe('Erreur de connexion');
      expect(result.title).not.toBe('Échec de l\'opération');
    });

    it('should prioritize specific sync errors over generic errors', () => {
      const result = parseError('Error: failed to get user info');
      expect(result.title).toBe('Impossible d\'accéder au compte');
      expect(result.title).not.toBe('Échec de l\'opération');
    });
  });

  /**
   * Return Structure Tests
   */
  describe('Return structure', () => {
    it('should always return an object with title, message, and action', () => {
      const result = parseError('any error');
      expect(result).toHaveProperty('title');
      expect(result).toHaveProperty('message');
      expect(result).toHaveProperty('action');
      expect(typeof result.title).toBe('string');
      expect(typeof result.message).toBe('string');
      expect(typeof result.action).toBe('string');
    });

    it('should return non-empty strings for all fields', () => {
      const result = parseError('test error');
      expect(result.title.length).toBeGreaterThan(0);
      expect(result.message.length).toBeGreaterThan(0);
      expect(result.action.length).toBeGreaterThan(0);
    });
  });
});

/**
 * Test Suite: escapeHtml()
 *
 * Critical security function that prevents XSS attacks by escaping HTML
 * special characters in user-facing messages.
 *
 * Note: escapeHtml is not currently exported from notifications.js
 * These tests are prepared for when it is exported.
 */
describe('Notifications - escapeHtml', () => {
  // Tests will be enabled once escapeHtml is exported from notifications.js
  it.todo('should escape < character');
  it.todo('should escape > character');
  it.todo('should escape & character');
  it.todo('should escape " character');
  it.todo('should escape \' character');
  it.todo('should handle complete XSS attempt');
  it.todo('should handle empty strings');
  it.todo('should handle strings with no special characters');
  it.todo('should handle mixed content');
});
