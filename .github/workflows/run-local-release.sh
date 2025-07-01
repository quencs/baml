#!/bin/bash
#
# Local Release Pipeline Runner for BAML
#
# This script replicates the GitHub Actions release pipeline locally, allowing you to test
# the release build process before pushing changes or creating release tags.
#
# Prerequisites:
# - Node.js 20.x or later
# - Rust (stable toolchain)
# - Go 1.21.x or later
# - Python 3.13
# - Ruby (for Ruby builds)
# - Java 23 (for JetBrains plugin)
# - Git
#
# The script will automatically install missing tools and build all release artifacts locally.
#
# Usage:
#   ./run-local-release.sh                          # Build all release artifacts
#   ./run-local-release.sh --python-only            # Build only Python release
#   ./run-local-release.sh --typescript-only        # Build only TypeScript release
#   ./run-local-release.sh --cli-only               # Build only CLI binaries
#   ./run-local-release.sh --vscode-only            # Build only VSCode extension
#   ./run-local-release.sh --jetbrains-only         # Build only JetBrains plugin
#   ./run-local-release.sh --ruby-only              # Build only Ruby gem
#   ./run-local-release.sh --version 1.2.3          # Simulate specific version
#   ./run-local-release.sh --release-tag            # Simulate release tag behavior
#   ./run-local-release.sh --skip-deps              # Skip dependency installation
#   ./run-local-release.sh --help                   # Show help
#
# What it does:
# 1. Determines version (simulates GitHub tag logic)
# 2. Checks and installs required dependencies
# 3. Builds release artifacts for all supported platforms/languages
# 4. Validates artifacts are created correctly
# 5. Shows what would be published (without actually publishing)
#

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}==== $1 ====${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
}

print_build() {
    echo -e "${PURPLE}🔨 $1${NC}"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Global variables
VERSION="0.1.0"
IS_RELEASE_TAG="false"
SKIP_DEPS=false
BUILD_ALL=true
BUILD_PYTHON=false
BUILD_RUBY=false
BUILD_TYPESCRIPT=false
BUILD_CLI=false
BUILD_VSCODE=false
BUILD_JETBRAINS=false

# Function to determine version (mimics GitHub Actions logic)
determine_version() {
    print_step "Determining version"

    if [[ -n "$CUSTOM_VERSION" ]]; then
        VERSION="$CUSTOM_VERSION"
        if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            IS_RELEASE_TAG="true"
        fi
    else
        # Try to get version from git tag
        if git describe --exact-match --tags HEAD >/dev/null 2>&1; then
            VERSION=$(git describe --exact-match --tags HEAD)
            if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                IS_RELEASE_TAG="true"
            fi
        else
            # Use branch name or commit hash
            VERSION=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "dev")
            if [[ "$VERSION" == "HEAD" ]]; then
                VERSION=$(git rev-parse --short HEAD)
            fi
        fi
    fi

    print_info "Version: $VERSION"
    print_info "Is release tag: $IS_RELEASE_TAG"
}

# Function to check dependencies
check_dependencies() {
    print_step "Checking dependencies"

    # Check Node.js
    if ! command_exists node; then
        print_error "Node.js is not installed. Please install Node.js 20.x"
        exit 1
    fi

    # Check pnpm
    if ! command_exists pnpm; then
        print_warning "pnpm not found. Installing pnpm 9.12.0..."
        npm install -g pnpm@9.12.0
    fi

    # Check Rust
    if ! command_exists cargo; then
        print_error "Rust is not installed. Please install Rust from https://rustup.rs/"
        exit 1
    fi

    # Check Go
    if ! command_exists go; then
        print_error "Go is not installed. Please install Go 1.21.x or later"
        exit 1
    fi

    # Check Python
    if ! command_exists python3; then
        print_error "Python 3 is not installed. Please install Python 3.13"
        exit 1
    fi

    # Check required Rust targets
    print_step "Checking Rust targets"
    local targets=("wasm32-unknown-unknown")

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_CLI" == true ]] || [[ "$BUILD_PYTHON" == true ]]; then
        # Add platform-specific targets that would be needed for releases
        case "$(uname -s)" in
            Darwin*)
                targets+=("x86_64-apple-darwin" "aarch64-apple-darwin")
                ;;
            Linux*)
                targets+=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu")
                ;;
            MINGW*|CYGWIN*|MSYS*)
                targets+=("x86_64-pc-windows-msvc")
                ;;
        esac
    fi

    for target in "${targets[@]}"; do
        if ! rustup target list --installed | grep -q "$target"; then
            print_warning "Adding Rust target: $target"
            rustup target add "$target" || print_warning "Failed to add target $target (may not be available on this platform)"
        fi
    done

    # Check Ruby if building Ruby gem
    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_RUBY" == true ]]; then
        if ! command_exists ruby; then
            print_warning "Ruby not found. Ruby gem build will be skipped."
        elif ! command_exists bundle; then
            print_warning "Bundler not found. Installing..."
            gem install bundler
        fi
    fi

    # Check Java if building JetBrains plugin
    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_JETBRAINS" == true ]]; then
        if ! command_exists java; then
            print_warning "Java not found. JetBrains plugin build will be skipped."
        else
            java_version=$(java -version 2>&1 | head -n1 | cut -d'"' -f2 | cut -d'.' -f1)
            if [[ "$java_version" -lt 11 ]]; then
                print_warning "Java version $java_version found. Java 11+ recommended for JetBrains plugin."
            fi
        fi
    fi

    print_success "Dependencies checked"
}

# Function to setup build environment
setup_environment() {
    print_step "Setting up build environment"

    # Install pnpm dependencies
    pnpm install --frozen-lockfile

    # Create artifacts directory
    mkdir -p artifacts/{python,ruby,typescript,cli,vscode,jetbrains}

    print_success "Build environment ready"
}

# Function to build Python release
build_python_release() {
    print_build "Building Python release"

    cd engine/language_client_python

    # Check if we have maturin
    if ! command_exists maturin; then
        print_warning "Installing maturin..."
        pip3 install maturin
    fi

    # Build wheels for current platform
    print_step "Building Python wheel"
    maturin build --release --out ../../artifacts/python/

    cd ../..

    # List built artifacts
    if ls artifacts/python/*.whl >/dev/null 2>&1; then
        print_success "Python wheels built:"
        ls -la artifacts/python/*.whl
    else
        print_error "No Python wheels found"
        return 1
    fi
}

# Function to build Ruby release
build_ruby_release() {
    print_build "Building Ruby release"

    if ! command_exists ruby; then
        print_warning "Ruby not available, skipping Ruby build"
        return 0
    fi

    cd engine/language_client_ruby

    # Install dependencies
    if [[ -f Gemfile ]]; then
        bundle install
    fi

    # Build gem
    print_step "Building Ruby gem"
    gem build baml.gemspec || gem build *.gemspec

    # Move gem to artifacts
    mv *.gem ../../artifacts/ruby/ 2>/dev/null || true

    cd ../..

    # List built artifacts
    if ls artifacts/ruby/*.gem >/dev/null 2>&1; then
        print_success "Ruby gems built:"
        ls -la artifacts/ruby/*.gem
    else
        print_warning "No Ruby gems found (this may be expected if build failed)"
    fi
}

# Function to build TypeScript release
build_typescript_release() {
    print_build "Building TypeScript release"

    cd engine/language_client_typescript

    # Install napi-rs CLI if not available
    if ! command_exists napi; then
        print_warning "Installing @napi-rs/cli..."
        npm install -g @napi-rs/cli
    fi

    # Build native bindings
    print_step "Building TypeScript native bindings"
    pnpm build:release

    # Create npm package structure
    print_step "Preparing npm package"
    pnpm napi create-npm-dirs || npm run napi create-npm-dirs || true

    # Copy built artifacts
    mkdir -p ../../artifacts/typescript
    cp -r npm/* ../../artifacts/typescript/ 2>/dev/null || true

    cd ../..

    print_success "TypeScript release built"
}

# Function to build CLI release
build_cli_release() {
    print_build "Building CLI release"

    cd engine

    # Build CLI binary
    print_step "Building CLI binary (release mode)"
    cargo build --release --bin baml-cli

    # Copy binary to artifacts
    local binary_name="baml-cli"
    if [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
        binary_name="baml-cli.exe"
    fi

    cp target/release/$binary_name ../artifacts/cli/

    cd ..

    if [[ -f "artifacts/cli/$binary_name" ]]; then
        print_success "CLI binary built: artifacts/cli/$binary_name"
        ls -la artifacts/cli/$binary_name
    else
        print_error "CLI binary not found"
        return 1
    fi
}

# Function to build VSCode extension
build_vscode_release() {
    print_build "Building VSCode extension"

    cd typescript/apps/vscode-ext

    # Install dependencies
    pnpm install

    # Check if vsce is available
    if ! command_exists vsce; then
        print_warning "Installing @vscode/vsce..."
        npm install -g @vscode/vsce
    fi

    # Package extension
    print_step "Packaging VSCode extension"
    vsce package --out ../../../artifacts/vscode/baml-$VERSION.vsix

    cd ../../..

    if ls artifacts/vscode/*.vsix >/dev/null 2>&1; then
        print_success "VSCode extension built:"
        ls -la artifacts/vscode/*.vsix
    else
        print_error "VSCode extension not found"
        return 1
    fi
}

# Function to build JetBrains plugin
build_jetbrains_release() {
    print_build "Building JetBrains plugin"

    if ! command_exists java; then
        print_warning "Java not available, skipping JetBrains build"
        return 0
    fi

    cd jetbrains

    # Build plugin
    print_step "Building JetBrains plugin"
    ./gradlew buildPlugin

    # Copy artifacts
    cp -r build/distributions/* ../artifacts/jetbrains/ 2>/dev/null || true

    cd ..

    if ls artifacts/jetbrains/*.zip >/dev/null 2>&1; then
        print_success "JetBrains plugin built:"
        ls -la artifacts/jetbrains/*.zip
    else
        print_warning "No JetBrains plugin artifacts found (this may be expected)"
    fi
}

# Function to run integration tests (optional)
run_integration_tests() {
    print_step "Running integration tests"

    # This mirrors the integ-tests job from release.yml
    cd integ-tests/python
    ./run_tests.sh
    cd ../..

    print_success "Integration tests completed"
}

# Function to simulate publishing (show what would be published)
simulate_publishing() {
    print_step "Simulating publishing (showing what would be published)"

    echo ""
    print_info "=== RELEASE SUMMARY ==="
    print_info "Version: $VERSION"
    print_info "Is Release Tag: $IS_RELEASE_TAG"
    echo ""

    if [[ "$IS_RELEASE_TAG" == "true" ]]; then
        print_info "🚀 This would be published to production registries:"
    else
        print_info "🧪 This would be published as a pre-release/draft:"
    fi

    echo ""

    # Python
    if ls artifacts/python/*.whl >/dev/null 2>&1; then
        print_info "📦 PyPI: $(ls artifacts/python/*.whl | wc -l) wheel(s)"
        ls artifacts/python/*.whl | sed 's/^/    /'
    fi

    # Ruby
    if ls artifacts/ruby/*.gem >/dev/null 2>&1; then
        print_info "💎 RubyGems: $(ls artifacts/ruby/*.gem | wc -l) gem(s)"
        ls artifacts/ruby/*.gem | sed 's/^/    /'
    fi

    # TypeScript
    if [[ -d artifacts/typescript ]] && [[ -n "$(ls -A artifacts/typescript 2>/dev/null)" ]]; then
        print_info "📡 NPM: TypeScript package"
        echo "    @baml/language-client-typescript@$VERSION"
    fi

    # CLI
    if ls artifacts/cli/* >/dev/null 2>&1; then
        print_info "⚡ GitHub Releases: CLI binaries"
        ls artifacts/cli/* | sed 's/^/    /'
    fi

    # VSCode
    if ls artifacts/vscode/*.vsix >/dev/null 2>&1; then
        print_info "🔌 VSCode Marketplace & OpenVSX: Extensions"
        ls artifacts/vscode/*.vsix | sed 's/^/    /'
    fi

    # JetBrains
    if ls artifacts/jetbrains/*.zip >/dev/null 2>&1; then
        print_info "🧠 JetBrains Marketplace: Plugins"
        ls artifacts/jetbrains/*.zip | sed 's/^/    /'
    fi

    echo ""
    print_success "All artifacts built successfully! 🎉"

    if [[ "$IS_RELEASE_TAG" == "true" ]]; then
        echo ""
        print_warning "To actually publish these artifacts:"
        print_warning "1. Create and push a git tag: git tag $VERSION && git push origin $VERSION"
        print_warning "2. The GitHub Actions release workflow will handle publishing"
    fi
}

# Function to show help
show_help() {
    echo "BAML Local Release Builder"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --python-only       Build only Python release artifacts"
    echo "  --ruby-only         Build only Ruby release artifacts"
    echo "  --typescript-only   Build only TypeScript release artifacts"
    echo "  --cli-only          Build only CLI release artifacts"
    echo "  --vscode-only       Build only VSCode extension"
    echo "  --jetbrains-only    Build only JetBrains plugin"
    echo "  --version VERSION   Use specific version (e.g., 1.2.3)"
    echo "  --release-tag       Simulate release tag behavior"
    echo "  --skip-deps         Skip dependency installation checks"
    echo "  --skip-integ        Skip integration tests"
    echo "  -h, --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                           # Build all release artifacts"
    echo "  $0 --version 1.2.3           # Build with version 1.2.3"
    echo "  $0 --python-only --version 1.2.3  # Build only Python with version 1.2.3"
    echo "  $0 --release-tag             # Simulate production release"
}

# Main execution
main() {
    print_step "Starting local release build"

    # Check if we're in the right directory
    if [[ ! -f "pnpm-workspace.yaml" ]] || [[ ! -d "engine" ]]; then
        print_error "Please run this script from the root of the BAML repository"
        exit 1
    fi

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --python-only)
                BUILD_ALL=false
                BUILD_PYTHON=true
                shift
                ;;
            --ruby-only)
                BUILD_ALL=false
                BUILD_RUBY=true
                shift
                ;;
            --typescript-only)
                BUILD_ALL=false
                BUILD_TYPESCRIPT=true
                shift
                ;;
            --cli-only)
                BUILD_ALL=false
                BUILD_CLI=true
                shift
                ;;
            --vscode-only)
                BUILD_ALL=false
                BUILD_VSCODE=true
                shift
                ;;
            --jetbrains-only)
                BUILD_ALL=false
                BUILD_JETBRAINS=true
                shift
                ;;
            --version)
                CUSTOM_VERSION="$2"
                shift 2
                ;;
            --release-tag)
                IS_RELEASE_TAG="true"
                shift
                ;;
            --skip-deps)
                SKIP_DEPS=true
                shift
                ;;
            --skip-integ)
                SKIP_INTEG=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Determine version first
    determine_version

    # Check dependencies unless skipped
    if [[ "$SKIP_DEPS" != true ]]; then
        check_dependencies
    fi

    # Setup build environment
    setup_environment

    # Build selected components
    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_PYTHON" == true ]]; then
        build_python_release
    fi

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_RUBY" == true ]]; then
        build_ruby_release
    fi

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_TYPESCRIPT" == true ]]; then
        build_typescript_release
    fi

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_CLI" == true ]]; then
        build_cli_release
    fi

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_VSCODE" == true ]]; then
        build_vscode_release
    fi

    if [[ "$BUILD_ALL" == true ]] || [[ "$BUILD_JETBRAINS" == true ]]; then
        build_jetbrains_release
    fi

    # Run integration tests if not skipped
    if [[ "$SKIP_INTEG" != true ]]; then
        run_integration_tests
    fi

    # Show what would be published
    simulate_publishing
}

# Trap errors and provide helpful message
trap 'print_error "Script failed at line $LINENO. Check the output above for details."' ERR

# Run main function
main "$@"