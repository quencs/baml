#!/bin/bash
#
# Local CI Pipeline Runner for BAML
#
# This script replicates the GitHub Actions CI pipeline locally, allowing you to run
# all the same checks and tests that run in CI before pushing your changes.
#
# Prerequisites:
# - Node.js 20.x or later
# - Rust (stable toolchain)
# - Go 1.21.x or later
# - Python 3.13
# - Git
#
# The script will automatically install missing tools like pnpm, goimports, ruff, and prettier.
#
# Usage:
#   ./run-local-ci.sh                 # Run full CI pipeline
#   ./run-local-ci.sh --lint-only     # Run only linting checks
#   ./run-local-ci.sh --test-only     # Run only tests
#   ./run-local-ci.sh --build-only    # Run only build steps
#   ./run-local-ci.sh --build-cli     # Include CLI build (normally skipped)
#   ./run-local-ci.sh --skip-deps     # Skip dependency installation checks
#   ./run-local-ci.sh --help          # Show help
#
# What it does:
# 1. Checks and installs required dependencies and tools
# 2. Runs TypeScript and Rust linting/formatting checks
# 3. Tests the Node.js code generator
# 4. Builds WASM targets
# 5. Runs Rust unit tests
# 6. Runs Python integration tests
# 7. Optionally builds the CLI binary
#

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install missing dependencies
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

    # Check if wasm32-unknown-unknown target is installed
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        print_warning "Adding wasm32-unknown-unknown Rust target..."
        rustup target add wasm32-unknown-unknown
    fi

    print_success "All dependencies checked"
}

# Function to install additional tools
install_tools() {
    print_step "Installing additional tools"

        # Install goimports (needed for Go code formatting)
    if ! command_exists goimports; then
        print_warning "Installing goimports..."

        # Show Go version for debugging
        echo "Current Go version: $(go version)"

        # Try installing with latest version
        if ! go install golang.org/x/tools/cmd/goimports@latest 2>/dev/null; then
            print_warning "Failed to install goimports with @latest, trying specific version..."
            # Try with a specific version that's known to work
            if ! go install golang.org/x/tools/cmd/goimports@v0.21.0 2>/dev/null; then
                print_warning "Failed with v0.21.0, trying even older version..."
                if ! go install golang.org/x/tools/cmd/goimports@v0.15.0 2>/dev/null; then
                    # Check if we're only running tests (goimports not critical for tests)
                    if [[ "$RUN_TESTS" == true ]] && [[ "$RUN_LINT" != true ]]; then
                        print_warning "goimports installation failed, but continuing with test-only run..."
                        print_warning "Note: Go code formatting checks will be skipped"
                        return 0
                    fi

                    print_error "Failed to install goimports with multiple versions."
                    print_error "Please try manually:"
                    print_error "  1. Update Go: go install golang.org/x/tools/cmd/goimports@latest"
                    print_error "  2. Or skip goimports: ./run-local-ci.sh --skip-deps"
                    print_error "  3. Check Go env with: go env"
                    exit 1
                fi
            fi
        fi
    fi

    # Install ruff
    if ! command_exists ruff; then
        print_warning "Installing ruff..."
        pip3 install ruff
    fi

    # Install prettier
    if ! command_exists prettier; then
        print_warning "Installing prettier..."
        npm install -g prettier
    fi

    print_success "Additional tools installed"
}

# Function to run lint job
run_lint() {
    print_step "Running lint job"

    # Install pnpm dependencies
    print_step "Installing pnpm dependencies"
    pnpm install --frozen-lockfile

    # Check TS Lint
    print_step "Checking TypeScript lint"
    pnpm format:ci

    # Build playground
    print_step "Building playground"
    pnpm build:playground

    # Check Rust Formatter
    print_step "Checking Rust formatter"
    cd engine
    cargo fmt --check -- --config imports_granularity="Crate" --config group_imports="StdExternalCrate"
    cd ..

    # Check Rust Lint (Clippy)
    print_step "Checking Rust lint (Clippy)"
    cd engine
    RUSTFLAGS="-A unused -D warnings" cargo clippy
    cd ..

    # Check Rust Lint WASM (Clippy)
    print_step "Checking Rust lint WASM (Clippy)"
    cd engine/baml-schema-wasm
    RUSTFLAGS="-A unused -D warnings" cargo clippy --target wasm32-unknown-unknown
    cd ../..

    print_success "Lint job completed"
}

# Function to test Node generator
test_node_generator() {
    print_step "Testing Node generator"

    # Build language client
    print_step "Building language client"
    pnpm --filter=@baml/language-client-typescript build:debug

    # Install Node dependencies for integ-tests
    print_step "Installing Node dependencies for integration tests"
    cd integ-tests/typescript
    pnpm install --frozen-lockfile

    # Test Node Generator (multiple times as in CI)
    for i in {1..3}; do
        print_step "Test Node Generator ($i of 3)"
        pnpm generate || (print_error "merge canary and run codegen again" && exit 1)

        print_step "Ensure No Changes ($i of 3)"
        git diff --exit-code || (print_error "merge canary and run codegen again" && exit 1)
    done

    cd ../..
    print_success "Node generator tests completed"
}

# Function to build WASM
build_wasm() {
    print_step "Building WASM"

    cd engine/baml-schema-wasm
    cargo build --target=wasm32-unknown-unknown
    cd ../..

    print_success "WASM build completed"
}

# Function to run build job
run_build() {
    print_step "Running build job"

    # Test Rust
    print_step "Testing Rust (lib only)"
    cd engine
    cargo test --lib
    cd ..

    print_success "Build job completed"
}

# Function to run integration tests
run_integ_tests() {
    print_step "Running integration tests"

    # Check if uv is available (Python package manager)
    if command_exists uv; then
        print_step "Using uv for Python tests"
    else
        print_warning "uv not found, using regular Python tools"
    fi

    # Run Python tests
    print_step "Running Python tests"
    cd integ-tests/python
    ./run_tests.sh
    cd ../..

    print_success "Integration tests completed"
}

# Function to build CLI (optional, only for releases)
build_cli() {
    print_step "Building CLI (optional)"

    cd engine
    cargo build --release --bin baml-cli
    cd ..

    print_success "CLI build completed"
}

# Main execution
main() {
    print_step "Starting local CI pipeline"

    # Check if we're in the right directory
    if [[ ! -f "pnpm-workspace.yaml" ]] || [[ ! -d "engine" ]]; then
        print_error "Please run this script from the root of the BAML repository"
        exit 1
    fi

    # Parse command line arguments
    RUN_ALL=true
    SKIP_DEPS=false
    SKIP_CLI=true  # Skip CLI build by default as it's only for releases

    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-deps)
                SKIP_DEPS=true
                shift
                ;;
            --build-cli)
                SKIP_CLI=false
                shift
                ;;
            --lint-only)
                RUN_ALL=false
                RUN_LINT=true
                shift
                ;;
            --test-only)
                RUN_ALL=false
                RUN_TESTS=true
                shift
                ;;
            --build-only)
                RUN_ALL=false
                RUN_BUILD=true
                shift
                ;;
            -h|--help)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --skip-deps     Skip dependency checks"
                echo "  --build-cli     Include CLI build (default: skip)"
                echo "  --lint-only     Run only linting"
                echo "  --test-only     Run only tests"
                echo "  --build-only    Run only build"
                echo "  -h, --help      Show this help message"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    # Check dependencies unless skipped
    if [[ "$SKIP_DEPS" != true ]]; then
        check_dependencies
        install_tools
    fi

    # Run selected jobs
    if [[ "$RUN_ALL" == true ]] || [[ "$RUN_LINT" == true ]]; then
        run_lint
    fi

    if [[ "$RUN_ALL" == true ]] || [[ "$RUN_TESTS" == true ]]; then
        test_node_generator
        build_wasm
        run_integ_tests
    fi

    if [[ "$RUN_ALL" == true ]] || [[ "$RUN_BUILD" == true ]]; then
        run_build
    fi

    if [[ "$SKIP_CLI" != true ]]; then
        build_cli
    fi

    print_success "Local CI pipeline completed successfully! 🎉"
}

# Trap errors and provide helpful message
trap 'print_error "Script failed at line $LINENO. Check the output above for details."' ERR

# Run main function
main "$@"