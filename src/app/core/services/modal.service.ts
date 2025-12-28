import { Injectable, computed, signal } from '@angular/core';

export type ModalVariant = 'mnemonic' | 'confirm' | 'password';

export class ModalDismissedError extends Error {
  constructor() {
    super('Modal dismissed');
    this.name = 'ModalDismissedError';
  }
}

export interface ModalRequest {
  title: string;
  description?: string;
  variant: ModalVariant;
  mnemonic?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  requiresPassword?: boolean;
  passwordMinLength?: number;
  onConfirm?: (password?: string) => Promise<void> | void;
  onCancel?: () => void;
}

export interface ModalState extends ModalRequest {
  id: number;
  resolving: boolean;
  errorMessage: string | null;
}

export interface PasswordPromptOptions {
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  passwordMinLength?: number;
}

@Injectable({ providedIn: 'root' })
export class ModalService {
  private readonly modalSignal = signal<ModalState | null>(null);
  private sequence = 0;

  readonly modal = computed(() => this.modalSignal());

  open(request: ModalRequest): void {
    const modal: ModalState = {
      ...request,
      id: ++this.sequence,
      resolving: false,
      errorMessage: null
    };
    this.modalSignal.set(modal);
  }

  openMnemonic(mnemonic: string, title = 'Wallet Mnemonic'): void {
    this.open({
      title,
      description: 'Copy and store this mnemonic securely. Anyone with access can control your funds.',
      variant: 'mnemonic',
      mnemonic,
      confirmLabel: 'Close',
      requiresPassword: false
    });
  }

  requestPassword(options: PasswordPromptOptions): Promise<string> {
    const {
      title,
      description,
      confirmLabel = 'Continue',
      cancelLabel = 'Cancel',
      passwordMinLength = 8
    } = options;

    if (!title) {
      throw new Error('Password prompt title is required');
    }

    let settled = false;

    return new Promise((resolve, reject) => {
      const resolveOnce = (value: string) => {
        if (settled) {
          return;
        }
        settled = true;
        resolve(value);
      };

      const rejectOnce = (reason: unknown) => {
        if (settled) {
          return;
        }
        settled = true;
        reject(reason);
      };

      this.open({
        title,
        description,
        variant: 'password',
        confirmLabel,
        cancelLabel,
        requiresPassword: true,
        passwordMinLength,
        onConfirm: (password) => {
          if (!password) {
            throw new Error('Password is required');
          }
          resolveOnce(password);
        },
        onCancel: () => {
          rejectOnce(new ModalDismissedError());
        }
      });
    });
  }

  close(): void {
    this.modalSignal.set(null);
  }

  cancel(): void {
    const modal = this.modalSignal();
    if (!modal) {
      return;
    }
    modal.onCancel?.();
    this.close();
  }

  async confirm(password?: string): Promise<void> {
    const modal = this.modalSignal();
    if (!modal) {
      return;
    }

    if (!modal.onConfirm) {
      this.close();
      return;
    }

    this.modalSignal.set({ ...modal, resolving: true, errorMessage: null });

    try {
      await modal.onConfirm(password);
      this.close();
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      this.modalSignal.set({ ...modal, resolving: false, errorMessage: message });
    }
  }
}
