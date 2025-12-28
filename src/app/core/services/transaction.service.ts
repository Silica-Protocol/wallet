import { Injectable, inject } from '@angular/core';
import { RpcService, RpcError } from './rpc.service';
import { ChallengeService, SolvedChallenge } from './challenge.service';

export interface FeeEstimate {
  estimated_fee: string;
  priority_fees: {
    low: string;
    medium: string;
    high: string;
  };
  network_congestion: 'low' | 'medium' | 'high';
  estimated_confirmation_time: number;
}

export interface FeeEstimateRequest {
  from_address: string;
  to_address: string;
  amount: string;
  priority?: 'low' | 'medium' | 'high';
}

export interface SendTransactionRequest {
  from_address: string;
  to_address: string;
  amount: string;
  fee: string;
  memo?: string;
  signature: string;
  public_key: string;
  nonce: number;
}

export interface SendTransactionResponse {
  transaction_id: string;
  status: 'pending' | 'submitted';
  timestamp: string;
}

export interface TransactionStatus {
  transaction_id: string;
  status: 'pending' | 'confirmed' | 'failed';
  block_height?: number;
  confirmations?: number;
  error?: string;
}

const DEFAULT_FEES: FeeEstimate = {
  estimated_fee: '1000000000',
  priority_fees: {
    low: '500000000',     // 0.5 CHERT
    medium: '1000000000', // 1.0 CHERT
    high: '2000000000'    // 2.0 CHERT
  },
  network_congestion: 'medium',
  estimated_confirmation_time: 2
};

@Injectable({ providedIn: 'root' })
export class TransactionService {
  private readonly rpc = inject(RpcService);
  private readonly challenge = inject(ChallengeService);

  /**
   * Estimate transaction fee
   */
  async estimateFee(
    nodeUrl: string,
    request: FeeEstimateRequest
  ): Promise<FeeEstimate> {
    try {
      const result = await this.rpc.call<FeeEstimate>(
        nodeUrl,
        'estimate_fee',
        {
          from_address: request.from_address,
          to_address: request.to_address,
          amount: request.amount
        }
      );

      return result;
    } catch (error) {
      // Log the error but return defaults so UI can still function
      if (error instanceof RpcError) {
        console.warn(`Fee estimation failed (${error.code}): ${error.message}`);
        if (error.isCorsError) {
          console.warn('CORS error detected - fee estimation unavailable from this domain');
        }
      } else {
        console.warn('Fee estimation failed:', error);
      }

      // Return default fees when node is unavailable
      return DEFAULT_FEES;
    }
  }

  /**
   * Get network congestion level
   */
  async getNetworkCongestion(nodeUrl: string): Promise<'low' | 'medium' | 'high'> {
    try {
      const result = await this.rpc.call<{ congestion: 'low' | 'medium' | 'high' }>(
        nodeUrl,
        'get_network_congestion',
        {}
      );
      return result.congestion;
    } catch {
      return 'medium';
    }
  }

  /**
   * Send a signed transaction
   *
   * This method automatically handles the challenge-response flow:
   * 1. Fetches a challenge from the node
   * 2. Solves the mini proof-of-work
   * 3. Submits the transaction with the solved challenge
   */
  async sendTransaction(
    nodeUrl: string,
    request: SendTransactionRequest
  ): Promise<SendTransactionResponse> {
    try {
      // Get and solve challenge for anti-spam protection
      console.log('[Transaction] Getting challenge from node...');
      const solvedChallenge = await this.challenge.getSolvedChallenge(nodeUrl);
      console.log('[Transaction] Challenge solved, submitting transaction...');

      const result = await this.rpc.call<SendTransactionResponse>(
        nodeUrl,
        'send_transaction',
        {
          from_address: request.from_address,
          to_address: request.to_address,
          amount: request.amount,
          fee: request.fee,
          memo: request.memo,
          signature: request.signature,
          public_key: request.public_key,
          nonce: request.nonce,
          // Include solved challenge
          challenge: solvedChallenge
        }
      );

      return result;
    } catch (error) {
      if (error instanceof RpcError) {
        if (error.isCorsError) {
          throw new Error('Cannot send transaction: CORS policy blocks requests to the node. Please use the desktop wallet or configure a proxy.');
        }
        if (error.isNetworkError) {
          throw new Error('Cannot send transaction: Unable to connect to the node. Please check your network connection.');
        }
        if (error.requiresChallenge) {
          throw new Error('Transaction requires proof-of-work challenge. Please try again.');
        }
        throw new Error(`Transaction failed: ${error.message}`);
      }
      throw error;
    }
  }

  /**
   * Get transaction status
   */
  async getTransactionStatus(
    nodeUrl: string,
    transactionId: string
  ): Promise<TransactionStatus> {
    try {
      return await this.rpc.call<TransactionStatus>(
        nodeUrl,
        'get_transaction_status',
        { transaction_id: transactionId }
      );
    } catch (error) {
      if (error instanceof RpcError) {
        return {
          transaction_id: transactionId,
          status: 'pending',
          error: error.message
        };
      }
      throw error;
    }
  }

  /**
   * Get account nonce for transaction signing
   */
  async getAccountNonce(nodeUrl: string, address: string): Promise<number> {
    try {
      const result = await this.rpc.call<{ nonce: number }>(
        nodeUrl,
        'get_account_nonce',
        { address }
      );
      return result.nonce;
    } catch {
      // If we can't get the nonce, return 0 and let the transaction fail
      // This is better than blocking the UI
      console.warn('Could not fetch account nonce, using 0');
      return 0;
    }
  }

  /**
   * Create unsigned transaction data for signing
   */
  createTransactionData(
    fromAddress: string,
    toAddress: string,
    amount: string,
    fee: string,
    nonce: number,
    memo?: string
  ): string {
    // Create deterministic transaction data for signing
    const data = {
      from: fromAddress,
      to: toAddress,
      amount,
      fee,
      nonce,
      memo: memo || '',
      timestamp: Math.floor(Date.now() / 1000)
    };

    return JSON.stringify(data);
  }
}
