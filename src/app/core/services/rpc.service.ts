import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpHeaders, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom, timeout, catchError } from 'rxjs';

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  method: string;
  params: Record<string, unknown>;
  id: number;
}

export interface JsonRpcResponse<T> {
  jsonrpc: string;
  result?: T;
  error?: {
    code: number;
    message: string;
    data?: {
      requires_challenge?: boolean;
      hint?: string;
    };
  };
  id: number;
}

export class RpcError extends Error {
  constructor(
    message: string,
    public readonly code: number = -1,
    public readonly isNetworkError: boolean = false,
    public readonly isCorsError: boolean = false,
    public readonly requiresChallenge: boolean = false
  ) {
    super(message);
    this.name = 'RpcError';
  }
}

/**
 * Methods that are public (don't require authentication or challenge)
 */
const PUBLIC_METHODS = new Set([
  // Balance and account info
  'get_balance', 'eth_getBalance', 'getBalance',
  'get_transaction', 'eth_getTransactionByHash', 'getTransaction',
  'get_transaction_history', 'getTransactionHistory', 'account_getTransactionHistory',

  // Block and chain info
  'get_blocks', 'eth_getBlockByNumber', 'eth_getBlockByHash', 'getBlocks',
  'get_block_number', 'eth_blockNumber', 'getBlockNumber',
  'get_gas_price', 'eth_gasPrice', 'getGasPrice',
  'get_chain_id', 'eth_chainId', 'getChainId',
  'get_network_id', 'net_version', 'getNetworkId',

  // Validator/staking read-only queries
  'staking_get_validators', 'staking_getValidators', 'stakingGetValidators',
  'staking_get_user_delegations', 'staking_getUserDelegations', 'stakingGetUserDelegations',
  'get_user_delegations', 'getUserDelegations',
  'staking_get_rewards', 'staking_getRewards', 'stakingGetRewards',
  'get_staking_rewards', 'getStakingRewards',
  'staking_get_lockbox_records', 'staking_getLockboxRecords', 'stakingGetLockboxRecords',
  'staking_get_auto_stake_status', 'staking_getAutoStakeStatus', 'stakingGetAutoStakeStatus',
  'get_validator_info', 'getValidatorInfo', 'validator_info',

  // Governance read-only queries
  'governance_list_proposals', 'governance_listProposals', 'governanceListProposals',
  'governance_get_proposal', 'governance_getProposal', 'governanceGetProposal',
  'governance_get_proposal_votes', 'governance_getProposalVotes', 'governanceGetProposalVotes',
  'governance_get_voting_power', 'governance_getVotingPower', 'governanceGetVotingPower',
  'governance_get_delegations', 'governance_getDelegations', 'governanceGetDelegations',

  // Privacy read-only
  'privacy_is_nullifier_spent', 'privacyIsNullifierSpent',
  'privacy_get_private_commitments', 'privacyGetPrivateCommitments',
  'privacy_get_bridge_transaction', 'privacyGetBridgeTransaction',

  // Challenge endpoint itself
  'get_transaction_challenge', 'getTransactionChallenge',

  // Health check
  'health',
]);

@Injectable({ providedIn: 'root' })
export class RpcService {
  private readonly http = inject(HttpClient);
  private requestIdCounter = 0;
  private readonly defaultTimeout = 30000; // 30 seconds

  /**
   * Check if a method is public (doesn't require challenge)
   */
  isPublicMethod(method: string): boolean {
    return PUBLIC_METHODS.has(method);
  }

  /**
   * Make a JSON-RPC call to the specified endpoint
   */
  async call<T>(
    nodeUrl: string,
    method: string,
    params: Record<string, unknown> = {},
    timeoutMs: number = this.defaultTimeout
  ): Promise<T> {
    const rpcUrl = nodeUrl.endsWith('/jsonrpc') ? nodeUrl : `${nodeUrl}/jsonrpc`;

    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id: ++this.requestIdCounter
    };

    const headers = new HttpHeaders({
      'Content-Type': 'application/json',
      'Accept': 'application/json'
    });

    try {
      const response = await firstValueFrom(
        this.http.post<JsonRpcResponse<T>>(rpcUrl, request, { headers }).pipe(
          timeout(timeoutMs),
          catchError((error: HttpErrorResponse) => {
            throw this.handleHttpError(error, rpcUrl);
          })
        )
      );

      if (!response) {
        throw new RpcError('Empty response from server');
      }

      if (response.error) {
        // Check if error indicates challenge is required
        const requiresChallenge = response.error.data?.requires_challenge === true;
        throw new RpcError(
          response.error.message,
          response.error.code,
          false,
          false,
          requiresChallenge
        );
      }

      if (response.result === undefined) {
        throw new RpcError('Response missing result field');
      }

      return response.result;
    } catch (error) {
      if (error instanceof RpcError) {
        throw error;
      }
      throw new RpcError(
        error instanceof Error ? error.message : 'Unknown RPC error',
        -1,
        true
      );
    }
  }

  private handleHttpError(error: HttpErrorResponse, url: string): RpcError {
    // Check for CORS error
    if (error.status === 0) {
      // Status 0 typically indicates a network/CORS error
      const isCors = error.message?.includes('CORS') ||
                     error.message?.includes('cross-origin') ||
                     !navigator.onLine === false; // If online but status 0, likely CORS

      if (isCors) {
        return new RpcError(
          `CORS error: Unable to connect to ${url}. The server may not allow cross-origin requests from this domain.`,
          0,
          true,
          true
        );
      }

      return new RpcError(
        `Network error: Unable to connect to ${url}. Please check if the node is running.`,
        0,
        true,
        false
      );
    }

    // HTTP error codes
    if (error.status >= 400 && error.status < 500) {
      return new RpcError(
        `Client error (${error.status}): ${error.statusText || 'Bad request'}`,
        error.status
      );
    }

    if (error.status >= 500) {
      return new RpcError(
        `Server error (${error.status}): ${error.statusText || 'Internal server error'}`,
        error.status
      );
    }

    return new RpcError(
      `HTTP error (${error.status}): ${error.statusText || 'Unknown error'}`,
      error.status
    );
  }

  /**
   * Check if a node is reachable
   */
  async checkNodeHealth(nodeUrl: string): Promise<{ healthy: boolean; latencyMs: number; error?: string }> {
    const startTime = performance.now();

    try {
      await this.call<{ status: string }>(nodeUrl, 'health', {}, 5000);
      return {
        healthy: true,
        latencyMs: Math.round(performance.now() - startTime)
      };
    } catch (error) {
      return {
        healthy: false,
        latencyMs: Math.round(performance.now() - startTime),
        error: error instanceof RpcError ? error.message : 'Unknown error'
      };
    }
  }
}
