#!/usr/bin/env python3
"""
Main entry point for decompile-everything skill.
Can be run directly: python -m decompile_everything.run [args]
"""

import sys
import os

# Add the skill directory to path
skill_dir = Path(__file__).parent
sys.path.insert(0, str(skill_dir))

from commands import main

if __name__ == "__main__":
    sys.exit(main())