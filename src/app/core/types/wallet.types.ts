/// Core DTOs mirroring `wallet/src-tauri/src/api/types.rs`

export type SignatureAlgorithm = 'Ed25519' | 'Dilithium2' | 'Kyber512';

export interface WalletSummary {
  walletName: string;
  createdAt: string;
  updatedAt: string;
  schemaVersion: number;
  primaryAddress: string | null;
  publicKeyHex: string | null;
  signatureAlgorithm: string | null;
  supportsPostQuantum: boolean | null;
}

export interface CreateWalletRequest {
  walletName: string;
  password: string;
  mnemonicWordCount: number;
  usePostQuantum: boolean;
}

export interface CreateWalletResponse {
  summary: WalletSummary;
  address: string;
  publicKey: string;
  mnemonic: string;
  supportsPostQuantum: boolean;
  algorithm: SignatureAlgorithm;
}

export interface ImportWalletRequest {
  walletName: string;
  password: string;
  mnemonic: string;
  usePostQuantum: boolean;
}

export interface ImportWalletResponse {
  summary: WalletSummary;
  address: string;
  publicKey: string;
  supportsPostQuantum: boolean;
  algorithm: SignatureAlgorithm;
}

export interface UnlockWalletRequest {
  password: string;
}

export interface UnlockWalletResponse {
  summary: WalletSummary;
  remainingAttempts: number;
}

export interface LockWalletResponse {
  locked: boolean;
}

export interface NetworkConfig {
  primaryEndpoint: string;
  failoverEndpoints: string[];
  allowUntrustedCerts: boolean;
}

export interface SessionConfig {
  autoLockMinutes: number;
  maxFailedAttempts: number;
}

export interface TelemetryConfig {
  enableAnalytics: boolean;
  allowErrorReports: boolean;
}

export interface WalletConfig {
  network: NetworkConfig;
  session: SessionConfig;
  telemetry: TelemetryConfig;
  environment: string;
  lastUpdated: string;
  version: number;
}

export interface WalletInfoResponse {
  exists: boolean;
  isLocked: boolean;
  remainingAttempts: number;
  metadata: WalletSummary | null;
  config: WalletConfig;
}

export interface ExportWalletResponse {
  summary: WalletSummary;
  mnemonic: string | null;
  seedHex: string;
  stealthMaterialHex: string | null;
  pqMaterialHex: string | null;
}

export interface ChangePasswordRequest {
  currentPassword: string;
  newPassword: string;
}

export interface ChangePasswordResponse {
  summary: WalletSummary;
}

export interface SignMessageRequest {
  message: string;
}

export interface SignMessageResponse {
  signatureHex: string;
  algorithm: SignatureAlgorithm;
  publicKeyHex: string;
}

export interface VerifySignatureRequest {
  message: string;
  signatureHex: string;
  publicKeyHex: string;
  algorithm: SignatureAlgorithm;
}

export interface VerifySignatureResponse {
  valid: boolean;
}

export interface ValidateAddressRequest {
  address: string;
}

export interface ValidateAddressResponse {
  isValid: boolean;
}

export interface FormatAmountRequest {
  amount: string;
  decimals?: number;
}

export interface FormatAmountResponse {
  formatted: string;
}

export interface BalanceResponse {
  address: string;
  balance: string;
  nonce: number;
}

export interface TransactionInfo {
  transactionId: string;
  fromAddress: string;
  toAddress: string;
  amount: string;
  fee: string;
  timestamp: string;
  status: string;
  blockHeight?: number;
}

export interface TransactionHistoryResponse {
  transactions: TransactionInfo[];
  totalCount: number;
}

// Governance types
export interface ProposalSummary {
  proposal_id: number;
  proposer: string;
  description: string;
  vote_start: number;
  vote_end: number;
  votes_for: number;
  votes_against: number;
  votes_abstain: number;
  state: string;
  created_at: number;
}

export interface ProposalDetail {
  proposal_id: number;
  proposer: string;
  targets: string[];
  values: string[];
  calldatas: string[];
  description: string;
  vote_start: number;
  vote_end: number;
  votes_for: number;
  votes_against: number;
  votes_abstain: number;
  state: string;
  executed_at?: number;
  created_at: number;
  updated_at: number;
  has_voted?: boolean;
  user_vote?: VoteInfo;
}

export interface VoteInfo {
  proposal_id: number;
  voter: string;
  support: number; // 0=Against, 1=For, 2=Abstain
  weight: number;
  reason?: string;
  voted_at: number;
}

export interface VotingPowerInfo {
  address: string;
  voting_power: number;
  delegated_power: number;
  total_power: number;
}

export interface DelegationInfo {
  delegator: string;
  validator: string;
  amount: number;
  shares: number;
  delegated_at: number;
  rewards_claimed: number;
}

export interface GetProposalsRequest {
  state?: string;
  proposer?: string;
  limit?: number;
  offset?: number;
}

export interface GetProposalsResponse {
  proposals: ProposalSummary[];
}

export interface GetProposalResponse {
  proposal: ProposalDetail;
}

export interface GetProposalVotesResponse {
  votes: VoteInfo[];
}

export interface GetVotingPowerResponse {
  voting_power: VotingPowerInfo;
}

export interface GetDelegationsResponse {
  delegations: DelegationInfo[];
}

export interface CastVoteRequest {
  proposal_id: number;
  voter: string;
  support: number; // 0=Against, 1=For, 2=Abstain
  reason?: string;
}

export interface CastVoteResponse {
  proposal_id: string;
  status: string;
  votes_for: number;
  votes_against: number;
  voter: string;
  vote_weight: number;
  approve: boolean;
  finalized: boolean;
}

export interface DelegateRequest {
  delegator: string;
  validator: string;
  amount: number;
}

export interface DelegateResponse {
  delegator: string;
  validator: string;
  amount: number;
  delegation: DelegationInfo;
}