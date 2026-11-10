"""
decompile-everything: Comprehensive Ghidra-powered decompilation skill for OpenCode.

This skill provides a complete pipeline for:
- Binary analysis with Ghidra via REA
- Full function decompilation
- AI-assisted symbol renaming
- Multi-format code export (C, Rust, Go, Python, JSON, Markdown)
- Progress tracking and parity verification
- Source reconstruction for reimplementation
- Binary comparison and diffing
- Native API boundary analysis
- Data type recovery

Usage:
    from decompile_everything import DecompileEverything
    
    tool = DecompileEverything()
    await tool.run(Mode.FULL, args)
"""

from .decompile_everything import (
    DecompileEverything,
    Mode,
    ExportFormat,
    RenameMode,
    SymbolInfo,
    FunctionInfo,
    BinaryInfo,
    ProjectState,
    ProgressTracker,
)

__version__ = "1.0.0"
__author__ = "OpenCode Agent"
__license__ = "MIT"

__all__ = [
    "DecompileEverything",
    "Mode",
    "ExportFormat", 
    "RenameMode",
    "SymbolInfo",
    "FunctionInfo",
    "BinaryInfo",
    "ProjectState",
    "ProgressTracker",
]