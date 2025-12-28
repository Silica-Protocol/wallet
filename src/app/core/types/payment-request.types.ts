export interface PaymentRequestOptions {
  amountBaseUnits?: string | null;
  memo?: string | null;
  expiresAt?: string | null;
}

export interface PaymentRequest {
  token: string;
  address: string;
  amountBaseUnits: string | null;
  memo: string | null;
  createdAt: string;
  expiresAt: string | null;
  uri: string;
}
