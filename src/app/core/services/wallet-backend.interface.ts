import { InjectionToken } from '@angular/core';
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

export interface WalletBackend {
  createWallet(request: CreateWalletRequest): Promise<CreateWalletResponse>;
  importWallet(request: ImportWalletRequest): Promise<ImportWalletResponse>;
  unlockWallet(request: UnlockWalletRequest): Promise<UnlockWalletResponse>;
  lockWallet(): Promise<LockWalletResponse>;
  getWalletInfo(): Promise<WalletInfoResponse>;
  exportWallet(): Promise<ExportWalletResponse>;
  changePassword(request: ChangePasswordRequest): Promise<ChangePasswordResponse>;
  signMessage(request: SignMessageRequest): Promise<SignMessageResponse>;
  verifyMessageSignature(request: VerifySignatureRequest): Promise<VerifySignatureResponse>;
  validateAddress(request: ValidateAddressRequest): Promise<ValidateAddressResponse>;
  formatAmount(request: FormatAmountRequest): Promise<FormatAmountResponse>;
  getBalance(address: string, nodeUrl?: string): Promise<BalanceResponse>;
  getTransactionHistory(address: string, limit?: number, offset?: number, nodeUrl?: string): Promise<TransactionHistoryResponse>;

  // Staking methods
  getValidators(): Promise<{ validators: any[] }>;
  getUserDelegations(request: { userAddress: string }): Promise<{ delegations: any[] }>;
  getStakingRewards(request: { userAddress: string }): Promise<{ rewards: any }>;
  getLockboxRecords(request: { userAddress: string }): Promise<{ records: any[] }>;
  getAutoStakeStatus(request: { userAddress: string }): Promise<{ status: any }>;
  delegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any>;
  undelegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any>;
  createLockboxStake(request: { account: string; amount: number; termMonths: number }): Promise<any>;
  toggleAutoStaking(request: { account: string; enable: boolean }): Promise<any>;
  claimStakingRewards(request: { account: string }): Promise<any>;

  // Governance methods
  getProposals(request?: GetProposalsRequest): Promise<GetProposalsResponse>;
  getProposal(proposalId: number): Promise<GetProposalResponse>;
  getProposalVotes(proposalId: number, limit?: number, offset?: number): Promise<GetProposalVotesResponse>;
  getVotingPower(address: string): Promise<GetVotingPowerResponse>;
  getDelegations(address: string): Promise<GetDelegationsResponse>;
  castVote(request: CastVoteRequest): Promise<CastVoteResponse>;
  delegate(request: DelegateRequest): Promise<DelegateResponse>;

  // Biometric authentication methods
  getBiometricStatus(): Promise<{ available: boolean; enrolled: boolean; supportedTypes: string[] }>;
  authenticateBiometric(reason: string): Promise<{ success: boolean; token?: string }>;

  // Push notification methods
  getPushNotificationStatus(): Promise<{ available: boolean; enabled: boolean; permissionGranted: boolean }>;
  registerPushNotifications(request: {
    token: string;
    enableTransactionAlerts: boolean;
    enableStakingAlerts: boolean;
    enableGovernanceAlerts: boolean;
  }): Promise<{ success: boolean; registrationId?: string }>;

  // Passkey/WebAuthn methods
  createPasskey(request: {
    challenge: string;
    userId: string;
    userName: string;
    relyingPartyId: string;
  }): Promise<{ credentialId: string; publicKey: string }>;
  authenticatePasskey(request: {
    challenge: string;
    credentialIds: string[];
  }): Promise<{
    credentialId: string;
    authenticatorData: string;
    clientDataJson: string;
    signature: string;
  }>;
}

export const WALLET_BACKEND = new InjectionToken<WalletBackend>('WalletBackend');
