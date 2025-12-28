#!/bin/bash

# Chert Wallet Mobile Build Script
# This script helps build the wallet for Android and iOS platforms

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "src-tauri/tauri.conf.json" ]; then
    print_error "Please run this script from the wallet/src-tauri directory"
    exit 1
fi

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."

    # Check if cargo is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust first."
        exit 1
    fi

    # Check if tauri-cli is installed
    if ! command -v tauri &> /dev/null; then
        print_error "Tauri CLI is not installed. Run: cargo install tauri-cli"
        exit 1
    fi
}

# Function to build for Android
build_android() {
    print_status "Building for Android..."

    # Check Android prerequisites
    if [ -z "$ANDROID_HOME" ]; then
        print_error "ANDROID_HOME environment variable is not set."
        print_error "Please install Android Studio and set ANDROID_HOME."
        exit 1
    fi

    if [ ! -d "$ANDROID_HOME" ]; then
        print_error "ANDROID_HOME directory does not exist: $ANDROID_HOME"
        exit 1
    fi

    # Add Android targets
    print_status "Adding Android Rust targets..."
    rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android

    # Build APK
    print_status "Building Android APK..."
    tauri build --target aarch64-linux-android

    if [ $? -eq 0 ]; then
        print_success "Android APK built successfully!"
        print_status "APK location: src-tauri/target/aarch64-linux-android/release/bundle/apk/"
    else
        print_error "Android build failed!"
        exit 1
    fi
}

# Function to build for iOS
build_ios() {
    print_status "Building for iOS..."

    # Check if we're on macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "iOS builds are only supported on macOS."
        exit 1
    fi

    # Check if Xcode is installed
    if ! command -v xcodebuild &> /dev/null; then
        print_error "Xcode is not installed. Please install Xcode from the App Store."
        exit 1
    fi

    # Add iOS targets
    print_status "Adding iOS Rust targets..."
    rustup target add aarch64-apple-ios aarch64-apple-ios-sim

    # Build IPA
    print_status "Building iOS IPA..."
    tauri build --target aarch64-apple-ios

    if [ $? -eq 0 ]; then
        print_success "iOS IPA built successfully!"
        print_status "IPA location: src-tauri/target/aarch64-apple-ios/release/bundle/ios/"
    else
        print_error "iOS build failed!"
        exit 1
    fi
}

# Function to run development server
dev_android() {
    print_status "Starting Android development server..."

    if [ -z "$ANDROID_HOME" ]; then
        print_error "ANDROID_HOME environment variable is not set."
        exit 1
    fi

    rustup target add aarch64-linux-android
    tauri dev --target aarch64-linux-android
}

dev_ios() {
    print_status "Starting iOS development server..."

    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "iOS development is only supported on macOS."
        exit 1
    fi

    rustup target add aarch64-apple-ios-sim
    tauri dev --target aarch64-apple-ios-sim
}

# Main script logic
case "$1" in
    "android")
        check_prerequisites
        build_android
        ;;
    "ios")
        check_prerequisites
        build_ios
        ;;
    "dev-android")
        check_prerequisites
        dev_android
        ;;
    "dev-ios")
        check_prerequisites
        dev_ios
        ;;
    "check")
        check_prerequisites
        print_success "Prerequisites check passed!"
        ;;
    *)
        echo "Chert Wallet Mobile Build Script"
        echo ""
        echo "Usage:"
        echo "  $0 android          - Build Android APK"
        echo "  $0 ios             - Build iOS IPA (macOS only)"
        echo "  $0 dev-android     - Start Android development server"
        echo "  $0 dev-ios         - Start iOS development server (macOS only)"
        echo "  $0 check           - Check prerequisites"
        echo ""
        echo "Examples:"
        echo "  $0 android"
        echo "  $0 dev-android"
        echo ""
        echo "For detailed instructions, see MOBILE_BUILD_GUIDE.md"
        exit 1
        ;;
esac</content>
<filePath>wallet/src-tauri/build-mobile.sh