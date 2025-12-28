import { Component, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { RouterLink } from '@angular/router';
import { WalletService } from '../../core/services/wallet.service';
import { ModalService } from '../../core/services/modal.service';

interface FeeEstimate {
  estimated_fee: string;
  priority_fees: {
    low: string;
    medium: string;
    high: string;
  };
  network_congestion: 'low' | 'medium' | 'high';
  estimated_confirmation_time: number;
}

interface TransactionPreview {
  from_address: string;
  to_address: string;
  amount: string;
  fee: string;
  total: string;
  memo?: string;
}

@Component({
  selector: 'app-send-transaction',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule, RouterLink],
  templateUrl: './send-transaction.component.html',
  styleUrl: './send-transaction.component.scss'
})
export class SendTransactionComponent {
  private walletService = inject(WalletService);
  private modalService = inject(ModalService);
  private formBuilder = inject(FormBuilder);

  // Make global objects available in template
  readonly Math = Math;
  readonly parseFloat = parseFloat;

  readonly walletInfo = this.walletService.info;
  readonly balance = this.walletService.balance;
  readonly isLocked = this.walletService.isLocked;
  readonly exists = this.walletService.exists;
  readonly createdAddress = this.walletService.createdAddress;

  // Form state
  sendForm: FormGroup;
  isLoading = signal(false);
  feeEstimate = signal<FeeEstimate | null>(null);
  selectedPriority: 'low' | 'medium' | 'high' = 'medium';

  // Computed values
  readonly availableBalance = computed(() => {
    const bal = this.balance();
    return bal ? parseFloat(bal.balance) / 1e9 : 0; // Convert from base units
  });

  readonly maxSendAmount = computed(() => {
    const balance = this.availableBalance();
    const fee = this.feeEstimate();
    if (!fee) return balance;

    const feeAmount = parseFloat(fee.priority_fees[this.selectedPriority]) / 1e9;
    return Math.max(0, balance - feeAmount);
  });

  readonly isWalletReady = computed(() =>
     this.exists() && !this.isLocked() && this.createdAddress()
   );

  // Fee computed properties for template
  readonly lowFeeAmount = computed(() => {
    const fee = this.feeEstimate();
    return fee ? parseFloat(fee.priority_fees.low) / 1e9 : 0;
  });

  readonly mediumFeeAmount = computed(() => {
    const fee = this.feeEstimate();
    return fee ? parseFloat(fee.priority_fees.medium) / 1e9 : 0;
  });

  readonly highFeeAmount = computed(() => {
    const fee = this.feeEstimate();
    return fee ? parseFloat(fee.priority_fees.high) / 1e9 : 0;
  });

  readonly selectedFeeAmount = computed(() => {
    const fee = this.feeEstimate();
    return fee ? parseFloat(fee.priority_fees[this.selectedPriority]) / 1e9 : 0;
  });

  constructor() {
    this.sendForm = this.formBuilder.group({
      recipient: ['', [Validators.required, this.addressValidator.bind(this)]],
      amount: ['', [Validators.required, Validators.min(0.000000001), this.balanceValidator.bind(this)]],
      memo: ['', [Validators.maxLength(256)]],
      priority: ['medium']
    });

    // Watch for priority changes to update fee estimates
    this.sendForm.get('priority')?.valueChanges.subscribe(priority => {
      this.selectedPriority = priority;
      this.updateFeeEstimate();
    });

    // Watch for amount changes to validate
    this.sendForm.get('amount')?.valueChanges.subscribe(() => {
      this.sendForm.get('amount')?.updateValueAndValidity();
    });
  }

  private addressValidator(control: any): { [key: string]: any } | null {
    if (!control.value) return null;

    // Basic Chert address validation
    const chertAddressRegex = /^chert_[a-z0-9]{40,64}$/i;
    if (!chertAddressRegex.test(control.value)) {
      return { invalidAddress: true };
    }

    return null;
  }

  private balanceValidator(control: any): { [key: string]: any } | null {
    if (!control.value) return null;

    const amount = parseFloat(control.value);
    if (isNaN(amount) || amount <= 0) {
      return { invalidAmount: true };
    }

    const maxAmount = this.maxSendAmount();
    if (amount > maxAmount) {
      return { insufficientBalance: true };
    }

    return null;
  }

  async ngOnInit() {
    if (this.isWalletReady()) {
      await this.updateFeeEstimate();
    }
  }

  async updateFeeEstimate() {
    if (!this.isWalletReady()) return;

    try {
      const address = this.createdAddress()!;
      const feeEstimate = await this.walletService.estimateTransactionFee({
        from_address: address,
        to_address: this.sendForm.get('recipient')?.value || '',
        amount: '1000000000', // Use a sample amount for estimation
        priority: this.selectedPriority
      });

      this.feeEstimate.set(feeEstimate);
    } catch (error) {
      console.error('Failed to estimate fee:', error);
      // Set fallback fee estimate
      this.feeEstimate.set({
        estimated_fee: '1000000000',
        priority_fees: {
          low: '500000000',
          medium: '1000000000',
          high: '2000000000'
        },
        network_congestion: 'medium',
        estimated_confirmation_time: 2
      });
    }
  }

  async sendTransaction() {
    if (this.sendForm.invalid || !this.isWalletReady()) {
      this.sendForm.markAllAsTouched();
      return;
    }

    const formValue = this.sendForm.value;
    const feeEstimate = this.feeEstimate();

    if (!feeEstimate) {
      alert('Unable to estimate transaction fee. Please try again.');
      return;
    }

    const transaction: TransactionPreview = {
      from_address: this.createdAddress()!,
      to_address: formValue.recipient,
      amount: (parseFloat(formValue.amount) * 1e9).toString(), // Convert to base units
      fee: feeEstimate.priority_fees[this.selectedPriority],
      total: ((parseFloat(formValue.amount) * 1e9) + parseFloat(feeEstimate.priority_fees[this.selectedPriority])).toString(),
      memo: formValue.memo || undefined
    };

    // Show confirmation modal
    const confirmed = await this.showTransactionConfirmation(transaction);
    if (!confirmed) return;

    await this.executeTransaction(transaction);
  }

  private async showTransactionConfirmation(transaction: TransactionPreview): Promise<boolean> {
    const modalContent = `
      <div class="transaction-confirmation">
        <h3>Confirm Transaction</h3>
        <div class="transaction-details">
          <div class="detail-row">
            <span class="label">From:</span>
            <span class="value">${this.truncateAddress(transaction.from_address)}</span>
          </div>
          <div class="detail-row">
            <span class="label">To:</span>
            <span class="value">${this.truncateAddress(transaction.to_address)}</span>
          </div>
          <div class="detail-row">
            <span class="label">Amount:</span>
            <span class="value">${this.formatAmount(transaction.amount)} CHERT</span>
          </div>
          <div class="detail-row">
            <span class="label">Fee:</span>
            <span class="value">${this.formatAmount(transaction.fee)} CHERT</span>
          </div>
          <div class="detail-row total">
            <span class="label">Total:</span>
            <span class="value">${this.formatAmount(transaction.total)} CHERT</span>
          </div>
          ${transaction.memo ? `<div class="detail-row"><span class="label">Memo:</span><span class="value">${transaction.memo}</span></div>` : ''}
        </div>
        <div class="warning">
          ⚠️ Please verify the recipient address carefully. Transactions cannot be reversed.
        </div>
      </div>
    `;

    return confirm(modalContent);
  }

  private async executeTransaction(transaction: TransactionPreview) {
    this.isLoading.set(true);

    try {
      const response = await this.walletService.sendTransaction({
        from_address: transaction.from_address,
        to_address: transaction.to_address,
        amount: transaction.amount,
        fee: transaction.fee,
        memo: transaction.memo
      });

      // Show success message
      alert(`Transaction sent successfully! TX ID: ${this.truncateAddress(response.transaction_id)}`);

      // Reset form
      this.sendForm.reset({ priority: 'medium' });

      // Refresh balance
      await this.walletService.getBalance();

    } catch (error) {
      console.error('Failed to send transaction:', error);
      alert(`Failed to send transaction: ${error}`);
    } finally {
      this.isLoading.set(false);
    }
  }

  setMaxAmount() {
    const maxAmount = this.maxSendAmount();
    this.sendForm.patchValue({ amount: maxAmount.toString() });
  }

  private formatAmount(amount: string): string {
    const num = parseFloat(amount) / 1e9; // Convert from base units
    return num.toLocaleString('en-US', { maximumFractionDigits: 9 });
  }

  private truncateAddress(address: string): string {
    if (address.length <= 20) return address;
    return address.substring(0, 10) + '...' + address.substring(address.length - 8);
  }

  getCongestionColor(): string {
    const fee = this.feeEstimate();
    if (!fee) return 'var(--text-secondary)';

    switch (fee.network_congestion) {
      case 'low': return 'var(--success-light)';
      case 'medium': return 'var(--warning-light)';
      case 'high': return 'var(--error-light)';
      default: return 'var(--text-secondary)';
    }
  }

  getCongestionText(): string {
    const fee = this.feeEstimate();
    if (!fee) return 'Unknown';

    switch (fee.network_congestion) {
      case 'low': return 'Low congestion';
      case 'medium': return 'Medium congestion';
      case 'high': return 'High congestion';
      default: return 'Unknown';
    }
  }
}