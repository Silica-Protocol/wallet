import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, OnChanges, OnDestroy, Output, SimpleChanges, computed, inject, signal } from '@angular/core';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';
import { WALLET_BACKEND, WalletBackend } from '../../services/wallet-backend.interface';
import { WalletInfoResponse } from '../../types/wallet.types';

const BASE_BACKOFF_MS = 3000;
const MAX_BACKOFF_MS = 300_000;

@Component({
  selector: 'app-unlock-wallet',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './unlock-wallet.component.html',
  styleUrl: './unlock-wallet.component.scss'
})
export class UnlockWalletComponent implements OnChanges, OnDestroy {
  private readonly walletBackend = inject<WalletBackend>(WALLET_BACKEND);
  private readonly formBuilder = inject(FormBuilder);

  private countdownTimer: number | null = null;
  private lockoutExpiry = 0;
  private failureCount = 0;

  readonly unlockForm = this.formBuilder.group({
    password: ['', [Validators.required, Validators.minLength(8)]]
  });

  readonly loading = signal(false);
  readonly lastError = signal<string | null>(null);
  readonly lockoutRemainingMs = signal(0);
  readonly isLockedOut = computed(() => this.lockoutRemainingMs() > 0);
  readonly lockoutSeconds = computed(() => {
    const remaining = this.lockoutRemainingMs();
    if (remaining <= 0) {
      return 0;
    }
    return Math.ceil(remaining / 1000);
  });

  @Input() walletInfo: WalletInfoResponse | null = null;

  @Output() walletInfoChange = new EventEmitter<WalletInfoResponse>();
  @Output() unlocked = new EventEmitter<void>();
  @Output() failed = new EventEmitter<number>();
  @Output() error = new EventEmitter<string>();

  get remainingAttempts(): number {
    return this.walletInfo?.remainingAttempts ?? 0;
  }

  get maxAttempts(): number {
    return this.walletInfo?.config.session.maxFailedAttempts ?? 0;
  }

  get disableForm(): boolean {
    return this.loading() || this.isLockedOut();
  }

  async submit(): Promise<void> {
    if (this.unlockForm.invalid || this.disableForm) {
      this.unlockForm.markAllAsTouched();
      return;
    }

    const password = this.unlockForm.controls.password.value ?? '';
    if (!password) {
      this.unlockForm.controls.password.markAsTouched();
      return;
    }

    this.loading.set(true);
    this.lastError.set(null);

    try {
      const response = await this.walletBackend.unlockWallet({ password });
      const refreshed = await this.walletBackend.getWalletInfo();
      this.walletInfo = refreshed;
      this.walletInfoChange.emit(refreshed);

      if (refreshed.isLocked) {
        this.failureCount += 1;
        this.lastError.set(`Incorrect password. Remaining attempts: ${response.remainingAttempts}`);
        this.unlockForm.reset({ password: '' });
        this.startBackoff(response.remainingAttempts);
        this.failed.emit(response.remainingAttempts);
        return;
      }

      this.failureCount = 0;
      this.clearBackoff();
      this.unlockForm.reset({ password: '' });
      this.unlocked.emit();
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : `${cause}`;
      this.lastError.set(message);
      this.error.emit(message);
    } finally {
      this.loading.set(false);
    }
  }

  ngOnChanges(changes: SimpleChanges): void {
    if ('walletInfo' in changes) {
      const info = this.walletInfo;
      if (!info) {
        this.failureCount = 0;
        this.clearBackoff();
        return;
      }

      if (!info.isLocked) {
        this.failureCount = 0;
        this.clearBackoff();
        return;
      }

      const maxAttempts = info.config.session.maxFailedAttempts;
      this.failureCount = Math.max(0, maxAttempts - info.remainingAttempts);
    }
  }

  ngOnDestroy(): void {
    this.clearBackoff();
  }

  private startBackoff(remainingAttempts: number): void {
    const delay = this.computeBackoffDelayMs(remainingAttempts);
    if (delay <= 0 || typeof window === 'undefined') {
      return;
    }

    this.lockoutExpiry = Date.now() + delay;
    this.lockoutRemainingMs.set(delay);

    if (this.countdownTimer !== null) {
      window.clearInterval(this.countdownTimer);
      this.countdownTimer = null;
    }

    this.countdownTimer = window.setInterval(() => {
      const remaining = Math.max(0, this.lockoutExpiry - Date.now());
      if (remaining <= 0) {
        this.clearBackoff();
        return;
      }
      this.lockoutRemainingMs.set(remaining);
    }, 250);
  }

  private clearBackoff(): void {
    if (typeof window !== 'undefined' && this.countdownTimer !== null) {
      window.clearInterval(this.countdownTimer);
    }
    this.countdownTimer = null;
    this.lockoutExpiry = 0;
    this.lockoutRemainingMs.set(0);
  }

  private computeBackoffDelayMs(remainingAttempts: number): number {
    if (remainingAttempts <= 0) {
      return MAX_BACKOFF_MS;
    }

    const exponent = Math.max(0, Math.min(this.failureCount - 1, 6));
    const computed = BASE_BACKOFF_MS * Math.pow(2, exponent);
    return Math.min(MAX_BACKOFF_MS, computed);
  }
}
