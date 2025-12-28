# Wallet Rebuild Plan

## Overview
A previous workspace reset removed in-progress fixes that were coordinating the wallet’s Tauri backend, WASM helpers, and Angular frontend. This document lists every feature area that must be rebuilt so work can resume without guesswork.

## Scope Summary
- **Rust (Tauri) runtime** – Security configuration bootstrap, biometric/push/passkey command wiring, blockchain address parsing, transaction accounting.
- **WASM support modules** – Config plumbing, balance subscriptions, cryptographic utilities, chart renderer animation lifecycle.
- **Angular services & UI** – Global error handling, logging backend integration, security configuration overrides, wallet backend HTTP client, governance settings and voting screens.

## Detailed Rebuild Checklist

### 1. Tauri Runtime (`wallet/src-tauri`)
- Reintroduce `security` module wiring in `main.rs` so the environment-specific `SecurityConfig` loads before commands register.
- Replace placeholder biometric, push notification, and WebAuthn commands with implementations that:
  - Validate feature gates via `SecurityConfig`.
  - Generate cryptographically strong tokens using `OsRng`.
  - Maintain registration state in cache-aligned structures (push + passkey registries).
- Update `blockchain.rs` to support hex + Bech32 addresses, embed address type markers, and enforce deterministic batch amount calculations.
- Ensure `blockchain_client.rs` and related types remain formatted and unaffected.

### 2. WASM Crate (`wallet/src-tauri/wasm`)
- Expand `Cargo.toml` dependencies (argon2, aes-gcm, serde-wasm-bindgen, etc.) keeping version alignment with workspace.
- Replace placeholder `WasmError` wrappers with custom converter helpers instead of conflicting `From` impls.
- `lib.rs`: manage global configuration store, expose `set_config/current_config`, and add string/list helpers for biometric/push/passkey settings.
- `balance_tracker.rs`: implement WebSocket subscription handling, HTTP fetch utilities, and cache with proper error propagation.
- `crypto_operations.rs`: use `bip39`, `argon2`, `aes-gcm`, `bech32`, etc. to provide deterministic key derivation, encryption, validation, and address generation.
- `chart_renderer.rs`: add animation support with requestAnimationFrame management and easing.

### 3. Angular Core Services (`wallet/src/app/core`)
- `global-error-handler.service.ts`: delegate to `LoggingService`, capture context, suppress console noise in production.
- `logging.service.ts`: configure log levels from `SecurityConfig`, buffer logs locally, and batch ship to backend endpoint.
- `security-config.service.ts`: support environment overrides (window/localStorage), expose sanitized endpoints, persist metadata, and provide helpers for log level & endpoint.
- `wallet-backend.service.ts`: replace mock backend with HTTP client that maps to REST endpoints while sharing DTOs with Tauri mode.

### 4. Angular Features & UI
- `settings.ts`: remove mock push token, request real subscription or deterministic fallback encoder.
- Governance components:
  - `create-proposal.component.ts`: swap out TODO alert for clear messaging + logging when API unavailable.
  - `voting-power.component.ts`: inject `WalletService`, resolve active address, fetch voting power/delegations via Observables, and replace TODO placeholders.

### 5. Documentation & Tests
- Update/author docs summarizing security configuration + governance limitations.
- Add unit tests where practical (e.g., Angular services via dependency injection, Rust modules via cargo test).
- Run `cargo fmt`, `cargo check`, and Angular lint/test commands to confirm rebuilt functionality.

## Next Steps
1. Restore Rust/Tauri code paths first to unblock Angular integration tests.
2. Rebuild WASM modules and run `cargo check -p chert-wallet-wasm`.
3. Re-implement Angular services & components, adding tests/stories as needed.
4. Perform end-to-end manual verification (biometric flows, push registration, governance screens).

Keep this file updated as elements move from “to rebuild” to “restored.”
