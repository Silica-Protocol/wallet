import { Injectable } from '@angular/core';
import { Observable, from, map } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class ZkBridgeService {
  private zkBridgeContext: any = null;

  constructor() { }

  async initialize(): Promise<void> {
    if (this.zkBridgeContext) return;

    try {
      const wasm = await import('../../../assets/wasm/chert_wallet_wasm');
      this.zkBridgeContext = new wasm.ZkBridgeContext(11); // k=11 for bridge circuits
      console.log('ZK Bridge context initialized');
    } catch (error) {
      console.error('Failed to initialize ZK Bridge:', error);
      throw error;
    }
  }

  proveShielding(
    publicUtxo: Uint8Array,
    amount: bigint,
    privateKey: Uint8Array,
    blinding: Uint8Array
  ): Observable<{ proof: Uint8Array; commitment: Uint8Array; nullifier: Uint8Array }> {
    return from(this.initialize()).pipe(
      map(() => {
        const result = this.zkBridgeContext.prove_shielding(publicUtxo, amount, privateKey, blinding);
        return {
          proof: result.proof,
          commitment: result.commitment,
          nullifier: result.nullifier
        };
      })
    );
  }

  proveUnshielding(
    privateUtxoCommitment: Uint8Array,
    amount: bigint,
    privateKey: Uint8Array,
    publicRecipient: Uint8Array,
    originPublicUtxoId: Uint8Array
  ): Observable<{ proof: Uint8Array; nullifier: Uint8Array }> {
    return from(this.initialize()).pipe(
      map(() => {
        const result = this.zkBridgeContext.prove_unshielding(
          privateUtxoCommitment,
          amount,
          privateKey,
          publicRecipient,
          originPublicUtxoId
        );
        return {
          proof: result.proof,
          nullifier: result.nullifier
        };
      })
    );
  }

  // Helper to convert hex string to Uint8Array
  hexToBytes(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
    }
    return bytes;
  }

  // Helper to convert Uint8Array to hex string
  bytesToHex(bytes: Uint8Array): string {
    return Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
  }
}