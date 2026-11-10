#!/usr/bin/env python3
"""Simple wrapper to run the CLI from the root module."""

import sys
import os

# Add the parent directory to path to import the root decompile_everything module
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

from decompile_everything import main

if __name__ == "__main__":
    main()