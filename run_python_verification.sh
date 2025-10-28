#!/bin/bash
# Quick setup and run script for Python DB verification with uv

set -e

echo "========================================="
echo "Python DB Verification Setup (uv)"
echo "========================================="
echo

# Check if uv is installed
if ! command -v uv &> /dev/null; then
    echo "❌ uv is not installed"
    echo "Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
    exit 1
fi

echo "✓ uv is installed"

# Install Python dependencies
echo
echo "Installing Python dependencies..."
uv pip install web3 python-dotenv maturin

echo
echo "✓ Dependencies installed"

# Build Python bindings
echo
echo "Building Python bindings..."
uv run maturin develop --features python

echo
echo "✓ Python bindings built"

# Check .env file
if [ ! -f .env ]; then
    echo
    echo "⚠️  No .env file found"
    echo "Creating template .env file..."
    cat > .env << 'ENVEOF'
RETH_DB_PATH=/path/to/reth/db
RPC_URL=http://localhost:8545
ENVEOF
    echo "✓ Created .env template - please edit with your values"
    exit 1
fi

echo
echo "✓ .env file found"

# Run the verification test
echo
echo "========================================="
echo "Running Verification Test"
echo "========================================="
echo

uv run python verify_db_vs_rpc.py

echo
echo "========================================="
echo "Test Complete"
echo "========================================="
