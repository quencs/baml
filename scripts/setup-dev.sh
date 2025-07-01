#!/bin/bash
set -e

echo "🚀 Setting up BAML development environment..."

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
SKIP_PNPM=false
SKIP_CARGO_WATCH=false
SKIP_RUST=false

for arg in "$@"; do
    case $arg in
        --skip-pnpm)
            SKIP_PNPM=true
            shift
            ;;
        --skip-cargo-watch)
            SKIP_CARGO_WATCH=true
            shift
            ;;
        --skip-rust)
            SKIP_RUST=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --skip-pnpm         Skip pnpm installation"
            echo "  --skip-cargo-watch  Skip cargo-watch installation"
            echo "  --skip-rust         Skip Rust/Cargo installation"
            echo "  --help, -h          Show this help message"
            exit 0
            ;;
        *)
            # Unknown option
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check if cargo is installed
if [ "$SKIP_RUST" = false ]; then
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}⚠️  Rust/Cargo is not installed. Installing Rust...${NC}"

        # Fix HOME directory issue in containerized environments (like Vercel)
        # where $HOME might not match the effective user's home directory
        ORIGINAL_HOME=$HOME
        CARGO_HOME=""
        if [ "$HOME" != "$(eval echo ~$(whoami))" ]; then
            echo -e "${YELLOW}🔧 Detected containerized environment, adjusting HOME directory...${NC}"
            ADJUSTED_HOME=$(eval echo ~$(whoami))
            export HOME=$ADJUSTED_HOME
            echo -e "${YELLOW}   Changed HOME from $ORIGINAL_HOME to $HOME${NC}"
            CARGO_HOME="$ADJUSTED_HOME/.cargo"
        fi

        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
        
        # Source cargo environment from the correct location
        if [ -n "$CARGO_HOME" ]; then
            source $CARGO_HOME/env
        else
            source $HOME/.cargo/env
        fi
        
        # Ensure the default toolchain is properly set
        rustup default 1.85.0

        # Restore original HOME if it was changed, but keep CARGO_HOME accessible
        if [ "$ORIGINAL_HOME" != "$HOME" ]; then
            export HOME=$ORIGINAL_HOME
            # Ensure .cargo is accessible from the original HOME
            if [ -n "$CARGO_HOME" ] && [ ! -d "$HOME/.cargo" ]; then
                ln -sf "$CARGO_HOME" "$HOME/.cargo"
            fi
            echo -e "${YELLOW}🔧 Restored HOME to $HOME${NC}"
        fi

        echo -e "${GREEN}✅ Rust installed successfully${NC}"
    else
        echo -e "${GREEN}✅ Rust/Cargo already installed${NC}"
    fi
else
    echo -e "${YELLOW}⏭️  Skipping Rust installation${NC}"
fi

# Check if pnpm is installed
if [ "$SKIP_PNPM" = false ]; then
    if ! command -v pnpm &> /dev/null; then
        echo -e "${YELLOW}⚠️  pnpm is not installed. Installing pnpm...${NC}"
        npm install -g pnpm
        echo -e "${GREEN}✅ pnpm installed successfully${NC}"
    fi
else
    echo -e "${YELLOW}⏭️  Skipping pnpm installation${NC}"
fi

# Install cargo-watch if not already installed
if [ "$SKIP_CARGO_WATCH" = false ]; then
    if ! command -v cargo-watch &> /dev/null; then
        echo -e "${YELLOW}📦 Installing cargo-watch for Rust hot reloading...${NC}"
        cargo install cargo-watch
        echo -e "${GREEN}✅ cargo-watch installed${NC}"
    else
        echo -e "${GREEN}✅ cargo-watch already installed${NC}"
    fi
else
    echo -e "${YELLOW}⏭️  Skipping cargo-watch installation${NC}"
fi

# Install wasm-bindgen-cli if not already installed (needed for WASM builds)
if [ "$SKIP_RUST" = false ]; then
    if ! command -v wasm-bindgen &> /dev/null; then
        echo -e "${YELLOW}📦 Installing wasm-bindgen-cli...${NC}"
        cargo install wasm-bindgen-cli --version 0.2.92
        echo -e "${GREEN}✅ wasm-bindgen-cli installed${NC}"
    else
        echo -e "${GREEN}✅ wasm-bindgen-cli already installed${NC}"
    fi

    # Install wasm-pack if not already installed (needed for building Rust WASM packages)
    if ! command -v wasm-pack &> /dev/null; then
        echo -e "${YELLOW}📦 Installing wasm-pack...${NC}"
        cargo install wasm-pack
        echo -e "${GREEN}✅ wasm-pack installed${NC}"
    else
        echo -e "${GREEN}✅ wasm-pack already installed${NC}"
    fi
else
    echo -e "${YELLOW}⏭️  Skipping WASM tools installation (Rust installation was skipped)${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Development environment setup complete!${NC}"
echo ""
echo "You can now run:"
echo "  pnpm dev              # Run everything with hot reloading"
echo "  pnpm dev:vscode-full  # Run VSCode extension with all dependencies"
echo "  pnpm dev:playground   # Run just the playground"
echo ""
echo "For VSCode extension debugging:"
echo "  1. Run 'pnpm dev:vscode-full'"
echo "  2. Press F5 in VSCode to launch the extension host"