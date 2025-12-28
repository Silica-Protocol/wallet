import { Injectable, inject } from '@angular/core';
import { argon2id } from 'hash-wasm';
import { RpcService, RpcError } from './rpc.service';

/**
 * Challenge returned by the server
 *
 * Uses Argon2id for ASIC-resistant proof-of-work.
 * Argon2 is memory-hard, making it impractical to build specialized
 * hardware (ASICs) to solve challenges faster than general CPUs.
 */
export interface TransactionChallenge {
  challenge_id: string;
  nonce: string;
  difficulty: number;       // Number of leading zero bits required
  memory_cost: number;      // Argon2 memory cost in KB (e.g., 1024 = 1MB)
  time_cost: number;        // Argon2 iterations
  expires_at: number;
  instructions: string;
}

/**
 * Solved challenge to submit with transactions
 */
export interface SolvedChallenge {
  challenge_id: string;
  solution: string;         // Hex-encoded counter that produces valid hash
  hash: string;             // Hex-encoded resulting Argon2 hash (for quick verification)
}

/**
 * Error thrown when challenge operations fail
 */
export class ChallengeError extends Error {
  constructor(
    message: string,
    public readonly code: 'FETCH_FAILED' | 'SOLVE_FAILED' | 'EXPIRED' | 'INVALID' | 'WASM_FAILED' = 'FETCH_FAILED'
  ) {
    super(message);
    this.name = 'ChallengeError';
  }
}

/**
 * Argon2 PoW configuration defaults
 * These are lightweight settings suitable for anti-spam (not password hashing)
 */
const DEFAULT_MEMORY_COST = 1024;  // 1 MB - enough to deter ASICs, fast for CPUs
const DEFAULT_TIME_COST = 2;       // 2 iterations
const DEFAULT_PARALLELISM = 1;     // Single-threaded for browser compatibility
const DEFAULT_HASH_LENGTH = 32;    // 256-bit output

/**
 * Service for handling transaction challenges with ASIC-resistant PoW
 *
 * This implements client-side Argon2id proof-of-work for anti-spam protection.
 * Argon2 is memory-hard, meaning:
 * - ASICs cannot efficiently parallelize it (need real RAM per instance)
 * - GPUs gain little advantage over CPUs
 * - Solution time is ~50-200ms on modern devices
 *
 * Flow:
 * 1. Call getChallenge() to get a challenge from the node
 * 2. Call solveChallenge() to compute the Argon2 PoW solution
 * 3. Include the solved challenge in transaction params
 */
@Injectable({ providedIn: 'root' })
export class ChallengeService {
  private readonly rpc = inject(RpcService);

  /**
   * Fetch a new challenge from the node
   */
  async getChallenge(nodeUrl: string): Promise<TransactionChallenge> {
    try {
      const response = await this.rpc.call<Partial<TransactionChallenge>>(
        nodeUrl,
        'get_transaction_challenge',
        {}
      );

      // Apply defaults for optional Argon2 parameters
      return {
        challenge_id: response.challenge_id!,
        nonce: response.nonce!,
        difficulty: response.difficulty ?? 16,
        memory_cost: response.memory_cost ?? DEFAULT_MEMORY_COST,
        time_cost: response.time_cost ?? DEFAULT_TIME_COST,
        expires_at: response.expires_at!,
        instructions: response.instructions ?? 'Solve Argon2id PoW challenge'
      };
    } catch (error) {
      if (error instanceof RpcError) {
        throw new ChallengeError(`Failed to get challenge: ${error.message}`, 'FETCH_FAILED');
      }
      throw new ChallengeError('Failed to get challenge from node', 'FETCH_FAILED');
    }
  }

  /**
   * Solve a challenge using Argon2id proof-of-work
   *
   * This finds a counter value such that:
   *   Argon2id(password=counter, salt=nonce, memory, time)
   * produces a hash with the required number of leading zero bits.
   *
   * Argon2 is ASIC-resistant because:
   * - Memory-hard: Each attempt needs ~1MB RAM
   * - Time-hard: Multiple iterations required
   * - Cannot be efficiently parallelized without proportional memory
   *
   * @param challenge The challenge to solve
   * @param maxAttempts Maximum number of Argon2 computations (default 10000)
   * @returns Solved challenge ready to submit
   */
  async solveChallenge(
    challenge: TransactionChallenge,
    maxAttempts: number = 10_000
  ): Promise<SolvedChallenge> {
    // Check if challenge has expired
    const now = Math.floor(Date.now() / 1000);
    if (challenge.expires_at < now) {
      throw new ChallengeError('Challenge has expired', 'EXPIRED');
    }

    // Decode nonce from hex to use as salt
    const salt = this.hexToBytes(challenge.nonce);
    if (salt.length !== 32) {
      throw new ChallengeError('Invalid nonce length', 'INVALID');
    }

    // Try to find a valid solution
    for (let counter = 0; counter < maxAttempts; counter++) {
      try {
        // Convert counter to 8-byte big-endian buffer (password)
        const counterBytes = new Uint8Array(8);
        const view = new DataView(counterBytes.buffer);
        view.setBigUint64(0, BigInt(counter), false); // big-endian

        // Compute Argon2id hash using hash-wasm
        const hashHex = await argon2id({
          password: counterBytes,
          salt: salt,
          memorySize: challenge.memory_cost,
          iterations: challenge.time_cost,
          parallelism: DEFAULT_PARALLELISM,
          hashLength: DEFAULT_HASH_LENGTH,
          outputType: 'hex'
        });

        // Convert hex to bytes for leading zeros check
        const hash = this.hexToBytes(hashHex);

        // Check if hash meets difficulty requirement
        if (this.hasLeadingZeros(hash, challenge.difficulty)) {
          return {
            challenge_id: challenge.challenge_id,
            solution: this.bytesToHex(counterBytes),
            hash: hashHex
          };
        }

        // Yield to UI every 10 attempts (Argon2 is slow, so less frequent)
        if (counter % 10 === 0 && counter > 0) {
          await this.yieldToUI();
        }
      } catch (error) {
        // Argon2 WASM error
        console.error('Argon2 computation failed:', error);
        throw new ChallengeError(
          `Argon2 computation failed: ${error instanceof Error ? error.message : 'Unknown error'}`,
          'WASM_FAILED'
        );
      }
    }

    throw new ChallengeError(
      `Failed to solve challenge after ${maxAttempts} attempts`,
      'SOLVE_FAILED'
    );
  }

  /**
   * Get a solved challenge ready for transaction submission
   *
   * This is a convenience method that fetches and solves in one call.
   */
  async getSolvedChallenge(nodeUrl: string): Promise<SolvedChallenge> {
    const challenge = await this.getChallenge(nodeUrl);
    return await this.solveChallenge(challenge);
  }

  /**
   * Check if a hash has the required number of leading zero bits
   */
  private hasLeadingZeros(hash: Uint8Array, requiredBits: number): boolean {
    const fullBytes = Math.floor(requiredBits / 8);
    const remainingBits = requiredBits % 8;

    // Check full zero bytes
    for (let i = 0; i < fullBytes; i++) {
      if (hash[i] !== 0) {
        return false;
      }
    }

    // Check remaining bits in the next byte
    if (remainingBits > 0 && fullBytes < hash.length) {
      const mask = 0xFF << (8 - remainingBits);
      if ((hash[fullBytes] & mask) !== 0) {
        return false;
      }
    }

    return true;
  }

  /**
   * Convert hex string to Uint8Array
   */
  private hexToBytes(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
    }
    return bytes;
  }

  /**
   * Convert Uint8Array to hex string
   */
  private bytesToHex(bytes: Uint8Array): string {
    return Array.from(bytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }

  /**
   * Yield control back to the UI thread
   */
  private yieldToUI(): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, 0));
  }
}
