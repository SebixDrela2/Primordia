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

Usage in OpenCode code mode:
    from decompile_everything import run_pipeline_with_tools
    
    result = await run_pipeline_with_tools(
        tools,  # OpenCode's tools namespace
        binary_path="D:\\target.exe",
        output_dir="./analysis",
        formats=["c", "rust", "json", "markdown"]
    )
"""

from .pipeline import (
    run_pipeline_with_tools,
    ExportFormat,
    RenameMode,
    SymbolInfo,
    FunctionInfo,
    BinaryInfo,
    ProjectState,
    ProgressTracker,
    IntelligentRenamer,
)

__version__ = "1.0.0"
__author__ = "OpenCode Agent"
__license__ = "MIT"

__all__ = [
    "run_pipeline_with_tools",
    "ExportFormat",
    "RenameMode",
    "SymbolInfo",
    "FunctionInfo",
    "BinaryInfo",
    "ProjectState",
    "ProgressTracker",
    "IntelligentRenamer",
]