#!/usr/bin/env python3
"""
decompile-everything - Main entry point for the skill.
Can be installed as a console script.
"""

import sys
from .commands import main

if __name__ == "__main__":
    sys.exit(main())