import { Injectable, signal, computed, inject } from '@angular/core';
import {
  BalanceResponse,
  ChangePasswordRequest,
  ChangePasswordResponse,
  CreateWalletRequest,
  CreateWalletResponse,
  ExportWalletResponse,
  FormatAmountRequest,
  FormatAmountResponse,
  ImportWalletRequest,
  ImportWalletResponse,
  LockWalletResponse,
  SignMessageRequest,
  SignMessageResponse,
  UnlockWalletRequest,
  UnlockWalletResponse,
  ValidateAddressRequest,
  ValidateAddressResponse,
  VerifySignatureRequest,
  VerifySignatureResponse,
  WalletInfoResponse,
  WalletSummary
} from '../types/wallet.types';
import { WebWalletBackendService } from './wallet-backend.service';
import { TransactionService, FeeEstimate, FeeEstimateRequest, SendTransactionRequest, SendTransactionResponse } from './transaction.service';
import { SessionStorageService } from './session-storage.service';

interface WalletViewState {
  summary: WalletSummary | null;
  address?: string | null;
}

@Injectable({
  providedIn: 'root'
})
export class WalletService {
  private walletInfo = signal<WalletInfoResponse | null>(null);
  private lastCreateResponse = signal<CreateWalletResponse | null>(null);
  private lastKnownAddress = signal<string | null>(null);
  private balanceInfo = signal<BalanceResponse | null>(null);
  private isBusy = signal(false);
  private lastError = signal<string | null>(null);

  readonly info = this.walletInfo.asReadonly();
  readonly busy = this.isBusy.asReadonly();
  readonly error = this.lastError.asReadonly();
  readonly metadata = computed(() => this.walletInfo()?.metadata ?? null);
  readonly exists = computed(() => this.walletInfo()?.exists ?? false);
  readonly isLocked = computed(() => this.walletInfo()?.isLocked ?? true);
  readonly remainingAttempts = computed(() => this.walletInfo()?.remainingAttempts ?? 0);
  readonly config = computed(() => this.walletInfo()?.config ?? null);
  readonly createdAddress = computed(() => {
    const known = this.lastKnownAddress();
    if (known) {
      return known;
    }

    const metadata = this.walletInfo()?.metadata;
    return metadata?.primaryAddress ?? null;
  });
  readonly balance = this.balanceInfo.asReadonly();
  readonly currentWallet = computed<WalletViewState | null>(() => {
    const metadata = this.walletInfo()?.metadata;
    if (!metadata) {
      return null;
    }
    return {
      summary: metadata,
      address: this.lastCreateResponse()?.address ?? metadata.primaryAddress ?? null
    };
  });

  private readonly walletBackend = inject(WebWalletBackendService);
  private readonly transactionService = inject(TransactionService);
  private readonly sessionStorage = inject(SessionStorageService);

  constructor() {
    void this.refresh();
  }

  async refresh(): Promise<void> {
    this.isBusy.set(true);
    this.lastError.set(null);
    try {
      const info = await this.walletBackend.getWalletInfo();
      this.walletInfo.set(info);
      const metadataAddress = info.metadata?.primaryAddress ?? null;
      if (metadataAddress) {
        this.lastKnownAddress.set(metadataAddress);
      } else if (!info.exists) {
        this.lastKnownAddress.set(null);
      }
      // Also refresh balance if wallet is unlocked
      if (info.exists && !info.isLocked) {
        await this.getBalance();
      } else {
        this.balanceInfo.set(null);
      }
    } catch (error) {
      console.error('Failed to load wallet info:', error);
      this.lastError.set(`${error}`);
      this.walletInfo.set(null);
      this.balanceInfo.set(null);
    } finally {
      this.isBusy.set(false);
    }
  }

  async createWallet(request: CreateWalletRequest): Promise<CreateWalletResponse> {
    const response = await this.walletBackend.createWallet(request);
    this.lastCreateResponse.set(response);
    this.lastKnownAddress.set(response.address);
    await this.refresh();
    return response;
  }

  async importWallet(request: ImportWalletRequest): Promise<ImportWalletResponse> {
    const response = await this.walletBackend.importWallet(request);
    this.lastKnownAddress.set(response.address);
    await this.refresh();
    return response;
  }

  async unlockWallet(request: UnlockWalletRequest): Promise<UnlockWalletResponse> {
    const response = await this.walletBackend.unlockWallet(request);
    await this.refresh();
    return response;
  }

  async lockWallet(): Promise<LockWalletResponse> {
    const response = await this.walletBackend.lockWallet();
    await this.refresh();
    return response;
  }

  async exportWallet(): Promise<ExportWalletResponse> {
    return this.walletBackend.exportWallet();
  }

  async changePassword(request: ChangePasswordRequest): Promise<ChangePasswordResponse> {
    const response = await this.walletBackend.changePassword(request);
    await this.refresh();
    return response;
  }

  async signMessage(request: SignMessageRequest): Promise<SignMessageResponse> {
    return this.walletBackend.signMessage(request);
  }

  async verifyMessageSignature(request: VerifySignatureRequest): Promise<VerifySignatureResponse> {
    return this.walletBackend.verifyMessageSignature(request);
  }

  async validateAddress(request: ValidateAddressRequest): Promise<ValidateAddressResponse> {
    return this.walletBackend.validateAddress(request);
  }

  async formatAmount(request: FormatAmountRequest): Promise<FormatAmountResponse> {
    return this.walletBackend.formatAmount(request);
  }

  async getBalance(): Promise<void> {
    const address = this.createdAddress();
    if (!address || this.isLocked()) {
      this.balanceInfo.set(null);
      return;
    }

    try {
      const primaryEndpoint = this.walletInfo()?.config?.network.primaryEndpoint?.trim();
      const balance = await this.walletBackend.getBalance(address, primaryEndpoint || undefined);
      this.balanceInfo.set(balance);
    } catch (error) {
      console.error('Failed to get balance:', error);
      this.balanceInfo.set(null);
    }
  }

  /**
   * Estimate transaction fee
   */
  async estimateTransactionFee(request: FeeEstimateRequest): Promise<FeeEstimate> {
    const nodeUrl = this.getNodeUrl();
    return this.transactionService.estimateFee(nodeUrl, request);
  }

  /**
   * Send a transaction
   */
  async sendTransaction(params: {
    from_address: string;
    to_address: string;
    amount: string;
    fee: string;
    memo?: string;
  }): Promise<SendTransactionResponse> {
    if (this.isLocked()) {
      throw new Error('Wallet is locked. Please unlock to send transactions.');
    }

    const nodeUrl = this.getNodeUrl();

    // Get current nonce
    const nonce = await this.transactionService.getAccountNonce(nodeUrl, params.from_address);

    // Create transaction data for signing
    const txData = this.transactionService.createTransactionData(
      params.from_address,
      params.to_address,
      params.amount,
      params.fee,
      nonce,
      params.memo
    );

    // Sign the transaction using wallet backend
    const signResult = await this.walletBackend.signMessage({ message: txData });

    // Send the signed transaction
    const result = await this.transactionService.sendTransaction(nodeUrl, {
      from_address: params.from_address,
      to_address: params.to_address,
      amount: params.amount,
      fee: params.fee,
      memo: params.memo,
      signature: signResult.signatureHex,
      public_key: signResult.publicKeyHex,
      nonce
    });

    // Refresh balance after sending
    await this.getBalance();

    return result;
  }

  /**
   * Get the configured node URL
   */
  private getNodeUrl(): string {
    const config = this.walletInfo()?.config;
    return config?.network.primaryEndpoint?.trim() || 'http://192.168.20.25:18080';
  }
}
