import { Injectable, inject } from '@angular/core';
import { argon2id } from 'hash-wasm';
import { RpcService, RpcError } from './rpc.service';
import { WasmService } from './wasm.service';

/**
 * Network Utility Work (NUW) Challenge Types
 *
 * Instead of wasteful proof-of-work, users can contribute useful computation
 * to the network in exchange for fee discounts.
 */
export enum NuwChallengeType {
  /** Standard Argon2 PoW - wasteful but always available */
  ARGON2_POW = 'argon2_pow',

  /** Verify batch of pending transaction signatures */
  SIGNATURE_BATCH = 'signature_batch',

  /** Verify a ZK proof (Halo2) */
  ZK_VERIFY = 'zk_verify',

  /** Assist with PQ key derivation/validation */
  PQ_ASSIST = 'pq_assist',

  /** Validate Merkle state proofs */
  MERKLE_VERIFY = 'merkle_verify',
}

/**
 * Fee discount tiers based on work usefulness
 */
export const NUW_FEE_DISCOUNTS: Record<NuwChallengeType, number> = {
  [NuwChallengeType.ARGON2_POW]: 0,        // No discount - wasteful work
  [NuwChallengeType.SIGNATURE_BATCH]: 25,   // 25% discount
  [NuwChallengeType.ZK_VERIFY]: 50,         // 50% discount - most valuable
  [NuwChallengeType.PQ_ASSIST]: 40,         // 40% discount
  [NuwChallengeType.MERKLE_VERIFY]: 30,     // 30% discount
};

/**
 * Signature to verify in a batch
 */
export interface PendingSignature {
  /** Transaction ID */
  tx_id: string;
  /** Message bytes (hex) */
  message: string;
  /** Signature bytes (hex) */
  signature: string;
  /** Public key (hex) */
  public_key: string;
  /** Algorithm: 'ed25519' or 'dilithium2' */
  algorithm: 'ed25519' | 'dilithium2';
}

/**
 * ZK Proof to verify
 */
export interface ZkProofTask {
  /** Proof ID */
  proof_id: string;
  /** Proof type (e.g., 'halo2', 'groth16') */
  proof_type: string;
  /** Serialized proof (hex) */
  proof: string;
  /** Public inputs (hex) */
  public_inputs: string[];
  /** Verification key (hex) */
  verification_key: string;
}

/**
 * Merkle proof to verify
 */
export interface MerkleProofTask {
  /** Root hash (hex) */
  root: string;
  /** Leaf hash (hex) */
  leaf: string;
  /** Proof path (hex-encoded hashes) */
  proof: string[];
  /** Leaf index */
  index: number;
}

/**
 * NUW Challenge from the server
 */
export interface NuwChallenge {
  /** Unique challenge ID */
  challenge_id: string;
  /** Type of work requested */
  challenge_type: NuwChallengeType;
  /** Fee discount percentage if completed */
  fee_discount_percent: number;
  /** Expiry timestamp */
  expires_at: number;

  // Type-specific data (only one will be present)
  argon2_params?: {
    nonce: string;
    difficulty: number;
    memory_cost: number;
    time_cost: number;
  };
  signature_batch?: PendingSignature[];
  zk_proof?: ZkProofTask;
  merkle_proofs?: MerkleProofTask[];
}

/**
 * Solved NUW challenge
 */
export interface NuwSolution {
  challenge_id: string;
  challenge_type: NuwChallengeType;

  // Type-specific solutions
  argon2_solution?: string;
  signature_results?: { tx_id: string; valid: boolean }[];
  zk_result?: { proof_id: string; valid: boolean };
  merkle_results?: { index: number; valid: boolean }[];
}

/**
 * Error during NUW challenge processing
 */
export class NuwChallengeError extends Error {
  constructor(
    message: string,
    public readonly code:
      | 'FETCH_FAILED'
      | 'SOLVE_FAILED'
      | 'EXPIRED'
      | 'INVALID'
      | 'WASM_UNAVAILABLE'
      | 'UNSUPPORTED_TYPE' = 'SOLVE_FAILED'
  ) {
    super(message);
    this.name = 'NuwChallengeError';
  }
}

/**
 * Network Utility Work Challenge Service
 *
 * This service allows users to perform useful network computations
 * instead of wasteful proof-of-work, earning fee discounts.
 *
 * Supported work types:
 * - Signature batch verification: Verify pending transaction signatures
 * - ZK proof verification: Verify Halo2/Groth16 proofs
 * - Merkle proof verification: Validate state proofs
 * - PQ key assistance: Help with post-quantum key operations
 *
 * Example usage:
 * ```typescript
 * // Get an available challenge (server picks best type)
 * const challenge = await nuwService.getChallenge(nodeUrl);
 *
 * // Or request a specific type for max discount
 * const zkChallenge = await nuwService.getChallenge(nodeUrl, NuwChallengeType.ZK_VERIFY);
 *
 * // Solve it
 * const solution = await nuwService.solveChallenge(challenge);
 *
 * // Include with transaction for fee discount
 * await txService.sendTransaction({ ..., nuw_solution: solution });
 * ```
 */
@Injectable({ providedIn: 'root' })
export class NuwChallengeService {
  private readonly rpc = inject(RpcService);
  private readonly wasmService = inject(WasmService);

  /**
   * Get an available NUW challenge from the node
   *
   * @param nodeUrl Node RPC endpoint
   * @param preferredType Optional: request a specific challenge type
   * @returns Challenge to solve
   */
  async getChallenge(
    nodeUrl: string,
    preferredType?: NuwChallengeType
  ): Promise<NuwChallenge> {
    try {
      return await this.rpc.call<NuwChallenge>(
        nodeUrl,
        'get_nuw_challenge',
        { preferred_type: preferredType }
      );
    } catch (error) {
      if (error instanceof RpcError) {
        throw new NuwChallengeError(
          `Failed to get NUW challenge: ${error.message}`,
          'FETCH_FAILED'
        );
      }
      throw new NuwChallengeError('Failed to get NUW challenge', 'FETCH_FAILED');
    }
  }

  /**
   * Solve a NUW challenge
   *
   * Automatically dispatches to the correct solver based on challenge type.
   */
  async solveChallenge(challenge: NuwChallenge): Promise<NuwSolution> {
    // Check expiry
    const now = Math.floor(Date.now() / 1000);
    if (challenge.expires_at < now) {
      throw new NuwChallengeError('Challenge has expired', 'EXPIRED');
    }

    switch (challenge.challenge_type) {
      case NuwChallengeType.ARGON2_POW:
        return this.solveArgon2(challenge);

      case NuwChallengeType.SIGNATURE_BATCH:
        return this.solveSignatureBatch(challenge);

      case NuwChallengeType.ZK_VERIFY:
        return this.solveZkVerify(challenge);

      case NuwChallengeType.MERKLE_VERIFY:
        return this.solveMerkleVerify(challenge);

      case NuwChallengeType.PQ_ASSIST:
        // Fall back to Argon2 if PQ not implemented yet
        return this.solveArgon2(challenge);

      default:
        throw new NuwChallengeError(
          `Unsupported challenge type: ${challenge.challenge_type}`,
          'UNSUPPORTED_TYPE'
        );
    }
  }

  /**
   * Get a solved challenge ready for transaction
   * Convenience method that fetches and solves in one call.
   */
  async getSolvedChallenge(
    nodeUrl: string,
    preferredType?: NuwChallengeType
  ): Promise<NuwSolution> {
    const challenge = await this.getChallenge(nodeUrl, preferredType);
    return this.solveChallenge(challenge);
  }

  /**
   * Check which challenge types are available based on current WASM capabilities
   */
  async getAvailableTypes(): Promise<NuwChallengeType[]> {
    const types: NuwChallengeType[] = [NuwChallengeType.ARGON2_POW];

    // Check if WASM is available for signature verification
    try {
      if (this.wasmService.isReady()) {
        types.push(NuwChallengeType.SIGNATURE_BATCH);
        types.push(NuwChallengeType.MERKLE_VERIFY);
      }
    } catch {
      // WASM not available, only Argon2 supported
    }

    // ZK verification requires additional WASM modules (future)
    // types.push(NuwChallengeType.ZK_VERIFY);

    return types;
  }

  // ==================== Solvers ====================

  /**
   * Solve Argon2 PoW challenge (fallback/default)
   */
  private async solveArgon2(challenge: NuwChallenge): Promise<NuwSolution> {
    if (!challenge.argon2_params) {
      throw new NuwChallengeError('Missing Argon2 parameters', 'INVALID');
    }

    const { nonce, difficulty, memory_cost, time_cost } = challenge.argon2_params;

    const salt = this.hexToBytes(nonce);
    const maxAttempts = 10_000;

    for (let counter = 0; counter < maxAttempts; counter++) {
      const counterBytes = new Uint8Array(8);
      new DataView(counterBytes.buffer).setBigUint64(0, BigInt(counter), false);

      try {
        // Use hash-wasm argon2id
        const hashHex = await argon2id({
          password: counterBytes,
          salt: salt,
          parallelism: 1,
          iterations: time_cost,
          memorySize: memory_cost,
          hashLength: 32,
          outputType: 'hex',
        });

        const hash = this.hexToBytes(hashHex);

        if (this.hasLeadingZeros(hash, difficulty)) {
          return {
            challenge_id: challenge.challenge_id,
            challenge_type: NuwChallengeType.ARGON2_POW,
            argon2_solution: this.bytesToHex(counterBytes),
          };
        }
      } catch (error) {
        console.error('Argon2 error:', error);
        throw new NuwChallengeError('Argon2 computation failed', 'SOLVE_FAILED');
      }

      // Yield every 10 attempts
      if (counter % 10 === 0) {
        await this.yieldToUI();
      }
    }

    throw new NuwChallengeError('Failed to solve Argon2 challenge', 'SOLVE_FAILED');
  }

  /**
   * Verify a batch of pending transaction signatures
   *
   * This is genuinely useful work - reduces validator load!
   */
  private async solveSignatureBatch(challenge: NuwChallenge): Promise<NuwSolution> {
    if (!challenge.signature_batch || challenge.signature_batch.length === 0) {
      throw new NuwChallengeError('No signatures to verify', 'INVALID');
    }

    const results: { tx_id: string; valid: boolean }[] = [];

    for (const sig of challenge.signature_batch) {
      try {
        const message = this.hexToBytes(sig.message);
        const signature = this.hexToBytes(sig.signature);
        const publicKey = this.hexToBytes(sig.public_key);

        // Use WASM service to verify signature
        const valid = await this.verifySignature(
          message,
          signature,
          publicKey,
          sig.algorithm
        );

        results.push({ tx_id: sig.tx_id, valid });
      } catch (error) {
        // If verification fails, mark as invalid
        console.warn(`Signature verification failed for ${sig.tx_id}:`, error);
        results.push({ tx_id: sig.tx_id, valid: false });
      }

      // Yield after each verification
      await this.yieldToUI();
    }

    return {
      challenge_id: challenge.challenge_id,
      challenge_type: NuwChallengeType.SIGNATURE_BATCH,
      signature_results: results,
    };
  }

  /**
   * Verify a ZK proof
   *
   * This provides significant value to the network!
   */
  private async solveZkVerify(challenge: NuwChallenge): Promise<NuwSolution> {
    if (!challenge.zk_proof) {
      throw new NuwChallengeError('No ZK proof to verify', 'INVALID');
    }

    // For now, this is a placeholder
    // In production, you'd use snarkjs or halo2-wasm
    //
    // Example with snarkjs:
    // const snarkjs = await import('snarkjs');
    // const valid = await snarkjs.groth16.verify(vk, publicInputs, proof);

    console.warn('ZK verification not yet implemented, returning placeholder');

    // TODO: Implement actual ZK verification
    // For now, return success to allow testing the flow
    return {
      challenge_id: challenge.challenge_id,
      challenge_type: NuwChallengeType.ZK_VERIFY,
      zk_result: {
        proof_id: challenge.zk_proof.proof_id,
        valid: true, // Placeholder - implement real verification
      },
    };
  }

  /**
   * Verify Merkle proofs
   */
  private async solveMerkleVerify(challenge: NuwChallenge): Promise<NuwSolution> {
    if (!challenge.merkle_proofs || challenge.merkle_proofs.length === 0) {
      throw new NuwChallengeError('No Merkle proofs to verify', 'INVALID');
    }

    const results: { index: number; valid: boolean }[] = [];

    for (const proof of challenge.merkle_proofs) {
      const valid = await this.verifyMerkleProof(
        proof.root,
        proof.leaf,
        proof.proof,
        proof.index
      );
      results.push({ index: proof.index, valid });
    }

    return {
      challenge_id: challenge.challenge_id,
      challenge_type: NuwChallengeType.MERKLE_VERIFY,
      merkle_results: results,
    };
  }

  // ==================== Helpers ====================

  /**
   * Verify a signature using the WASM crypto module
   *
   * Note: For now, since the WASM module doesn't expose raw verify functions,
   * we use WebCrypto API for Ed25519 and mark Dilithium as requiring WASM.
   */
  private async verifySignature(
    message: Uint8Array,
    signature: Uint8Array,
    publicKey: Uint8Array,
    algorithm: 'ed25519' | 'dilithium2'
  ): Promise<boolean> {
    try {
      if (algorithm === 'ed25519') {
        // Use WebCrypto API for Ed25519 verification
        // Note: Ed25519 support in WebCrypto is relatively recent
        try {
          // Copy to ArrayBuffer to ensure compatibility
          const keyData = new Uint8Array(publicKey).buffer as ArrayBuffer;
          const sigData = new Uint8Array(signature).buffer as ArrayBuffer;
          const msgData = new Uint8Array(message).buffer as ArrayBuffer;

          const key = await crypto.subtle.importKey(
            'raw',
            keyData,
            { name: 'Ed25519' },
            false,
            ['verify']
          );
          return await crypto.subtle.verify('Ed25519', key, sigData, msgData);
        } catch {
          // Fallback: If Ed25519 not supported, we need WASM
          console.warn('Ed25519 not available in WebCrypto, requires WASM');
          return false;
        }
      } else {
        // Dilithium requires WASM - not yet implemented
        console.warn('Dilithium verification requires WASM implementation');
        return false;
      }
    } catch (error) {
      console.error('Signature verification error:', error);
      return false;
    }
  }

  /**
   * Verify a Merkle proof using SHA-256
   */
  private async verifyMerkleProof(
    rootHex: string,
    leafHex: string,
    proofHex: string[],
    index: number
  ): Promise<boolean> {
    const root = this.hexToBytes(rootHex);
    let current = this.hexToBytes(leafHex);

    for (let i = 0; i < proofHex.length; i++) {
      const sibling = this.hexToBytes(proofHex[i]);
      const isRight = (index >> i) & 1;

      // Concatenate in correct order
      const combined = new Uint8Array(64);
      if (isRight) {
        combined.set(sibling, 0);
        combined.set(current, 32);
      } else {
        combined.set(current, 0);
        combined.set(sibling, 32);
      }

      // Hash the pair
      const hashBuffer = await crypto.subtle.digest('SHA-256', combined);
      current = new Uint8Array(hashBuffer);
    }

    // Compare with root
    if (current.length !== root.length) return false;
    for (let i = 0; i < current.length; i++) {
      if (current[i] !== root[i]) return false;
    }
    return true;
  }

  /**
   * Check if hash has required leading zero bits
   */
  private hasLeadingZeros(hash: Uint8Array, requiredBits: number): boolean {
    const fullBytes = Math.floor(requiredBits / 8);
    const remainingBits = requiredBits % 8;

    for (let i = 0; i < fullBytes; i++) {
      if (hash[i] !== 0) return false;
    }

    if (remainingBits > 0 && fullBytes < hash.length) {
      const mask = 0xff << (8 - remainingBits);
      if ((hash[fullBytes] & mask) !== 0) return false;
    }

    return true;
  }

  private hexToBytes(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
    }
    return bytes;
  }

  private bytesToHex(bytes: Uint8Array): string {
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  }

  private yieldToUI(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
  }
}
