/// Tauri API service for communicating with the Rust backend.
///
/// Exposes strongly typed wrappers around the wallet command surface so the
/// Angular layer stays in sync with the Rust DTOs.

import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { WalletBackend } from './wallet-backend.interface';
import {
  BalanceResponse,
  CastVoteRequest,
  CastVoteResponse,
  ChangePasswordRequest,
  ChangePasswordResponse,
  CreateWalletRequest,
  CreateWalletResponse,
  DelegateRequest,
  DelegateResponse,
  ExportWalletResponse,
  FormatAmountRequest,
  FormatAmountResponse,
  GetDelegationsResponse,
  GetProposalResponse,
  GetProposalsRequest,
  GetProposalsResponse,
  GetProposalVotesResponse,
  GetVotingPowerResponse,
  ImportWalletRequest,
  ImportWalletResponse,
  LockWalletResponse,
  SignMessageRequest,
  SignMessageResponse,
  TransactionHistoryResponse,
  UnlockWalletRequest,
  UnlockWalletResponse,
  ValidateAddressRequest,
  ValidateAddressResponse,
  VerifySignatureRequest,
  VerifySignatureResponse,
  WalletInfoResponse
} from '../types/wallet.types';

@Injectable({
  providedIn: 'root'
})
export class TauriService implements WalletBackend {
  // Wallet lifecycle
  async createWallet(request: CreateWalletRequest): Promise<CreateWalletResponse> {
    return invoke<CreateWalletResponse>('create_wallet', { request });
  }

  async importWallet(request: ImportWalletRequest): Promise<ImportWalletResponse> {
    return invoke<ImportWalletResponse>('import_wallet', { request });
  }

  async unlockWallet(request: UnlockWalletRequest): Promise<UnlockWalletResponse> {
    return invoke<UnlockWalletResponse>('unlock_wallet', { request });
  }

  async lockWallet(): Promise<LockWalletResponse> {
    return invoke<LockWalletResponse>('lock_wallet');
  }

  async getWalletInfo(): Promise<WalletInfoResponse> {
    return invoke<WalletInfoResponse>('get_wallet_info');
  }

  async exportWallet(): Promise<ExportWalletResponse> {
    return invoke<ExportWalletResponse>('export_wallet');
  }

  async changePassword(request: ChangePasswordRequest): Promise<ChangePasswordResponse> {
    return invoke<ChangePasswordResponse>('change_password', { request });
  }

  // Crypto helpers
  async signMessage(request: SignMessageRequest): Promise<SignMessageResponse> {
    return invoke<SignMessageResponse>('sign_message', { request });
  }

  async verifyMessageSignature(request: VerifySignatureRequest): Promise<VerifySignatureResponse> {
    return invoke<VerifySignatureResponse>('verify_message_signature', { request });
  }

  async validateAddress(request: ValidateAddressRequest): Promise<ValidateAddressResponse> {
    return invoke<ValidateAddressResponse>('validate_address', { request });
  }

  async formatAmount(request: FormatAmountRequest): Promise<FormatAmountResponse> {
    return invoke<FormatAmountResponse>('format_amount', { request });
  }

  // Blockchain communication methods
  async getBalance(address: string, nodeUrl?: string): Promise<BalanceResponse> {
    return invoke<BalanceResponse>('get_balance', { address, node_url: nodeUrl });
  }

  async getTransactionHistory(
    address: string,
    limit?: number,
    offset?: number,
    nodeUrl?: string
  ): Promise<TransactionHistoryResponse> {
    return invoke<TransactionHistoryResponse>('get_transaction_history', {
      address,
      limit,
      offset,
      node_url: nodeUrl
    });
  }

  async sendTransaction(request: {
    from_address: string;
    to_address: string;
    amount: string;
    fee?: string;
    memo?: string;
  }): Promise<{ transaction_id: string; status: string; fee_used: string; timestamp: string }> {
    return invoke('send_transaction', { request });
  }

  async estimateTransactionFee(request: {
    from_address: string;
    to_address: string;
    amount: string;
    priority?: 'low' | 'medium' | 'high';
  }): Promise<{
    estimated_fee: string;
    priority_fees: { low: string; medium: string; high: string };
    network_congestion: 'low' | 'medium' | 'high';
    estimated_confirmation_time: number;
  }> {
    return invoke('estimate_transaction_fee', { request });
  }

  async getTransactionStatus(transactionId: string): Promise<{
    transaction_id: string;
    status: 'pending' | 'confirmed' | 'failed';
    confirmations: number;
    block_height?: number;
    timestamp: string;
    error_message?: string;
  }> {
    return invoke('get_transaction_status', { transaction_id: transactionId });
  }

  // Staking methods
  async getValidators(): Promise<{ validators: any[] }> {
    return invoke<{ validators: any[] }>('get_validators');
  }

  async getUserDelegations(request: { userAddress: string }): Promise<{ delegations: any[] }> {
    return invoke<{ delegations: any[] }>('get_user_delegations', { request });
  }

  async getStakingRewards(request: { userAddress: string }): Promise<{ rewards: any }> {
    return invoke<{ rewards: any }>('get_staking_rewards', { request });
  }

  async getLockboxRecords(request: { userAddress: string }): Promise<{ records: any[] }> {
    return invoke<{ records: any[] }>('get_lockbox_records', { request });
  }

  async getAutoStakeStatus(request: { userAddress: string }): Promise<{ status: any }> {
    return invoke<{ status: any }>('get_auto_stake_status', { request });
  }

  async delegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any> {
    return invoke('delegate_tokens', { request });
  }

  async undelegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any> {
    return invoke('undelegate_tokens', { request });
  }

  async createLockboxStake(request: { account: string; amount: number; termMonths: number }): Promise<any> {
    return invoke('create_lockbox_stake', { request });
  }

  async toggleAutoStaking(request: { account: string; enable: boolean }): Promise<any> {
    return invoke('toggle_auto_staking', { request });
  }

  async claimStakingRewards(request: { account: string }): Promise<any> {
    return invoke('claim_staking_rewards', { request });
  }

  // Governance methods
  async getProposals(request?: GetProposalsRequest): Promise<GetProposalsResponse> {
    return invoke<GetProposalsResponse>('get_proposals', { request });
  }

  async getProposal(proposalId: number): Promise<GetProposalResponse> {
    return invoke<GetProposalResponse>('get_proposal', { proposal_id: proposalId });
  }

  async getProposalVotes(proposalId: number, limit?: number, offset?: number): Promise<GetProposalVotesResponse> {
    return invoke<GetProposalVotesResponse>('get_proposal_votes', {
      proposal_id: proposalId,
      limit,
      offset
    });
  }

  async getVotingPower(address: string): Promise<GetVotingPowerResponse> {
    return invoke<GetVotingPowerResponse>('get_voting_power', { address });
  }

  async getDelegations(address: string): Promise<GetDelegationsResponse> {
    return invoke<GetDelegationsResponse>('get_delegations', { address });
  }

  async castVote(request: CastVoteRequest): Promise<CastVoteResponse> {
    return invoke<CastVoteResponse>('cast_vote', { request });
  }

  async delegate(request: DelegateRequest): Promise<DelegateResponse> {
    return invoke<DelegateResponse>('delegate', { request });
  }

  // Biometric authentication methods
  async getBiometricStatus(): Promise<{ available: boolean; enrolled: boolean; supportedTypes: string[] }> {
    return invoke('get_biometric_status');
  }

  async authenticateBiometric(reason: string): Promise<{ success: boolean; token?: string }> {
    return invoke('authenticate_biometric', { reason });
  }

  // Push notification methods
  async getPushNotificationStatus(): Promise<{ available: boolean; enabled: boolean; permissionGranted: boolean }> {
    return invoke('get_push_notification_status');
  }

  async registerPushNotifications(request: {
    token: string;
    enableTransactionAlerts: boolean;
    enableStakingAlerts: boolean;
    enableGovernanceAlerts: boolean;
  }): Promise<{ success: boolean; registrationId?: string }> {
    return invoke('register_push_notifications', { request });
  }

  // Passkey/WebAuthn methods
  async createPasskey(request: {
    challenge: string;
    userId: string;
    userName: string;
    relyingPartyId: string;
  }): Promise<{ credentialId: string; publicKey: string }> {
    return invoke('create_passkey', { request });
  }

  async authenticatePasskey(request: {
    challenge: string;
    credentialIds: string[];
  }): Promise<{
    credentialId: string;
    authenticatorData: string;
    clientDataJson: string;
    signature: string;
  }> {
    return invoke('authenticate_passkey', { request });
  }
}