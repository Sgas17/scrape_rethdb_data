#!/bin/bash
# Test script to verify database access

# Set the database path
export RETH_DB_PATH="/var/lib/docker/volumes/eth-docker_reth-el-data/_data/db"

echo "========================================"
echo "Reth Database Access Test"
echo "========================================"
echo ""
echo "Database path: $RETH_DB_PATH"
echo ""

# Check if path exists
if [ ! -d "$RETH_DB_PATH" ]; then
    echo "❌ Error: Database path does not exist!"
    echo "Please check the path or run with sudo if needed."
    exit 1
fi

echo "✅ Database directory exists"
echo ""

# Check for MDBX files
echo "Database files:"
ls -lh "$RETH_DB_PATH"
echo ""

# Check permissions
if [ ! -r "$RETH_DB_PATH" ]; then
    echo "⚠️  Warning: Current user cannot read database directory"
    echo "You may need to run with sudo or adjust permissions"
    echo ""
    echo "Try: sudo chown -R $(whoami):$(whoami) $RETH_DB_PATH"
    echo "Or run the Rust program with: sudo -E env PATH=$PATH cargo run --example collect_pool_data"
    exit 1
fi

echo "✅ Database is readable"
echo ""
echo "Ready to run! Execute:"
echo "  cargo run --example collect_pool_data"
echo ""
