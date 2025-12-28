# Wallet UI Implementation Plan

## Phase A – Cleanup & Readiness
- [x] Move legacy Angular services into `core/services/legacy` quarantine (replaces `services/enhanced-wallet.service.ts` and `services/wasm-loader.service.ts`).
- [x] Remove prompt/alert usage in `app.component.ts`; introduce a shared modal/dialog framework for password and mnemonic flows.
- [x] Add password unlock form component; integrate with `WalletService.unlockWallet` and backoff messaging.
- [x] Implement export confirmation modal with password re-authentication and clipboard-safe display.
- [ ] Fix remaining Angular lint errors in active UI paths (accessibility, inject usage, keyboard handlers).
- [ ] Update `package.json` scripts to include `npm run tauri:test` once lint succeeds.

## Phase B – Core Session UX
- [ ] Wallet onboarding wizard: create/import screens with validation, error handling, and mnemonic confirmation step.
- [ ] Wallet dashboard: show metadata, lock state, remaining attempts, and last update timestamps from `WalletService` signals.
- [x] Change password flow: modal form invoking `WalletService.changePassword` with success/error banner.
- [ ] Signing tools panel: allow message signing/verification using DTOs, display algorithm info and copy helpers.
- [x] Notifications/toasts for success/failure events (unlock, export, change password).

## Phase C – Future RPC Integration Hooks
- [ ] Define placeholder components for balances, transactions, staking, governance with "coming soon" messaging.
- [ ] Create TypeScript interfaces for planned RPC data (balance summaries, transaction entries, validator info) without backend calls yet.
- [ ] Wire placeholders to state service so backend data can drop in when Phase C lands.
- [ ] Document required RPC endpoints and DTOs for future implementation.
