import { TestBed } from '@angular/core/testing';
import { AppComponent } from './app.component';
import { WALLET_BACKEND } from './core/services/wallet-backend.interface';
import { WasmService } from './core/services/wasm.service';

class MockWasmService {
  waitForReady(): Promise<void> {
    return Promise.resolve();
  }
}

const mockBackend = {
  createWallet: async () => {
    const summary = {
      walletName: 'Mock',
      createdAt: '',
      updatedAt: '',
      schemaVersion: 1,
      primaryAddress: 'chert_mock',
      publicKeyHex: 'mock_pk',
      signatureAlgorithm: 'Ed25519',
      supportsPostQuantum: false
    };
    return {
      summary,
      address: 'chert_mock',
      publicKey: 'pk',
      mnemonic: 'mock mnemonic',
      supportsPostQuantum: false,
      algorithm: 'Ed25519'
    };
  },
  importWallet: async () => ({
    summary: {
      walletName: 'Mock',
      createdAt: '',
      updatedAt: '',
      schemaVersion: 1,
      primaryAddress: 'chert_mock',
      publicKeyHex: 'mock_pk',
      signatureAlgorithm: 'Ed25519',
      supportsPostQuantum: false
    },
    address: 'chert_mock',
    publicKey: 'pk',
    supportsPostQuantum: false,
    algorithm: 'Ed25519'
  }),
  unlockWallet: async () => ({
    summary: {
      walletName: 'Mock',
      createdAt: '',
      updatedAt: '',
      schemaVersion: 1,
      primaryAddress: 'chert_mock',
      publicKeyHex: 'mock_pk',
      signatureAlgorithm: 'Ed25519',
      supportsPostQuantum: false
    },
    remainingAttempts: 5
  }),
  lockWallet: async () => ({ locked: true }),
  getWalletInfo: async () => ({
    exists: false,
    isLocked: true,
    remainingAttempts: 0,
    metadata: null,
    config: {
      network: { primaryEndpoint: '', failoverEndpoints: [], allowUntrustedCerts: false },
      session: { autoLockMinutes: 15, maxFailedAttempts: 5 },
      telemetry: { enableAnalytics: false, allowErrorReports: false },
      environment: 'test',
      lastUpdated: '',
      version: 1
    }
  }),
  exportWallet: async () => ({
    summary: {
      walletName: 'Mock',
      createdAt: '',
      updatedAt: '',
      schemaVersion: 1,
      primaryAddress: 'chert_mock',
      publicKeyHex: 'mock_pk',
      signatureAlgorithm: 'Ed25519',
      supportsPostQuantum: false
    },
    mnemonic: 'mock mnemonic',
    seedHex: 'seed',
    stealthMaterialHex: 'stealth',
    pqMaterialHex: 'pq'
  }),
  changePassword: async () => ({
    summary: {
      walletName: 'Mock',
      createdAt: '',
      updatedAt: '',
      schemaVersion: 1,
      primaryAddress: 'chert_mock',
      publicKeyHex: 'mock_pk',
      signatureAlgorithm: 'Ed25519',
      supportsPostQuantum: false
    }
  }),
  signMessage: async () => ({ signatureHex: 'sig', algorithm: 'Ed25519', publicKeyHex: 'pk' }),
  verifyMessageSignature: async () => ({ valid: true }),
  validateAddress: async () => ({ isValid: true }),
  formatAmount: async () => ({ formatted: '0' }),
  getBalance: async () => ({ address: 'chert_mock', balance: '0', nonce: 0 }),
  getTransactionHistory: async () => ({ transactions: [], totalCount: 0 }),
  getProposals: async () => ({ proposals: [] }),
  getProposal: async () => ({ proposal: { id: 1, title: 'Mock', description: 'Mock', proposer: 'mock', voteStart: 0, voteEnd: 0, votesFor: 0, votesAgainst: 0, votesAbstain: 0, state: 'pending', executedAt: null, createdAt: '', updatedAt: '' } }),
  getProposalVotes: async () => ({ votes: [] }),
  getVotingPower: async () => ({ voting_power: { address: 'mock', balance: '0', delegated: '0', total: '0' } }),
  getDelegations: async () => ({ delegations: [] }),
  castVote: async () => ({ vote: { proposalId: 1, voter: 'mock', support: 1, weight: '0', reason: null, votedAt: '' } }),
  delegate: async () => ({ delegation: { delegator: 'mock', delegatee: 'mock', amount: '0', delegatedAt: '' } }),
  createProposal: async () => ({ proposal: { id: 1, title: 'Mock', description: 'Mock', proposer: 'mock', voteStart: 0, voteEnd: 0, votesFor: 0, votesAgainst: 0, votesAbstain: 0, state: 'pending', executedAt: null, createdAt: '', updatedAt: '' } })
};

describe('AppComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [
        { provide: WasmService, useClass: MockWasmService },
        { provide: WALLET_BACKEND, useValue: mockBackend }
      ]
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it('should have the expected title', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app.title).toEqual('Chert Wallet');
  });

  it('should render title', () => {
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('h1')?.textContent).toContain('Chert Wallet');
  });
});
