# Mobile Build Guide for Chert Wallet

This guide covers building the Chert Wallet for Android and iOS platforms using Tauri.

## Prerequisites

### For Android Development
```bash
# Install Android Studio
# Download from: https://developer.android.com/studio

# Install Android SDK (via Android Studio SDK Manager)
# Required SDK: API 21+ (Android 5.0+)

# Set environment variables
export ANDROID_HOME=$HOME/Android/Sdk
export PATH=$PATH:$ANDROID_HOME/tools:$ANDROID_HOME/platform-tools

# Install NDK (if needed for specific features)
# Version: 25.2.9519653 or later
```

### For iOS Development (macOS only)
```bash
# Install Xcode
# Download from: https://developer.apple.com/xcode/

# Install Xcode Command Line Tools
xcode-select --install

# Install CocoaPods (if using plugins that require it)
sudo gem install cocoapods
```

### Tauri Mobile Setup
```bash
# Install Tauri CLI with mobile support
cargo install tauri-cli --version "^2.0.0"

# For iOS development (macOS only)
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim

# For Android development
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

## Development Setup

### 1. Configure Mobile Development

The `tauri.conf.json` is already configured for mobile builds. Key settings:

```json
{
  "iOS": {
    "minimumSystemVersion": "13.0",
    "requiresFullScreen": false
  },
  "android": {
    "minSdkVersion": 21,
    "permissions": [
      "android.permission.INTERNET",
      "android.permission.ACCESS_NETWORK_STATE",
      "android.permission.CAMERA"
    ]
  }
}
```

### 2. Mobile-Specific Code Considerations

#### Camera Access (QR Code Scanning)
The wallet requires camera permissions for QR code scanning. This is already configured in the mobile settings.

#### Storage Permissions
Mobile apps have different storage access patterns. The wallet uses Tauri's secure storage APIs.

#### Network Security
Mobile apps may have additional network restrictions. Ensure your blockchain nodes support mobile connections.

## Building for Mobile

### Android APK/AAB

```bash
# Build Android APK
cd wallet/src-tauri
tauri build --target aarch64-linux-android

# Build Android App Bundle (AAB)
tauri build --target aarch64-linux-android --bundles aab

# Development build
tauri dev --target aarch64-linux-android
```

### iOS IPA (macOS only)

```bash
# Build iOS IPA
cd wallet/src-tauri
tauri build --target aarch64-apple-ios

# Development build
tauri dev --target aarch64-apple-ios-sim
```

## Distribution

### Android
1. **Google Play Store**: Upload the AAB file
2. **Direct APK**: Distribute APK files (not recommended for production)
3. **Internal Testing**: Use Google Play Internal Test Track

### iOS
1. **App Store**: Submit IPA through App Store Connect
2. **TestFlight**: Distribute beta versions
3. **Enterprise**: For internal enterprise distribution

## Mobile-Specific Features

### Biometric Authentication
```rust
// Add to main.rs for biometric unlock
#[tauri::command]
async fn authenticate_biometric() -> Result<bool, String> {
    // Implement biometric authentication
    // iOS: Face ID / Touch ID
    // Android: Fingerprint / Face unlock
    Ok(false) // Placeholder
}
```

### Push Notifications (Future)
```rust
// Add push notification support
#[tauri::command]
async fn register_push_token(token: String) -> Result<(), String> {
    // Register device for push notifications
    // Useful for transaction confirmations, staking rewards, etc.
    Ok(())
}
```

### Deep Linking
```json
// Add to tauri.conf.json for deep linking
{
  "bundle": {
    "iOS": {
      "CFBundleURLTypes": [
        {
          "CFBundleURLSchemes": ["chert"]
        }
      ]
    },
    "android": {
      "intentFilters": [
        {
          "action": "android.intent.action.VIEW",
          "data": {
            "scheme": "chert"
          }
        }
      ]
    }
  }
}
```

## Testing on Mobile

### Android
```bash
# Install on connected device
adb install path/to/app.apk

# Run on emulator
tauri dev --target aarch64-linux-android
```

### iOS
```bash
# Install on connected device
tauri build --target aarch64-apple-ios
# Then use Xcode to install on device

# Run on simulator
tauri dev --target aarch64-apple-ios-sim
```

## Troubleshooting

### Common Android Issues
- **SDK not found**: Ensure `ANDROID_HOME` is set correctly
- **Build tools missing**: Install required build tools via SDK Manager
- **USB debugging**: Enable developer options and USB debugging

### Common iOS Issues
- **Code signing**: Set up proper code signing certificates
- **Provisioning profiles**: Create and install provisioning profiles
- **Simulator issues**: Ensure Xcode simulators are properly configured

### Network Issues
- **CORS**: Mobile apps don't have CORS restrictions like web apps
- **SSL pinning**: Consider implementing SSL pinning for production
- **Network permissions**: Ensure proper network permissions in mobile manifests

## Performance Considerations

### Mobile-Specific Optimizations
- **Bundle size**: Keep APK/IPA sizes reasonable (< 100MB)
- **Memory usage**: Mobile devices have limited RAM
- **Battery life**: Minimize background processing
- **Storage**: Use appropriate storage APIs

### Security Considerations
- **Keychain/KeyStore**: Use platform-specific secure storage
- **Biometric authentication**: Implement device biometric unlock
- **App sandboxing**: Respect platform security boundaries

## CI/CD for Mobile

### GitHub Actions Example
```yaml
name: Mobile Build
on:
  push:
    branches: [main]
  pull_request:

jobs:
  android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '17'
      - run: |
          cd wallet/src-tauri
          tauri build --target aarch64-linux-android

  ios:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: |
          cd wallet/src-tauri
          tauri build --target aarch64-apple-ios
```

## Future Enhancements

### Planned Mobile Features
- [ ] Biometric wallet unlock
- [ ] Push notifications for transactions
- [ ] NFC support for secure transactions
- [ ] Hardware wallet integration
- [ ] Offline transaction signing
- [ ] Multi-wallet support

### Integration Opportunities
- [ ] WalletConnect protocol support
- [ ] DeFi protocol integrations
- [ ] NFT marketplace features
- [ ] Social recovery features

---

## Quick Start Commands

```bash
# Android development
cd wallet/src-tauri
tauri dev --target aarch64-linux-android

# iOS development (macOS only)
cd wallet/src-tauri
tauri dev --target aarch64-apple-ios-sim

# Production builds
tauri build --target aarch64-linux-android  # Android
tauri build --target aarch64-apple-ios     # iOS
```

The wallet is now ready for mobile development!