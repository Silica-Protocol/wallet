# Chert Coin Wallet

A secure, multi-platform wallet for the Chert blockchain featuring post-quantum cryptography, staking, and governance capabilities.

## Features

- **Multi-Platform Support**: Web, Android, iOS, Linux, Windows, macOS
- **Post-Quantum Security**: Dilithium-2 signatures for future-proof security
- **Modern UI**: Angular frontend with Material Design
- **Comprehensive Features**: Send/receive, staking, governance, transaction history
- **Secure Storage**: Encrypted key storage with OS keychain integration
- **Real-time Updates**: Live balance and transaction monitoring
- **Mobile Native**: Full mobile apps for Android and iOS

## Technology Stack

- **Frontend**: Angular 17+ with TypeScript
- **Backend**: Tauri (Rust) for native integration
- **Mobile**: Tauri Mobile for Android/iOS native apps
- **Web**: Standalone web wallet with HTTP API integration
- **Blockchain**: Direct integration with Chert's Silica node
- **Cryptography**: Ed25519 + Dilithium-2 hybrid signatures
- **Storage**: RocksDB for local data, OS keychain for sensitive data

## Development Setup

### Prerequisites

- Node.js 18+ with npm
- Rust 1.70+ with Cargo
- Angular CLI 17+

### Installation

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Install Tauri CLI** (if not already installed):
   ```bash
   cargo install tauri-cli
   ```

3. **Run in development mode**:
   ```bash
   npm run tauri:dev
   ```

### Building

- **Development build**:
  ```bash
  npm run ng:build
  ```

- **Production build for all platforms**:
  ```bash
  npm run tauri:build
  ```

## Platform-Specific Builds

### Web
The wallet can run as a Progressive Web App (PWA) in any modern browser:
```bash
npm run ng:serve
```

### Desktop (Linux, Windows, macOS)
Built using Tauri for native performance:
```bash
npm run tauri:build
```

### Android
Requires Android SDK and build tools:
```bash
# Check prerequisites
npm run mobile:check

# Development
npm run mobile:dev:android

# Production build
npm run mobile:android
```

### iOS (macOS only)
Requires Xcode and iOS SDK:
```bash
# Check prerequisites
npm run mobile:check

# Development
npm run mobile:dev:ios

# Production build
npm run mobile:ios
```

For detailed mobile setup instructions, see [MOBILE_BUILD_GUIDE.md](MOBILE_BUILD_GUIDE.md).

## Architecture

### Frontend (Angular)
- **Services**: Wallet operations, blockchain communication
- **Components**: Reusable UI components for transactions, staking, etc.
- **Pages**: Main wallet views (dashboard, send, receive, stake, governance)

### Backend (Tauri/Rust)
- **Wallet Manager**: Key generation, storage, and transaction signing
- **Blockchain Client**: Communication with Chert nodes
- **Crypto Engine**: Post-quantum cryptographic operations
- **Storage Layer**: Encrypted local data persistence

## Security Features

### Post-Quantum Cryptography
- **Dilithium-2**: NIST-standardized post-quantum signatures
- **Ed25519**: Classical elliptic curve signatures for current compatibility
- **Hybrid Approach**: Both signature types for maximum security

### Secure Key Management
- **OS Integration**: Platform-specific secure storage (Keychain, Credential Manager)
- **Encryption**: AES-256-GCM for local data encryption
- **Zero-Knowledge**: Private keys never leave the device

### Network Security
- **TLS 1.3**: All network communications encrypted
- **Certificate Pinning**: Protection against man-in-the-middle attacks
- **Request Signing**: All API requests cryptographically signed

## Usage

### Creating a New Wallet
1. Click "Create New Wallet"
2. Enter a secure password
3. Save the generated mnemonic phrase securely
4. Your wallet is ready to use

### Importing an Existing Wallet
1. Click "Import Existing Wallet"
2. Enter your mnemonic phrase
3. Set a password for local encryption
4. Access your existing funds

### Sending Transactions
1. Click "Send" from the main dashboard
2. Enter recipient address and amount
3. Add optional memo
4. Review and confirm transaction
5. Enter password to sign and broadcast

### Staking
1. Navigate to "Stake" section
2. Browse available validators
3. Select validator and amount to delegate
4. Confirm delegation transaction
5. Monitor rewards in real-time

### Governance
The Chert wallet includes a comprehensive governance interface for participating in network decisions.

#### Viewing Proposals
1. Navigate to the "Governance" section from the main menu
2. Browse the list of active proposals with status indicators
3. Filter proposals by state (Active, Pending, Succeeded, etc.)
4. Click on any proposal to view detailed information

#### Proposal Details
- **Proposal Information**: Title, description, proposer, voting period
- **Voting Statistics**: Current vote counts (For/Against/Abstain)
- **Voting Power**: Your current voting power and delegation status
- **Actions**: Cast vote, delegate voting power, or create new proposal

#### Casting Votes
1. Select a proposal from the list
2. Choose your vote: For, Against, or Abstain
3. Optionally add a reason for your vote
4. Confirm and sign the transaction with your wallet password

#### Managing Voting Power
1. Access "Voting Power" from the governance menu
2. View your current balance and delegated power
3. Delegate voting power to other addresses for representation
4. Track delegation history and rewards

#### Creating Proposals
1. Click "Create Proposal" from the governance menu
2. Fill in proposal details:
   - Title and description
   - Target contracts and function calls
   - Voting parameters (start/end times)
3. Review and submit the proposal transaction
4. Monitor proposal status and community feedback

#### Delegation
1. Go to "Voting Power" > "Delegate"
2. Enter the address to delegate to
3. Specify the amount of voting power to delegate
4. Confirm the delegation transaction
5. Manage and revoke delegations as needed

## Configuration

### Network Settings
The wallet can connect to different Chert networks:
- **Mainnet**: Production network
- **Testnet**: Testing network
- **Local**: Development node

### Node Configuration
Custom node endpoints can be configured for:
- RPC URL
- WebSocket URL
- Explorer URL

## Security Considerations

- Never share your mnemonic phrase or private keys
- Use strong passwords for wallet encryption
- Keep the application updated for security patches
- Verify transaction details before signing
- Use hardware wallets for large amounts (when supported)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

For support and questions:
- GitHub Issues: Report bugs and feature requests
- Discord: Join the Chert community
- Documentation: Comprehensive guides and API reference