# Chert Wallet Development Environment Setup Guide

## Prerequisites

Before setting up the development environment, ensure you have the following installed:

### Required Tools
- **Node.js** 18+ with npm
- **Rust** 1.70+ with Cargo
- **Angular CLI** 17+
- **wasm-pack** (for WebAssembly compilation)

### Optional Tools
- **Git** for version control
- **VS Code** with recommended extensions
- **Chrome DevTools** for debugging

## Step 1: Install Prerequisites

### Install Node.js and npm
```bash
# Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Or download from https://nodejs.org/
```

### Install Rust and Cargo
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Install Angular CLI
```bash
npm install -g @angular/cli@17
```

### Install wasm-pack
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Step 2: Verify Existing Project Structure

The Chert wallet project should have the following structure:

```
wallet/
├── src/                          # Angular source code
│   ├── app/
│   │   ├── services/            # Angular services
│   │   ├── components/          # Angular components
│   │   └── types/              # TypeScript types
│   └── assets/                 # Static assets
├── src-tauri/                   # Tauri backend
│   ├── wasm/                   # WebAssembly modules
│   │   └── src/               # Rust source files
│   └── Cargo.toml             # Rust dependencies
├── package.json                # Node.js dependencies
├── angular.json               # Angular configuration
├── build-wasm.sh             # WebAssembly build script
└── README.md                 # Project documentation
```

## Step 3: Install Node.js Dependencies

```bash
cd wallet
npm install
```

This will install all Angular dependencies including:
- Angular 17+ framework
- Material Design components
- RxJS for reactive programming
- TypeScript and ESLint
- Tauri integration packages

## Step 4: Verify Rust Dependencies

Check that the WebAssembly dependencies are properly configured:

```bash
cd src-tauri/wasm
cargo check
```

If you encounter any issues, run:
```bash
cargo update
cargo build
```

## Step 5: Set Up Environment Configuration

Create environment configuration files:

### Development Environment
```bash
# Create development environment file
cp .env.example .env.development
```

Edit `.env.development` with your development settings:
```env
# API Endpoints
CHERT_API_ENDPOINT=https://dev-api.chert.com
CHERT_WS_ENDPOINT=wss://dev-ws.chert.com

# Network Configuration
CHERT_NETWORK=devnet
CHERT_CHAIN_ID=1337

# Feature Flags
ENABLE_PERFORMANCE_MONITORING=true
ENABLE_DEBUG_LOGGING=true
```

### Production Environment
```bash
# Create production environment file
cp .env.example .env.production
```

Edit `.env.production` with your production settings:
```env
# API Endpoints
CHERT_API_ENDPOINT=https://api.chert.com
CHERT_WS_ENDPOINT=wss://ws.chert.com

# Network Configuration
CHERT_NETWORK=mainnet
CHERT_CHAIN_ID=1

# Feature Flags
ENABLE_PERFORMANCE_MONITORING=false
ENABLE_DEBUG_LOGGING=false
```

## Step 6: Configure Angular for WebAssembly

### Update angular.json
Ensure your `angular.json` includes WebAssembly assets:

```json
{
  "projects": {
    "chert-wallet": {
      "architect": {
        "build": {
          "options": {
            "assets": [
              "src/favicon.ico",
              "src/assets",
              "src/assets/wasm"
            ]
          }
        }
      }
    }
  }
}
```

### Update tsconfig.json
Add WebAssembly support to TypeScript configuration:

```json
{
  "compilerOptions": {
    "allowJs": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable", "WebWorker"]
  }
}
```

## Step 7: Make Build Scripts Executable

```bash
chmod +x build-wasm.sh
```

## Step 8: Verify Development Setup

### Test Angular Development Server
```bash
npm run ng:serve
```

Navigate to `http://localhost:4242` to verify the Angular app loads.

### Test Tauri Development
```bash
npm run tauri:dev
```

This should launch the desktop application with hot reload.

### Test WebAssembly Build
```bash
./build-wasm.sh
```

This should build the WebAssembly modules without errors.

## Step 9: Install VS Code Extensions (Optional)

For the best development experience, install these VS Code extensions:

```bash
# Install extensions using code CLI
code --install-extension ms-vscode.vscode-typescript-next
code --install-extension rust-lang.rust-analyzer
code --install-extension ms-vscode.vscode-json
code --install-extension angular.ng-template
code --install-extension bradlc.vscode-tailwindcss
code --install-extension esbenp.prettier-vscode
code --install-extension ms-vscode.vscode-eslint
```

## Step 10: Configure Git Hooks (Optional)

Set up pre-commit hooks for code quality:

```bash
# Install husky for Git hooks
npm install --save-dev husky

# Initialize husky
npx husky install

# Add pre-commit hook
npx husky add .husky/pre-commit "npm run lint && npm run test"
```

## Common Issues and Solutions

### Issue: wasm-pack not found
```bash
# Solution: Reinstall wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### Issue: Rust compilation errors
```bash
# Solution: Update Rust toolchain
rustup update
cargo clean
cargo build
```

### Issue: Angular build fails with WebAssembly
```bash
# Solution: Clean Angular cache
npm run clean
npm install
npm run ng:build
```

### Issue: Tauri development server won't start
```bash
# Solution: Check port availability
lsof -i :4242
# Kill any process using port 4242, then restart
npm run tauri:dev
```

## Development Workflow

### 1. Start Development
```bash
# Terminal 1: Start Angular dev server
npm run ng:serve

# Terminal 2: Start Tauri (for desktop development)
npm run tauri:dev
```

### 2. Build WebAssembly
```bash
# Build WASM modules
./build-wasm.sh

# Or watch for changes (requires additional setup)
npm run wasm:watch
```

### 3. Run Tests
```bash
# Run Angular tests
npm run test

# Run Rust tests
cd src-tauri/wasm && cargo test
```

### 4. Lint Code
```bash
# Lint TypeScript
npm run lint

# Lint Rust
cd src-tauri/wasm && cargo clippy
```

### 5. Build for Production
```bash
# Build Angular for production
npm run ng:build:prod

# Build WebAssembly for production
./build-wasm.sh

# Build Tauri application
npm run tauri:build
```

## Environment Variables

The application supports the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CHERT_API_ENDPOINT` | API server URL | `https://api.chert.com` |
| `CHERT_WS_ENDPOINT` | WebSocket server URL | `wss://ws.chert.com` |
| `CHERT_NETWORK` | Network name | `mainnet` |
| `CHERT_CHAIN_ID` | Chain ID | `1` |
| `ENABLE_PERFORMANCE_MONITORING` | Enable performance tracking | `true` |
| `ENABLE_DEBUG_LOGGING` | Enable debug logs | `false` |

## Next Steps

Once your development environment is set up:

1. **Build the WebAssembly modules** using the provided build script
2. **Integrate WASM services** with Angular components
3. **Implement core wallet functionality** using the enhanced services
4. **Add real-time balance updates** via WebSocket connections

For detailed implementation guidance, refer to the [Implementation Summary](IMPLEMENTATION_SUMMARY.md) and the comprehensive [Development Plan](DEVELOPMENT_PLAN.md).

## Troubleshooting

If you encounter any issues during setup:

1. Check the [Common Issues](#common-issues-and-solutions) section above
2. Verify all prerequisites are installed correctly
3. Ensure all environment variables are set properly
4. Check that all dependencies are up to date
5. Review the build logs for specific error messages

For additional support, create an issue in the project repository or contact the development team.