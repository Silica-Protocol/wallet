import { CommonModule } from '@angular/common';
import { Component, effect, inject, signal } from '@angular/core';
import { FormControl, ReactiveFormsModule, Validators } from '@angular/forms';
import { ModalService } from '../../services/modal.service';

@Component({
  selector: 'app-modal-host',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './modal-host.component.html',
  styleUrl: './modal-host.component.scss'
})
export class ModalHostComponent {
  private readonly modalService = inject(ModalService);

  readonly modal = this.modalService.modal;
  readonly passwordControl = new FormControl('', { nonNullable: true });
  readonly copyFeedback = signal<string | null>(null);

  constructor() {
    effect(() => {
      const modal = this.modal();
      if (!modal) {
        this.passwordControl.reset('');
        this.passwordControl.clearValidators();
        this.passwordControl.updateValueAndValidity({ emitEvent: false });
        this.copyFeedback.set(null);
        return;
      }

      if (modal.requiresPassword) {
        const minLength = modal.passwordMinLength ?? 8;
        this.passwordControl.setValidators([Validators.required, Validators.minLength(minLength)]);
      } else {
        this.passwordControl.clearValidators();
      }
      this.passwordControl.reset('');
      this.passwordControl.updateValueAndValidity({ emitEvent: false });
      this.copyFeedback.set(null);
    });
  }

  get isOpen(): boolean {
    return this.modal() !== null;
  }

  get canConfirm(): boolean {
    const modal = this.modal();
    if (!modal || modal.resolving) {
      return false;
    }

    if (modal.requiresPassword) {
      return this.passwordControl.valid;
    }

    return true;
  }

  get confirmLabel(): string {
    return this.modal()?.confirmLabel ?? 'Confirm';
  }

  get cancelLabel(): string {
    return this.modal()?.cancelLabel ?? 'Cancel';
  }

  onClose(): void {
    this.modalService.cancel();
  }

  onCancel(): void {
    this.modalService.cancel();
  }

  onConfirm(): void {
    const modal = this.modal();
    if (!modal) {
      return;
    }

    const password = modal.requiresPassword ? this.passwordControl.value : undefined;
    void this.modalService.confirm(password);
  }

  async onCopyMnemonic(mnemonic: string): Promise<void> {
    if (!mnemonic) {
      return;
    }

    try {
      if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(mnemonic);
      } else if (typeof document !== 'undefined') {
        const textarea = document.createElement('textarea');
        textarea.value = mnemonic;
        textarea.setAttribute('readonly', '');
        textarea.style.position = 'absolute';
        textarea.style.left = '-9999px';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
      } else {
        throw new Error('Clipboard API unavailable');
      }
      this.copyFeedback.set('Mnemonic copied to clipboard');
    } catch (error) {
      const message = error instanceof Error ? error.message : `${error}`;
      this.copyFeedback.set(`Copy failed: ${message}`);
    }
  }

  trackByIndex(index: number): number {
    return index;
  }
}
