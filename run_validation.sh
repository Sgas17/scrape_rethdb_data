#!/bin/bash
# Wrapper script to run validation with proper environment

# Activate the venv from tmp and run validation
cd /tmp/scrape_test
source .venv/bin/activate

# Force Python to not use cached bytecode
export PYTHONDONTWRITEBYTECODE=1

# Remove any cached imports
rm -rf __pycache__ 2>/dev/null || true

python /home/sam-sullivan/scrape_rethdb_data/validate_simple.py
