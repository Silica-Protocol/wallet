import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

export interface WalletSession {
  address: string;
  publicKey: string;
  walletName: string;
  algorithm: string;
  unlockedAt: number;
  expiresAt: number;
}

export interface StoredWalletData {
  // Encrypted wallet data (never store raw keys in storage)
  encryptedMnemonic: string;
  encryptedPrivateKey: string;
  // Non-sensitive metadata
  address: string;
  publicKey: string;
  walletName: string;
  algorithm: string;
  supportsPostQuantum: boolean;
  createdAt: string;
  lastUpdated: string;
  // Password verification (hash, not the password)
  passwordHash: string;
  // Attempt tracking
  failedAttempts: number;
  lockedUntil: number | null;
}

const WALLET_STORAGE_KEY = 'chert_wallet_data';
const SESSION_STORAGE_KEY = 'chert_wallet_session';
const SESSION_DURATION_MS = 30 * 60 * 1000; // 30 minutes

@Injectable({ providedIn: 'root' })
export class SessionStorageService {
  private sessionSubject = new BehaviorSubject<WalletSession | null>(null);
  public session$: Observable<WalletSession | null> = this.sessionSubject.asObservable();

  private sessionCheckInterval: ReturnType<typeof setInterval> | null = null;

  constructor() {
    this.restoreSession();
    this.startSessionMonitor();
  }

  /**
   * Store wallet data in localStorage (encrypted sensitive data)
   */
  saveWalletData(data: StoredWalletData): void {
    try {
      localStorage.setItem(WALLET_STORAGE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('Failed to save wallet data:', error);
      throw new Error('Failed to save wallet data to storage');
    }
  }

  /**
   * Load wallet data from localStorage
   */
  loadWalletData(): StoredWalletData | null {
    try {
      const data = localStorage.getItem(WALLET_STORAGE_KEY);
      if (!data) return null;
      return JSON.parse(data) as StoredWalletData;
    } catch (error) {
      console.error('Failed to load wallet data:', error);
      return null;
    }
  }

  /**
   * Check if wallet exists in storage
   */
  walletExists(): boolean {
    return localStorage.getItem(WALLET_STORAGE_KEY) !== null;
  }

  /**
   * Clear wallet data from storage
   */
  clearWalletData(): void {
    localStorage.removeItem(WALLET_STORAGE_KEY);
    this.clearSession();
  }

  /**
   * Create a new session after successful unlock
   */
  createSession(walletData: StoredWalletData): WalletSession {
    const now = Date.now();
    const session: WalletSession = {
      address: walletData.address,
      publicKey: walletData.publicKey,
      walletName: walletData.walletName,
      algorithm: walletData.algorithm,
      unlockedAt: now,
      expiresAt: now + SESSION_DURATION_MS
    };

    try {
      sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
      this.sessionSubject.next(session);
      return session;
    } catch (error) {
      console.error('Failed to create session:', error);
      throw new Error('Failed to create wallet session');
    }
  }

  /**
   * Extend the current session
   */
  extendSession(): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    session.expiresAt = Date.now() + SESSION_DURATION_MS;

    try {
      sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
      this.sessionSubject.next({ ...session });
    } catch (error) {
      console.error('Failed to extend session:', error);
    }
  }

  /**
   * Clear the current session (lock wallet)
   */
  clearSession(): void {
    sessionStorage.removeItem(SESSION_STORAGE_KEY);
    this.sessionSubject.next(null);
  }

  /**
   * Check if session is valid
   */
  isSessionValid(): boolean {
    const session = this.sessionSubject.value;
    if (!session) return false;
    return Date.now() < session.expiresAt;
  }

  /**
   * Get current session
   */
  getSession(): WalletSession | null {
    return this.sessionSubject.value;
  }

  /**
   * Restore session from sessionStorage on page load
   */
  private restoreSession(): void {
    try {
      const sessionData = sessionStorage.getItem(SESSION_STORAGE_KEY);
      if (!sessionData) return;

      const session = JSON.parse(sessionData) as WalletSession;

      // Check if session is still valid
      if (Date.now() < session.expiresAt) {
        this.sessionSubject.next(session);
      } else {
        // Session expired, clear it
        this.clearSession();
      }
    } catch (error) {
      console.error('Failed to restore session:', error);
      this.clearSession();
    }
  }

  /**
   * Start monitoring session expiry
   */
  private startSessionMonitor(): void {
    // Check session every minute
    this.sessionCheckInterval = setInterval(() => {
      if (!this.isSessionValid() && this.sessionSubject.value) {
        console.log('Session expired, locking wallet');
        this.clearSession();
      }
    }, 60000);
  }

  /**
   * Record failed login attempt
   */
  recordFailedAttempt(): number {
    const walletData = this.loadWalletData();
    if (!walletData) return 0;

    walletData.failedAttempts = (walletData.failedAttempts || 0) + 1;
    walletData.lastUpdated = new Date().toISOString();

    // Lock for increasing duration after multiple failures
    if (walletData.failedAttempts >= 5) {
      const lockDuration = Math.min(
        Math.pow(2, walletData.failedAttempts - 5) * 60000, // Exponential backoff starting at 1 min
        3600000 // Max 1 hour
      );
      walletData.lockedUntil = Date.now() + lockDuration;
    }

    this.saveWalletData(walletData);
    return walletData.failedAttempts;
  }

  /**
   * Reset failed attempt counter
   */
  resetFailedAttempts(): void {
    const walletData = this.loadWalletData();
    if (!walletData) return;

    walletData.failedAttempts = 0;
    walletData.lockedUntil = null;
    walletData.lastUpdated = new Date().toISOString();
    this.saveWalletData(walletData);
  }

  /**
   * Check if wallet is locked due to failed attempts
   */
  isLockedOut(): { locked: boolean; remainingMs: number } {
    const walletData = this.loadWalletData();
    if (!walletData?.lockedUntil) {
      return { locked: false, remainingMs: 0 };
    }

    const remainingMs = walletData.lockedUntil - Date.now();
    if (remainingMs <= 0) {
      // Lock expired, reset
      walletData.lockedUntil = null;
      this.saveWalletData(walletData);
      return { locked: false, remainingMs: 0 };
    }

    return { locked: true, remainingMs };
  }

  ngOnDestroy(): void {
    if (this.sessionCheckInterval) {
      clearInterval(this.sessionCheckInterval);
    }
  }
}
