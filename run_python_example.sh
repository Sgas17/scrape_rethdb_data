#!/bin/bash
# Wrapper script to run Python example with sudo
# The database requires elevated permissions to access

cd "$(dirname "$0")"
sudo -E .venv/bin/python python_example.py
