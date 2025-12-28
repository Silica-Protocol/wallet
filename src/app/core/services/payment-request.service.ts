import { Injectable } from '@angular/core';
import { PaymentRequest, PaymentRequestOptions } from '../types/payment-request.types';

const HEX_ALPHABET = '0123456789abcdef';

@Injectable({ providedIn: 'root' })
export class PaymentRequestService {
  supportsNfc(): boolean {
    return typeof window !== 'undefined' && 'NDEFReader' in window;
  }

  async generate(address: string, options: PaymentRequestOptions): Promise<PaymentRequest> {
    if (!address.trim()) {
      throw new Error('Wallet address is required to create a payment request');
    }

    if (typeof window === 'undefined' || !window.crypto?.getRandomValues) {
      throw new Error('Secure random generator is unavailable in this environment');
    }

    const token = this.createToken();
    const createdAt = new Date().toISOString();
    const expiresAt = options.expiresAt ?? null;
    const amountBaseUnits = options.amountBaseUnits ?? null;
    const memo = options.memo?.trim() ? options.memo.trim() : null;

    const params = new URLSearchParams();
    params.set('request_token', token);

    if (amountBaseUnits) {
      params.set('amount', amountBaseUnits);
    }

    if (memo) {
      params.set('memo', memo);
    }

    if (expiresAt) {
      params.set('expires', expiresAt);
    }

    const uri = params.toString() ? `chert:${address}?${params.toString()}` : `chert:${address}`;

    return {
      token,
      address,
      amountBaseUnits,
      memo,
      createdAt,
      expiresAt,
      uri
    };
  }

  async writeToNfc(request: PaymentRequest): Promise<void> {
    if (!this.supportsNfc()) {
      throw new Error('Web NFC is not supported on this device');
    }

    const ndef = new (window as any).NDEFReader();
    await ndef.write({
      records: [
        {
          recordType: 'url',
          data: request.uri
        },
        {
          recordType: 'text',
          data: JSON.stringify({
            token: request.token,
            amount: request.amountBaseUnits,
            memo: request.memo,
            createdAt: request.createdAt,
            expiresAt: request.expiresAt
          })
        }
      ]
    });
  }

  private createToken(): string {
    const buffer = new Uint8Array(32);
    window.crypto.getRandomValues(buffer);

    let token = '';
    for (let i = 0; i < buffer.length; i++) {
      const byte = buffer[i];
      token += HEX_ALPHABET[(byte >> 4) & 0x0f];
      token += HEX_ALPHABET[byte & 0x0f];
    }
    return token;
  }
}
