import { Component, OnInit, OnDestroy, ElementRef, ViewChild, AfterViewInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { ReactiveFormsModule, FormBuilder, FormGroup, Validators } from '@angular/forms';
import { WasmService } from './core/services/wasm.service';
import { WalletInfoResponse, CreateWalletRequest, ImportWalletRequest } from './core/types/wallet.types';
import { WALLET_BACKEND, WalletBackend } from './core/services/wallet-backend.interface';
import { ModalHostComponent } from './core/ui/modal-host/modal-host.component';
import { UnlockWalletComponent } from './core/ui/unlock-wallet/unlock-wallet.component';
import { ThemeToggleComponent } from './core/ui/theme-toggle/theme-toggle.component';

import { ModalDismissedError, ModalService } from './core/services/modal.service';

interface TutorialStep {
  title: string;
  description: string;
  icon: string;
  background: string;
}

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, ReactiveFormsModule, ModalHostComponent, UnlockWalletComponent, ThemeToggleComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.scss'
})
export class AppComponent implements OnInit, OnDestroy, AfterViewInit {
  @ViewChild('parallaxContainer', { static: false }) parallaxContainer?: ElementRef;
  
  title = 'Chert Wallet';
  walletInfo: WalletInfoResponse | null = null;
  loading = false;
  errorMessage: string | null = null;
  
  // Fast loading state - shows spinner while initializing if wallet exists
  initializing = true;
  
  showTutorial = false;
  currentTutorialStep = 0;
  
  tutorialSteps: TutorialStep[] = [
    {
      title: 'Secure Wallet Creation',
      description: 'Generate cryptographically secure wallets with advanced encryption protocols.',
      icon: '🔐',
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)'
    },
    {
      title: 'Seamless Transactions',
      description: 'Send and receive Chert tokens with lightning-fast, low-cost transactions.',
      icon: '⚡',
      background: 'linear-gradient(135deg, #4facfe 0%, #00f2fe 100%)'
    },
    {
      title: 'Staking & Rewards',
      description: 'Participate in network consensus and earn rewards through secure staking.',
      icon: '💎',
      background: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)'
    },
    {
      title: 'Governance Power',
      description: 'Shape the future of Chert by participating in decentralized governance.',
      icon: '🗳️',
      background: 'linear-gradient(135deg, #fad961 0%, #f76b1c 100%)'
    }
  ];

  private scrollListener?: () => void;
  private static readonly NEW_PASSWORD_MIN_LENGTH = 12;

  private readonly walletBackend = inject<WalletBackend>(WALLET_BACKEND);
  private readonly wasmService = inject(WasmService);
  private readonly formBuilder = inject(FormBuilder);
  private readonly modalService = inject(ModalService);

  createForm: FormGroup = this.formBuilder.group({
    walletName: ['Main Wallet', [Validators.required, Validators.maxLength(64)]],
    password: ['', [Validators.required, Validators.minLength(8)]],
    confirmPassword: ['', [Validators.required]]
  });

  importForm: FormGroup = this.formBuilder.group({
    walletName: ['Imported Wallet', [Validators.required, Validators.maxLength(64)]],
    password: ['', [Validators.required, Validators.minLength(8)]],
    mnemonic: ['', [Validators.required, Validators.minLength(12)]]
  });

  toastMessage: string | null = null;
  toastVariant: 'success' | 'error' | 'info' = 'info';
  private toastTimer: number | null = null;

  setupMode: 'create' | 'import' = 'create';

  // Check localStorage synchronously to determine if wallet likely exists
  // This allows instant UI decision before async operations complete
  private static readonly WALLET_DATA_KEY = 'chert_web_wallet_data';
  private static readonly SETUP_MODE_KEY = 'chert_wallet_setup_mode';
  
  constructor() {
    // Apply theme immediately from localStorage (before any async work)
    this.applyStoredTheme();

    this.setupMode = this.readStoredSetupMode();

    // Check if wallet data exists in localStorage
    // If it does, we'll show a loading spinner instead of sign-in page
    this.checkStoredWallet();
  }

  private applyStoredTheme(): void {
    if (typeof window === 'undefined' || typeof localStorage === 'undefined') {
      return;
    }
    
    const savedTheme = localStorage.getItem('chert_wallet_theme');
    const prefersDark = window.matchMedia?.('(prefers-color-scheme: dark)').matches;
    
    let isDark = false;
    if (savedTheme === 'dark') {
      isDark = true;
    } else if (savedTheme === 'system' || !savedTheme) {
      isDark = prefersDark;
    }
    
    // Apply theme class immediately to prevent flash
    document.documentElement.classList.remove('light', 'dark');
    document.documentElement.classList.add(isDark ? 'dark' : 'light');
  }

  private checkStoredWallet(): void {
    if (typeof window === 'undefined' || typeof localStorage === 'undefined') {
      this.initializing = false;
      return;
    }

    const walletData = localStorage.getItem(AppComponent.WALLET_DATA_KEY);
    if (walletData) {
      // Wallet data exists - keep initializing=true to show loading spinner
      // The actual wallet info will be loaded in ngOnInit
      this.initializing = true;
    } else {
      // No wallet data - show sign-in page immediately
      this.initializing = false;
      // No wallet => default to Create unless user explicitly picked Import.
      const storedMode = localStorage.getItem(AppComponent.SETUP_MODE_KEY);
      if (storedMode !== 'import') {
        this.setupMode = 'create';
      }
    }
  }

  setSetupMode(mode: 'create' | 'import'): void {
    this.setupMode = mode;

    if (typeof window === 'undefined' || typeof localStorage === 'undefined') {
      return;
    }

    localStorage.setItem(AppComponent.SETUP_MODE_KEY, mode);
  }

  private readStoredSetupMode(): 'create' | 'import' {
    if (typeof window === 'undefined' || typeof localStorage === 'undefined') {
      return 'create';
    }

    const stored = localStorage.getItem(AppComponent.SETUP_MODE_KEY);
    return stored === 'import' ? 'import' : 'create';
  }

  async ngOnInit() {
    try {
      // Initialize WASM module (optional - wallet can work without it for basic operations)
      console.log('Initializing WASM module...');
      try {
        await this.wasmService.waitForReady();
        console.log('WASM module ready');
      } catch (wasmError) {
        // WASM is optional for basic wallet unlock operations
        console.warn('WASM module not available (this is OK for basic operations):', wasmError);
      }

      // Check if wallet exists by attempting to get wallet info
      // This should work even without WASM since it just reads localStorage
      this.walletInfo = await this.walletBackend.getWalletInfo();
      console.log('Wallet info loaded:', { 
        exists: this.walletInfo?.exists, 
        isLocked: this.walletInfo?.isLocked 
      });
    } catch (error) {
      console.error('Failed to get wallet info:', error);
      // Only set to null if we truly can't get wallet info
      // This preserves the unlock screen if wallet data exists
      this.walletInfo = null;
    } finally {
      // Initialization complete - hide spinner
      this.initializing = false;
    }
  }

  ngAfterViewInit() {
    if (typeof window === 'undefined' || typeof document === 'undefined') {
      return;
    }

    this.setupParallaxEffects();
    this.setupScrollAnimations();
  }

  ngOnDestroy() {
    if (this.scrollListener && typeof window !== 'undefined') {
      window.removeEventListener('scroll', this.scrollListener);
    }

    if (this.toastTimer && typeof window !== 'undefined') {
      window.clearTimeout(this.toastTimer);
      this.toastTimer = null;
    }
  }

  hasWallet(): boolean {
    return (this.walletInfo?.exists ?? false) && !this.walletInfo?.isLocked;
  }

  // Returns true if wallet exists but is locked (needs unlock)
  isWalletLocked(): boolean {
    return (this.walletInfo?.exists ?? false) && (this.walletInfo?.isLocked ?? false);
  }

  // Returns true if no wallet exists (needs create/import)
  needsWalletSetup(): boolean {
    return !(this.walletInfo?.exists ?? false);
  }

  isLoading(): boolean {
    return this.loading;
  }

  error(): string | null {
    return this.errorMessage;
  }

  async refreshWalletInfo() {
    this.loading = true;
    this.errorMessage = null;

    try {
      this.walletInfo = await this.walletBackend.getWalletInfo();
    } catch (error) {
      this.errorMessage = `Failed to refresh wallet: ${error}`;
      console.error(error);
    } finally {
      this.loading = false;
    }
  }

  async lockWallet() {
    try {
      await this.walletBackend.lockWallet();
      this.walletInfo = await this.walletBackend.getWalletInfo();
      this.showToast('Wallet locked', 'info');
    } catch (error) {
      this.errorMessage = `Failed to lock wallet: ${error}`;
      this.showToast('Failed to lock wallet', 'error');
      console.error(error);
    }
  }

  async handleExportWallet(): Promise<void> {
    if (!this.walletInfo?.exists) {
      return;
    }

    let password: string;
    try {
      password = await this.modalService.requestPassword({
        title: 'Export Wallet',
        description: 'Enter your password to unlock the mnemonic. It will be displayed once unlocked.',
        confirmLabel: 'Unlock Wallet',
        cancelLabel: 'Cancel',
        passwordMinLength: 8
      });
    } catch (error) {
      if (error instanceof ModalDismissedError) {
        return;
      }
      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to export wallet: ${message}`;
      this.showToast('Failed to export wallet', 'error');
      console.error(error);
      return;
    }

    let unlocked = false;
    this.errorMessage = null;
    this.loading = true;

    try {
      const unlockResult = await this.walletBackend.unlockWallet({ password });
      this.walletInfo = await this.walletBackend.getWalletInfo();
      unlocked = !(this.walletInfo?.isLocked ?? true);

      if (!unlocked) {
        this.showToast(`Incorrect password. Remaining attempts: ${unlockResult.remainingAttempts}`, 'error');
        return;
      }

      const exportResult = await this.walletBackend.exportWallet();
      if (!exportResult.mnemonic) {
        throw new Error('Mnemonic export is unavailable');
      }

      this.modalService.openMnemonic(exportResult.mnemonic, 'Exported Wallet Mnemonic');
      this.showToast('Wallet mnemonic exported. Copy it securely.', 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to export wallet: ${message}`;
      this.showToast('Failed to export wallet', 'error');
      console.error(error);
    } finally {
      try {
        if (unlocked) {
          await this.walletBackend.lockWallet();
        }
      } catch (lockError) {
        console.error('Failed to re-lock wallet after export:', lockError);
      }

      try {
        this.walletInfo = await this.walletBackend.getWalletInfo();
      } catch (infoError) {
        console.error('Failed to refresh wallet info after export:', infoError);
      }

      this.loading = false;
    }
  }

  async handleChangePassword(): Promise<void> {
    if (!this.hasWallet()) {
      this.showToast('Unlock your wallet to change the password.', 'info');
      return;
    }

    let currentPassword: string;
    try {
      currentPassword = await this.modalService.requestPassword({
        title: 'Verify Current Password',
        description: 'Enter your current password to continue.',
        confirmLabel: 'Continue',
        cancelLabel: 'Cancel',
        passwordMinLength: 8
      });
    } catch (error) {
      if (error instanceof ModalDismissedError) {
        return;
      }
      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to change password: ${message}`;
      this.showToast('Failed to change password', 'error');
      console.error(error);
      return;
    }

    this.errorMessage = null;
    this.loading = true;
    let unlockedForChange = false;

    try {
      const unlockResult = await this.walletBackend.unlockWallet({ password: currentPassword });
      this.walletInfo = await this.walletBackend.getWalletInfo();
      unlockedForChange = !(this.walletInfo?.isLocked ?? true);

      if (!unlockedForChange) {
        this.showToast(`Incorrect password. Remaining attempts: ${unlockResult.remainingAttempts}`, 'error');
        return;
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to change password: ${message}`;
      this.showToast('Failed to change password', 'error');
      console.error(error);
      return;
    } finally {
      this.loading = false;
    }

    let newPassword: string;
    try {
      newPassword = await this.promptForNewPassword();
    } catch (error) {
      if (error instanceof ModalDismissedError) {
        if (unlockedForChange) {
          try {
            await this.walletBackend.lockWallet();
          } catch (lockError) {
            console.error('Failed to re-lock wallet after cancellation:', lockError);
          }
          try {
            this.walletInfo = await this.walletBackend.getWalletInfo();
          } catch (infoError) {
            console.error('Failed to refresh wallet info after cancellation:', infoError);
          }
          unlockedForChange = false;
        }
        return;
      }

      if (unlockedForChange) {
        try {
          await this.walletBackend.lockWallet();
        } catch (lockError) {
          console.error('Failed to re-lock wallet after password prompt error:', lockError);
        }
        try {
          this.walletInfo = await this.walletBackend.getWalletInfo();
        } catch (infoError) {
          console.error('Failed to refresh wallet info after password prompt error:', infoError);
        }
        unlockedForChange = false;
      }

      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to change password: ${message}`;
      this.showToast('Failed to change password', 'error');
      console.error(error);
      return;
    }

    this.loading = true;
    this.errorMessage = null;

    try {
      await this.walletBackend.changePassword({ currentPassword, newPassword });
      try {
        await this.walletBackend.lockWallet();
        unlockedForChange = false;
      } catch (lockError) {
        console.error('Failed to lock wallet after password change:', lockError);
      }
      this.showToast('Password updated. Wallet locked for security.', 'success');
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      this.errorMessage = `Failed to change password: ${message}`;
      this.showToast('Failed to change password', 'error');
      console.error(error);
    } finally {
      if (unlockedForChange) {
        try {
          await this.walletBackend.lockWallet();
        } catch (lockError) {
          console.error('Failed to ensure wallet locked after password change error:', lockError);
        }
      }

      try {
        this.walletInfo = await this.walletBackend.getWalletInfo();
      } catch (infoError) {
        console.error('Failed to refresh wallet info after password change:', infoError);
      }

  this.loading = false;
    }
  }

  async submitCreateWallet() {
    if (this.createForm.invalid) {
      this.createForm.markAllAsTouched();
      return;
    }

    const { walletName, password, confirmPassword } = this.createForm.value;
    if (password !== confirmPassword) {
      this.showToast('Passwords do not match', 'error');
      return;
    }

    this.loading = true;
    this.errorMessage = null;

    try {
      const request: CreateWalletRequest = {
        walletName: walletName ?? 'Main Wallet',
        password: password ?? '',
        mnemonicWordCount: 24,
        usePostQuantum: false
      };

      const response = await this.walletBackend.createWallet(request);
      this.walletInfo = await this.walletBackend.getWalletInfo();
      this.showToast('Wallet created successfully. Remember to store your mnemonic safely.', 'success');
      this.revealMnemonic(response.mnemonic);
      this.createForm.reset({ walletName: 'Main Wallet', password: '', confirmPassword: '' });
    } catch (error) {
      const message = `Failed to generate wallet: ${error}`;
      this.errorMessage = message;
      this.showToast('Failed to create wallet', 'error');
      console.error(error);
    } finally {
      this.loading = false;
    }
  }

  async submitImportWallet() {
    if (this.importForm.invalid) {
      this.importForm.markAllAsTouched();
      return;
    }

    const { walletName, password, mnemonic } = this.importForm.value;
    this.loading = true;
    this.errorMessage = null;

    try {
      const request: ImportWalletRequest = {
        walletName: walletName ?? 'Imported Wallet',
        password: password ?? '',
        mnemonic: mnemonic ?? '',
        usePostQuantum: false
      };

      await this.walletBackend.importWallet(request);
      this.walletInfo = await this.walletBackend.getWalletInfo();
      this.showToast('Wallet imported successfully', 'success');
      this.importForm.reset({ walletName: 'Imported Wallet', password: '', mnemonic: '' });
    } catch (error) {
      const message = `Failed to import wallet: ${error}`;
      this.errorMessage = message;
      this.showToast('Failed to import wallet', 'error');
      console.error(error);
    } finally {
      this.loading = false;
    }
  }

  onWalletInfoChange(info: WalletInfoResponse) {
    this.walletInfo = info;
  }

  onUnlockSuccess() {
    this.showToast('Wallet unlocked', 'success');
  }

  onUnlockFailure(remainingAttempts: number) {
    this.showToast(`Incorrect password. Remaining attempts: ${remainingAttempts}`, 'error');
  }

  onUnlockError(message: string) {
    this.errorMessage = `Failed to unlock wallet: ${message}`;
    this.showToast('Failed to unlock wallet', 'error');
  }

  startTutorial() {
    this.showTutorial = true;
    this.currentTutorialStep = 0;
  }

  nextTutorialStep() {
    if (this.currentTutorialStep < this.tutorialSteps.length - 1) {
      this.currentTutorialStep++;
    } else {
      this.completeTutorial();
    }
  }

  previousTutorialStep() {
    if (this.currentTutorialStep > 0) {
      this.currentTutorialStep--;
    }
  }

  completeTutorial() {
    this.showTutorial = false;
    this.currentTutorialStep = 0;
  }

  clearError() {
    this.errorMessage = null;
  }

  trackByIndex(index: number): number {
    return index;
  }

  private async promptForNewPassword(): Promise<string> {
    const minimumLength = AppComponent.NEW_PASSWORD_MIN_LENGTH;
    const maxAttempts = 3;

    for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
      const newPassword = await this.modalService.requestPassword({
        title: attempt === 0 ? 'New Password' : 'New Password (Retry)',
        description: `Enter the new password (minimum ${minimumLength} characters).`,
        confirmLabel: 'Continue',
        cancelLabel: 'Cancel',
        passwordMinLength: minimumLength
      });

      const confirmation = await this.modalService.requestPassword({
        title: 'Confirm New Password',
        description: 'Re-enter the new password to confirm.',
        confirmLabel: 'Update Password',
        cancelLabel: 'Cancel',
        passwordMinLength: minimumLength
      });

      if (newPassword === confirmation) {
        return newPassword;
      }

      this.showToast('Passwords do not match, please try again.', 'error');
    }

    this.showToast('Password confirmation failed after multiple attempts.', 'error');
    throw new Error('Password confirmation failed after multiple attempts.');
  }

  private revealMnemonic(mnemonic: string) {
    this.toastMessage = null;
    // Display mnemonic within the page for copy, using alert-like behaviour in the UI
    this.errorMessage = null;
    this.modalService.openMnemonic(mnemonic);
  }

  private showToast(message: string, variant: 'success' | 'error' | 'info') {
    this.toastMessage = message;
    this.toastVariant = variant;

    if (this.toastTimer && typeof window !== 'undefined') {
      window.clearTimeout(this.toastTimer);
    }

    if (typeof window !== 'undefined') {
      this.toastTimer = window.setTimeout(() => {
        this.toastMessage = null;
        this.toastTimer = null;
      }, 4000);
    }
  }

  private setupParallaxEffects() {
    if (typeof window === 'undefined' || typeof document === 'undefined') {
      return;
    }

    this.scrollListener = () => {
      const scrollTop = window.pageYOffset;
      const parallaxElements = document.querySelectorAll('.parallax-element');
      
      parallaxElements.forEach((element, index) => {
        const speed = 0.5 + (index * 0.1);
        const yPos = -(scrollTop * speed);
        (element as HTMLElement).style.transform = `translate3d(0, ${yPos}px, 0)`;
      });
    };

    window.addEventListener('scroll', this.scrollListener, { passive: true });
  }

  private setupScrollAnimations() {
    if (
      typeof window === 'undefined' ||
      typeof document === 'undefined' ||
      !('IntersectionObserver' in window)
    ) {
      return;
    }

    const observerOptions = {
      threshold: 0.1,
      rootMargin: '0px 0px -50px 0px'
    };

    const observer = new window.IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          entry.target.classList.add('in-view');
        }
      });
    }, observerOptions);

    // Observe elements that should animate on scroll
    document.querySelectorAll('.animate-on-scroll').forEach(el => {
      observer.observe(el);
    });
  }
}