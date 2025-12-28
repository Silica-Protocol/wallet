/* tslint:disable */
/* eslint-disable */
export function test_wasm(): string;
/**
 * Initialize the WASM module
 */
export function init_wasm(): object;
export function main(): void;
export function greet(name: string): string;
export function add(a: number, b: number): number;
export function multiply(a: number, b: number): number;
export function get_version(): string;
export function get_build_info(): object;
export function async_greet(name: string): string;
export function test_performance(): number;
export class ShieldingResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  readonly proof: Uint8Array;
  readonly commitment: Uint8Array;
  readonly nullifier: Uint8Array;
}
export class UnshieldingResult {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  readonly proof: Uint8Array;
  readonly nullifier: Uint8Array;
}
export class ZkBridgeContext {
  free(): void;
  [Symbol.dispose](): void;
  constructor(k: number);
  prove_shielding(public_utxo_bytes: Uint8Array, amount: bigint, private_key_bytes: Uint8Array, blinding_bytes: Uint8Array): ShieldingResult;
  prove_unshielding(private_utxo_commitment_bytes: Uint8Array, amount: bigint, private_key_bytes: Uint8Array, public_recipient_bytes: Uint8Array, origin_public_utxo_id_bytes: Uint8Array): UnshieldingResult;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly test_wasm: () => [number, number];
  readonly init_wasm: () => [number, number, number];
  readonly main: () => void;
  readonly __wbg_shieldingresult_free: (a: number, b: number) => void;
  readonly shieldingresult_proof: (a: number) => [number, number];
  readonly shieldingresult_commitment: (a: number) => [number, number];
  readonly shieldingresult_nullifier: (a: number) => [number, number];
  readonly __wbg_unshieldingresult_free: (a: number, b: number) => void;
  readonly unshieldingresult_proof: (a: number) => [number, number];
  readonly unshieldingresult_nullifier: (a: number) => [number, number];
  readonly __wbg_zkbridgecontext_free: (a: number, b: number) => void;
  readonly zkbridgecontext_new: (a: number) => [number, number, number];
  readonly zkbridgecontext_prove_shielding: (a: number, b: number, c: number, d: bigint, e: number, f: number, g: number, h: number) => [number, number, number];
  readonly zkbridgecontext_prove_unshielding: (a: number, b: number, c: number, d: bigint, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number];
  readonly greet: (a: number, b: number) => [number, number];
  readonly add: (a: number, b: number) => number;
  readonly multiply: (a: number, b: number) => number;
  readonly get_version: () => [number, number];
  readonly get_build_info: () => [number, number, number];
  readonly async_greet: (a: number, b: number) => [number, number];
  readonly test_performance: () => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
