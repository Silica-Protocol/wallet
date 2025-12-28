import { Injectable, Provider, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';

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
  WalletConfig,
  WalletInfoResponse,
  WalletSummary,
  SignatureAlgorithm
} from '../types/wallet.types';
import { WALLET_BACKEND, WalletBackend } from './wallet-backend.interface';

interface JsonRpcResponse<T> {
  jsonrpc: string;
  result?: T;
  error?: {
    code: number;
    message: string;
  };
  id: number;
}

interface StoredWallet {
  summary: WalletSummary;
  passwordHash: string;
  mnemonic: string;
  address: string;
  publicKey: string;
  supportsPostQuantum: boolean;
  algorithm: SignatureAlgorithm;
  config: WalletConfig;
  unlocked: boolean;
  remainingAttempts: number;
}

const WORD_LIST = [
  'chert',
  'silica',
  'network',
  'ledger',
  'consensus',
  'hash',
  'quantum',
  'secure',
  'wallet',
  'signal',
  'vector',
  'zero',
  'verifier',
  'spectrum',
  'stellar',
  'crystal'
];

// Use localStorage for persistent wallet data, sessionStorage for unlock state
const WALLET_DATA_KEY = 'chert_web_wallet_data';
const SESSION_KEY = 'chert_web_wallet_session';

@Injectable({ providedIn: 'root' })
export class WebWalletBackendService implements WalletBackend {
  private state: StoredWallet | null = null;
  private defaultNodeUrl = 'http://192.168.20.25:18080';

  private http = inject(HttpClient);

  constructor() {
    this.state = this.loadState();
  }

  async createWallet(request: CreateWalletRequest): Promise<CreateWalletResponse> {
    const algorithm: SignatureAlgorithm = request.usePostQuantum ? 'Dilithium2' : 'Ed25519';
    const publicKey = this.randomHex(64);
    const mnemonic = this.generateMnemonic(request.mnemonicWordCount);
    const address = this.generateAddress();
    const summary = this.buildSummary(
      request.walletName,
      address,
      publicKey,
      algorithm,
      request.usePostQuantum
    );
    const passwordHash = await this.hashSecret(request.password);
    const config = this.buildDefaultConfig();

    this.state = {
      summary,
      passwordHash,
      mnemonic,
      address,
      publicKey,
      supportsPostQuantum: request.usePostQuantum,
      algorithm,
      config,
      unlocked: true,
      remainingAttempts: config.session.maxFailedAttempts
    };

    this.persistState();

    return {
      summary,
      address,
      publicKey,
      mnemonic,
      supportsPostQuantum: request.usePostQuantum,
      algorithm
    };
  }

  async importWallet(request: ImportWalletRequest): Promise<ImportWalletResponse> {
    const algorithm: SignatureAlgorithm = request.usePostQuantum ? 'Dilithium2' : 'Ed25519';
    const publicKey = this.randomHex(64);
    const address = this.generateAddress();
    const summary = this.buildSummary(
      request.walletName,
      address,
      publicKey,
      algorithm,
      request.usePostQuantum
    );
    const passwordHash = await this.hashSecret(request.password);
    const config = this.buildDefaultConfig();

    this.state = {
      summary,
      passwordHash,
      mnemonic: request.mnemonic,
      address,
      publicKey,
      supportsPostQuantum: request.usePostQuantum,
      algorithm,
      config,
      unlocked: true,
      remainingAttempts: config.session.maxFailedAttempts
    };

    this.persistState();

    return {
      summary,
      address,
      publicKey,
      supportsPostQuantum: request.usePostQuantum,
      algorithm
    };
  }

  async unlockWallet(request: UnlockWalletRequest): Promise<UnlockWalletResponse> {
    const wallet = await this.requireState();
    const attemptedHash = await this.hashSecret(request.password);

    if (attemptedHash !== wallet.passwordHash) {
      wallet.unlocked = false;
      wallet.remainingAttempts = Math.max(0, wallet.remainingAttempts - 1);
      wallet.config.lastUpdated = new Date().toISOString();
      const result = {
        summary: wallet.summary,
        remainingAttempts: wallet.remainingAttempts
      };
      this.persistState();
      return result;
    }

    wallet.unlocked = true;
    wallet.remainingAttempts = wallet.config.session.maxFailedAttempts;
    wallet.config.lastUpdated = new Date().toISOString();

    this.persistState();

    return {
      summary: wallet.summary,
      remainingAttempts: wallet.remainingAttempts
    };
  }

  async lockWallet(): Promise<LockWalletResponse> {
    const wallet = await this.requireState();
    wallet.unlocked = false;
    wallet.config.lastUpdated = new Date().toISOString();
    this.persistState();
    return { locked: true };
  }

  async getWalletInfo(): Promise<WalletInfoResponse> {
    if (!this.state) {
      const config = this.buildDefaultConfig();
      return {
        exists: false,
        isLocked: true,
        remainingAttempts: 0,
        metadata: null,
        config
      };
    }

    const info = {
      exists: true,
      isLocked: !this.state.unlocked,
      remainingAttempts: this.state.remainingAttempts,
      metadata: this.state.summary,
      config: { ...this.state.config }
    };
    return info;
  }

  async exportWallet(): Promise<ExportWalletResponse> {
    const wallet = await this.requireUnlocked();
    const seedHex = await this.computeDigest(wallet.mnemonic, 'SHA-256');
    const stealthMaterialHex = await this.computeDigest(`${wallet.address}:stealth`, 'SHA-384');
    const pqMaterialHex = await this.computeDigest(`${wallet.address}:pq`, 'SHA-512');

    wallet.config.lastUpdated = new Date().toISOString();
    this.persistState();

    return {
      summary: wallet.summary,
      mnemonic: wallet.mnemonic,
      seedHex,
      stealthMaterialHex,
      pqMaterialHex
    };
  }

  async changePassword(request: ChangePasswordRequest): Promise<ChangePasswordResponse> {
    const wallet = await this.requireUnlocked();
    const currentHash = await this.hashSecret(request.currentPassword);
    if (currentHash !== wallet.passwordHash) {
      wallet.remainingAttempts = Math.max(0, wallet.remainingAttempts - 1);
      wallet.config.lastUpdated = new Date().toISOString();
      this.persistState();
      throw new Error('Current password is incorrect');
    }

    wallet.passwordHash = await this.hashSecret(request.newPassword);
    wallet.remainingAttempts = wallet.config.session.maxFailedAttempts;
    wallet.config.lastUpdated = new Date().toISOString();
    this.persistState();

    return {
      summary: wallet.summary
    };
  }

  async signMessage(request: SignMessageRequest): Promise<SignMessageResponse> {
    const wallet = await this.requireUnlocked();
    const signatureHex = await this.computeDigest(`${wallet.address}:${request.message}`, 'SHA-512');
    wallet.config.lastUpdated = new Date().toISOString();
    this.persistState();

    return {
      signatureHex,
      algorithm: wallet.algorithm,
      publicKeyHex: wallet.publicKey
    };
  }

  async verifyMessageSignature(request: VerifySignatureRequest): Promise<VerifySignatureResponse> {
    const expected = await this.computeDigest(`${request.publicKeyHex}:${request.message}`, 'SHA-512');
    return {
      valid: expected.toLowerCase() === request.signatureHex.toLowerCase()
    };
  }

  async validateAddress(request: ValidateAddressRequest): Promise<ValidateAddressResponse> {
    return {
      isValid: /^chert_[a-z0-9]{40,64}$/i.test(request.address)
    };
  }

  async formatAmount(request: FormatAmountRequest): Promise<FormatAmountResponse> {
    const decimals = request.decimals ?? 9;
    const formatted = this.formatTokenAmount(request.amount, decimals);
    return { formatted };
  }

  async getBalance(address: string, nodeUrl?: string): Promise<BalanceResponse> {
    const url = nodeUrl || this.defaultNodeUrl;
    const rpcUrl = `${url}/jsonrpc`;

    const request = {
      jsonrpc: '2.0',
      method: 'get_balance',
      params: { address },
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<BalanceResponse>>(rpcUrl, request).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      // Check for CORS or network errors
      const errorMessage = this.formatNetworkError(error, url);
      throw new Error(errorMessage);
    }
  }

  async getTransactionHistory(
    address: string,
    limit: number = 50,
    offset: number = 0,
    nodeUrl?: string
  ): Promise<TransactionHistoryResponse> {
    const url = nodeUrl || this.defaultNodeUrl;
    const rpcUrl = `${url}/jsonrpc`;

    const request = {
      jsonrpc: '2.0',
      method: 'get_transaction_history',
      params: { address, limit, offset },
      id: 2
    };

    try {
      const response = await this.http.post<JsonRpcResponse<TransactionHistoryResponse>>(rpcUrl, request).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, url);
      throw new Error(errorMessage);
    }
  }

  // Staking methods - web implementation with proper error handling
  async getValidators(): Promise<{ validators: any[] }> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const request = {
      jsonrpc: '2.0',
      method: 'get_validators',
      params: {},
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<{ validators: any[] }>>(rpcUrl, request).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, this.defaultNodeUrl);
      throw new Error(errorMessage);
    }
  }

  async getUserDelegations(request: { userAddress: string }): Promise<{ delegations: any[] }> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const rpcRequest = {
      jsonrpc: '2.0',
      method: 'get_user_delegations',
      params: { userAddress: request.userAddress },
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<{ delegations: any[] }>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, this.defaultNodeUrl);
      throw new Error(errorMessage);
    }
  }

  async getStakingRewards(request: { userAddress: string }): Promise<{ rewards: any }> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const rpcRequest = {
      jsonrpc: '2.0',
      method: 'get_staking_rewards',
      params: { userAddress: request.userAddress },
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<{ rewards: any }>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, this.defaultNodeUrl);
      throw new Error(errorMessage);
    }
  }

  async getLockboxRecords(request: { userAddress: string }): Promise<{ records: any[] }> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const rpcRequest = {
      jsonrpc: '2.0',
      method: 'get_lockbox_records',
      params: { userAddress: request.userAddress },
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<{ records: any[] }>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, this.defaultNodeUrl);
      throw new Error(errorMessage);
    }
  }

  async getAutoStakeStatus(request: { userAddress: string }): Promise<{ status: any }> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const rpcRequest = {
      jsonrpc: '2.0',
      method: 'get_auto_stake_status',
      params: { userAddress: request.userAddress },
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<{ status: any }>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error: unknown) {
      const errorMessage = this.formatNetworkError(error, this.defaultNodeUrl);
      throw new Error(errorMessage);
    }
  }

  async delegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any> {
    const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
    const rpcRequest = {
      jsonrpc: '2.0',
      method: 'delegate_tokens',
      params: request,
      id: 1
    };

    try {
      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to delegate tokens via node, operation may have failed:', error);
      throw error;
    }
  }

  async undelegateTokens(request: { delegatorAddress: string; validatorAddress: string; amount: number }): Promise<any> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'undelegate_tokens',
        params: request,
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to undelegate tokens via node, operation may have failed:', error);
      throw error;
    }
  }

  async createLockboxStake(request: { account: string; amount: number; termMonths: number }): Promise<any> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'create_lockbox_stake',
        params: request,
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to create lockbox stake via node, operation may have failed:', error);
      throw error;
    }
  }

  async toggleAutoStaking(request: { account: string; enable: boolean }): Promise<any> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'toggle_auto_staking',
        params: request,
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to toggle auto-staking via node, operation may have failed:', error);
      throw error;
    }
  }

  async claimStakingRewards(request: { account: string }): Promise<any> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'claim_staking_rewards',
        params: request,
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to claim staking rewards via node, operation may have failed:', error);
      throw error;
    }
  }

  // Governance methods - web implementation with fallback
  async getProposals(request?: GetProposalsRequest): Promise<GetProposalsResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'get_proposals',
        params: request || {},
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<GetProposalsResponse>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to get proposals from node, using mock data:', error);
      // Fallback to mock data
      return {
        proposals: [
          {
            proposal_id: 1,
            proposer: 'chert_proposer_001',
            description: 'Increase Block Size Limit: Proposal to increase the maximum block size from 1MB to 2MB to improve network throughput.',
            vote_start: Math.floor(Date.now() / 1000),
            vote_end: Math.floor(Date.now() / 1000) + 86400 * 7, // 7 days
            votes_for: 150000,
            votes_against: 25000,
            votes_abstain: 10000,
            state: 'active',
            created_at: Math.floor(Date.now() / 1000) - 86400
          }
        ]
      };
    }
  }

  async getProposal(proposalId: number): Promise<GetProposalResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'get_proposal',
        params: { proposalId },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<GetProposalResponse>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to get proposal from node, using mock data:', error);
      // Fallback to mock data
      return {
        proposal: {
          proposal_id: proposalId,
          proposer: 'chert_proposer_mock',
          targets: ['chert_contract_001'],
          values: ['0'],
          calldatas: ['0x0000000000000000000000000000000000000000000000000000000000200000'],
          description: 'Mock Proposal: This is a mock proposal for demonstration purposes.',
          vote_start: Math.floor(Date.now() / 1000),
          vote_end: Math.floor(Date.now() / 1000) + 86400 * 7,
          votes_for: 100000,
          votes_against: 20000,
          votes_abstain: 5000,
          state: 'active',
          created_at: Math.floor(Date.now() / 1000) - 86400,
          updated_at: Math.floor(Date.now() / 1000)
        }
      };
    }
  }

  async getProposalVotes(proposalId: number, limit?: number, offset?: number): Promise<GetProposalVotesResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'get_proposal_votes',
        params: { proposalId, limit, offset },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<GetProposalVotesResponse>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to get proposal votes from node, using mock data:', error);
      // Fallback to mock data
      return {
        votes: [
          {
            proposal_id: proposalId,
            voter: 'chert_voter_001',
            support: 1, // for
            weight: 1000,
            reason: 'Increasing block size will improve network performance',
            voted_at: Math.floor(Date.now() / 1000) - 3600
          }
        ]
      };
    }
  }

  async getVotingPower(address: string): Promise<GetVotingPowerResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'governance_get_voting_power',
        params: { address },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<GetVotingPowerResponse>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to get voting power from node, using mock data:', error);
      // Fallback to mock data
      return {
        voting_power: {
          address,
          voting_power: 10000,
          delegated_power: 5000,
          total_power: 15000
        }
      };
    }
  }

  async getDelegations(address: string): Promise<GetDelegationsResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'governance_get_delegations',
        params: { address },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<GetDelegationsResponse>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to get delegations from node, using mock data:', error);
      // Fallback to mock data
      return {
        delegations: [
          {
            delegator: address,
            validator: 'chert_validator_001',
            amount: 5_000,
            shares: 4_800,
            delegated_at: Math.floor(Date.now() / 1000) - 86_400,
            rewards_claimed: 1_250
          }
        ]
      };
    }
  }

  async castVote(request: CastVoteRequest): Promise<CastVoteResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'governance_cast_vote',
        params: {
          proposal_id: request.proposal_id.toString(),
          voter: request.voter,
          approve: request.support === 1,
          reason: request.reason
        },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result as CastVoteResponse;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to cast vote via node, operation may have failed:', error);
      throw error;
    }
  }

  async delegate(request: DelegateRequest): Promise<DelegateResponse> {
    try {
      const rpcUrl = `${this.defaultNodeUrl}/jsonrpc`;
      const rpcRequest = {
        jsonrpc: '2.0',
        method: 'governance_delegate_stake',
        params: {
          delegator: request.delegator,
          validator: request.validator,
          amount: request.amount
        },
        id: 1
      };

      const response = await this.http.post<JsonRpcResponse<any>>(rpcUrl, rpcRequest).toPromise();
      if (response && response.result) {
        return response.result as DelegateResponse;
      } else if (response && response.error) {
        throw new Error(`RPC Error: ${response.error.message}`);
      } else {
        throw new Error('Invalid response format');
      }
    } catch (error) {
      console.warn('Failed to delegate voting power via node, operation may have failed:', error);
      throw error;
    }
  }

  // Biometric authentication methods - web implementation
  async getBiometricStatus(): Promise<{ available: boolean; enrolled: boolean; supportedTypes: string[] }> {
    try {
      // Check if WebAuthn/passkeys are available
      if (typeof window !== 'undefined' && window.PublicKeyCredential) {
        // Check if any credentials are enrolled
        const credentials = await navigator.credentials.get({
          publicKey: {
            challenge: new Uint8Array(32),
            allowCredentials: [],
            userVerification: 'preferred'
          }
        } as any);

        return {
          available: true,
          enrolled: credentials !== null,
          supportedTypes: ['passkey', 'webauthn']
        };
      } else {
        return {
          available: false,
          enrolled: false,
          supportedTypes: []
        };
      }
    } catch (error) {
      console.warn('Failed to check biometric status:', error);
      return {
        available: false,
        enrolled: false,
        supportedTypes: []
      };
    }
  }

  async authenticateBiometric(reason: string): Promise<{ success: boolean; token?: string }> {
    try {
      // For web, we'll simulate biometric authentication with passkeys
      // In a real implementation, this would use WebAuthn
      if (typeof window !== 'undefined' && window.PublicKeyCredential) {
        // Simulate successful authentication
        return {
          success: true,
          token: 'web_auth_token_' + Date.now()
        };
      } else {
        throw new Error('Biometric authentication not available');
      }
    } catch (error) {
      console.warn('Biometric authentication failed:', error);
      return { success: false };
    }
  }

  async getPushNotificationStatus(): Promise<{ available: boolean; enabled: boolean; permissionGranted: boolean }> {
    try {
      if (typeof window !== 'undefined' && 'Notification' in window) {
        const permission = Notification.permission;
        return {
          available: true,
          enabled: permission === 'granted',
          permissionGranted: permission === 'granted'
        };
      } else {
        return {
          available: false,
          enabled: false,
          permissionGranted: false
        };
      }
    } catch (error) {
      console.warn('Failed to check push notification status:', error);
      return {
        available: false,
        enabled: false,
        permissionGranted: false
      };
    }
  }

  async registerPushNotifications(request: {
    token: string;
    enableTransactionAlerts: boolean;
    enableStakingAlerts: boolean;
    enableGovernanceAlerts: boolean;
  }): Promise<{ success: boolean; registrationId?: string }> {
    try {
      // Request permission if not already granted
      if (typeof window !== 'undefined' && 'Notification' in window) {
        if (Notification.permission === 'default') {
          const permission = await Notification.requestPermission();
          if (permission !== 'granted') {
            throw new Error('Push notification permission denied');
          }
        }

        // In a real implementation, register with push service
        // For now, simulate successful registration
        return {
          success: true,
          registrationId: 'web_push_' + Date.now()
        };
      } else {
        throw new Error('Push notifications not supported');
      }
    } catch (error) {
      console.warn('Failed to register push notifications:', error);
      return { success: false };
    }
  }

  async createPasskey(request: {
    challenge: string;
    userId: string;
    userName: string;
    relyingPartyId: string;
  }): Promise<{ credentialId: string; publicKey: string }> {
    try {
      if (typeof window !== 'undefined' && window.PublicKeyCredential) {
        const publicKeyCredentialCreationOptions: PublicKeyCredentialCreationOptions = {
          challenge: new Uint8Array(32), // Should be from server
          rp: {
            name: 'Chert Wallet',
            id: request.relyingPartyId
          },
          user: {
            id: new TextEncoder().encode(request.userId),
            name: request.userName,
            displayName: request.userName
          },
          pubKeyCredParams: [
            { alg: -7, type: 'public-key' }, // ES256
            { alg: -257, type: 'public-key' } // RS256
          ],
          authenticatorSelection: {
            authenticatorAttachment: 'platform',
            userVerification: 'required'
          },
          timeout: 60000,
          attestation: 'direct'
        };

        const credential = await navigator.credentials.create({
          publicKey: publicKeyCredentialCreationOptions
        }) as PublicKeyCredential;

        if (credential && credential.rawId) {
          return {
            credentialId: this.arrayBufferToBase64(credential.rawId),
            publicKey: 'passkey_created_' + Date.now() // Simplified
          };
        } else {
          throw new Error('Failed to create passkey');
        }
      } else {
        throw new Error('WebAuthn not supported');
      }
    } catch (error) {
      console.warn('Failed to create passkey:', error);
      throw error;
    }
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
    try {
      if (typeof window !== 'undefined' && window.PublicKeyCredential) {
        const allowCredentials = request.credentialIds.map(id => ({
          type: 'public-key' as const,
          id: this.base64ToArrayBuffer(id)
        }));

        const publicKeyCredentialRequestOptions: PublicKeyCredentialRequestOptions = {
          challenge: new Uint8Array(32), // Should be from server
          allowCredentials,
          userVerification: 'required',
          timeout: 60000
        };

        const assertion = await navigator.credentials.get({
          publicKey: publicKeyCredentialRequestOptions
        }) as PublicKeyCredential;

        if (assertion) {
          const response = assertion.response as any; // Type assertion for WebAuthn response
          return {
            credentialId: this.arrayBufferToBase64(assertion.rawId),
            authenticatorData: this.arrayBufferToBase64(response.authenticatorData),
            clientDataJson: this.arrayBufferToBase64(response.clientDataJSON),
            signature: this.arrayBufferToBase64(response.signature)
          };
        } else {
          throw new Error('Authentication failed');
        }
      } else {
        throw new Error('WebAuthn not supported');
      }
    } catch (error) {
      console.warn('Passkey authentication failed:', error);
      throw error;
    }
  }

  // Utility methods for WebAuthn
  private arrayBufferToBase64(buffer: ArrayBuffer): string {
    const bytes = new Uint8Array(buffer);
    let binary = '';
    for (let i = 0; i < bytes.byteLength; i++) {
      binary += String.fromCharCode(bytes[i]);
    }
    return btoa(binary);
  }

  private base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binaryString = atob(base64);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes.buffer;
  }

  private async requireState(): Promise<StoredWallet> {
    if (!this.state) {
      throw new Error('Wallet is not initialized');
    }
    return this.state;
  }

  private async requireUnlocked(): Promise<StoredWallet> {
    const wallet = await this.requireState();
    if (!wallet.unlocked) {
      throw new Error('Wallet is locked');
    }
    return wallet;
  }

  private loadState(): StoredWallet | null {
    if (typeof window === 'undefined') {
      return null;
    }

    try {
      // First, load wallet data from localStorage (persistent)
      const walletData = window.localStorage.getItem(WALLET_DATA_KEY);
      if (!walletData) {
        return null;
      }

      const parsed = JSON.parse(walletData) as StoredWallet;
      if (!parsed || !parsed.summary || !parsed.address) {
        return null;
      }

      // Check if there's an active session in sessionStorage
      const sessionData = window.sessionStorage.getItem(SESSION_KEY);
      if (sessionData) {
        try {
          const session = JSON.parse(sessionData);
          // Session is valid if not expired
          if (session.expiresAt && session.expiresAt > Date.now()) {
            parsed.unlocked = true;
          } else {
            // Session expired
            window.sessionStorage.removeItem(SESSION_KEY);
            parsed.unlocked = false;
          }
        } catch {
          parsed.unlocked = false;
        }
      } else {
        parsed.unlocked = false;
      }

      parsed.remainingAttempts = parsed.config.session.maxFailedAttempts;
      return parsed;
    } catch (error) {
      console.warn('Failed to restore web wallet state:', error);
      return null;
    }
  }

  private persistState(): void {
    if (typeof window === 'undefined') {
      return;
    }

    if (!this.state) {
      window.localStorage.removeItem(WALLET_DATA_KEY);
      window.sessionStorage.removeItem(SESSION_KEY);
      return;
    }

    try {
      // Store wallet data in localStorage (without unlock state)
      const walletData = { ...this.state, unlocked: false };
      window.localStorage.setItem(WALLET_DATA_KEY, JSON.stringify(walletData));

      // Store session in sessionStorage if unlocked
      if (this.state.unlocked) {
        const session = {
          address: this.state.address,
          unlockedAt: Date.now(),
          expiresAt: Date.now() + (30 * 60 * 1000) // 30 minutes
        };
        window.sessionStorage.setItem(SESSION_KEY, JSON.stringify(session));
      }
    } catch (error) {
      console.warn('Failed to persist web wallet state:', error);
    }
  }

  private buildSummary(
    walletName: string,
    primaryAddress: string | null,
    publicKeyHex: string | null,
    algorithm: SignatureAlgorithm | null,
    supportsPostQuantum: boolean | null
  ): WalletSummary {
    const timestamp = new Date().toISOString();
    return {
      walletName,
      createdAt: timestamp,
      updatedAt: timestamp,
      schemaVersion: 1,
      primaryAddress,
      publicKeyHex,
      signatureAlgorithm: algorithm,
      supportsPostQuantum
    };
  }

  private buildDefaultConfig(): WalletConfig {
    const timestamp = new Date().toISOString();
    return {
      network: {
        primaryEndpoint: 'http://192.168.20.25:18080',
        failoverEndpoints: ['http://192.168.20.25:18080'],
        allowUntrustedCerts: false
      },
      session: {
        autoLockMinutes: 15,
        maxFailedAttempts: 5
      },
      telemetry: {
        enableAnalytics: false,
        allowErrorReports: false
      },
      environment: 'web-mock',
      lastUpdated: timestamp,
      version: 1
    };
  }

  private generateMnemonic(wordCount: number): string {
    const count = Math.min(Math.max(wordCount, 12), 24);
    const words: string[] = [];
    for (let i = 0; i < count; i += 1) {
      const idx = this.randomInt(0, WORD_LIST.length - 1);
      words.push(WORD_LIST[idx]);
    }
    return words.join(' ');
  }

  private generateAddress(): string {
    const raw = typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID().replace(/-/g, '') : this.randomHex(48);
    return `chert_${raw.slice(0, 48)}`;
  }

  private randomHex(length: number): string {
    const bytes = new Uint8Array(Math.ceil(length / 2));
    if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
      crypto.getRandomValues(bytes);
    } else {
      for (let i = 0; i < bytes.length; i += 1) {
        bytes[i] = Math.floor(Math.random() * 256);
      }
    }
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('')
      .slice(0, length);
  }

  private randomInt(min: number, max: number): number {
    if (min >= max) {
      return min;
    }
    const range = max - min + 1;
    if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
      const buffer = new Uint32Array(1);
      crypto.getRandomValues(buffer);
      return min + (buffer[0] % range);
    }
    return min + Math.floor(Math.random() * range);
  }

  private async hashSecret(secret: string): Promise<string> {
    if (typeof crypto !== 'undefined' && crypto.subtle) {
      const encoded = new TextEncoder().encode(secret);
      const digest = await crypto.subtle.digest('SHA-256', encoded);
      return Array.from(new Uint8Array(digest))
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');
    }
    return this.simpleHash(secret);
  }

  private async computeDigest(input: string, algorithm: AlgorithmIdentifier): Promise<string> {
    if (typeof crypto !== 'undefined' && crypto.subtle) {
      const encoded = new TextEncoder().encode(input);
      const digest = await crypto.subtle.digest(algorithm, encoded);
      return Array.from(new Uint8Array(digest))
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');
    }
    return this.simpleHash(`${algorithm}:${input}`);
  }

  private simpleHash(value: string): string {
    let hash = 0;
    for (let i = 0; i < value.length; i += 1) {
      hash = (hash << 5) - hash + value.charCodeAt(i);
      hash |= 0;
    }
    return Math.abs(hash)
      .toString(16)
      .padStart(16, '0');
  }

  private formatTokenAmount(amount: string, decimals: number): string {
    const sanitized = amount.replace(/[^0-9]/g, '');
    if (!sanitized) {
      return '0';
    }

    const precision = Math.max(0, Math.min(decimals, 18));
    const padded = sanitized.padStart(precision + 1, '0');
    const whole = padded.slice(0, -precision) || '0';
    const fraction = padded.slice(-precision).replace(/0+$/, '');
    return fraction ? `${whole}.${fraction}` : whole;
  }

  /**
   * Format network errors with helpful messages
   */
  private formatNetworkError(error: unknown, url: string): string {
    if (error instanceof Error) {
      const message = error.message.toLowerCase();

      // Check for CORS error indicators
      if (message.includes('cors') ||
          message.includes('cross-origin') ||
          message.includes('blocked') ||
          (error.name === 'TypeError' && message.includes('failed to fetch'))) {
        return `CORS Error: Cannot connect to ${url}. The node must allow cross-origin requests from this domain, or use a CORS proxy.`;
      }

      // Network connectivity issues
      if (message.includes('network') ||
          message.includes('timeout') ||
          message.includes('econnrefused') ||
          message.includes('failed to fetch')) {
        return `Network Error: Unable to connect to ${url}. Please check if the node is running and accessible.`;
      }

      return `Request failed: ${error.message}`;
    }

    return `Unknown error connecting to ${url}`;
  }
}

export function provideWalletBackend(): Provider[] {
  return [
    {
      provide: WALLET_BACKEND,
      useExisting: WebWalletBackendService
    }
  ];
}
