#!/usr/bin/env python3
"""
decompile-everything: CLI entry point for the decompilation skill.

Can be used as:
1. CLI tool: `python -m decompile_everything full --binary target.exe --output ./out`
2. Imported module: `from decompile_everything import run_pipeline_with_tools`
"""

import asyncio
import argparse
import sys
from pathlib import Path

# Import the code-mode pipeline (absolute import)
from decompile_everything.pipeline import run_pipeline_with_tools


def create_parser() -> argparse.ArgumentParser:
    """Create argument parser for CLI usage."""
    parser = argparse.ArgumentParser(
        prog="decompile-everything",
        description="Comprehensive Ghidra-powered decompilation and symbol recovery tool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Full pipeline (requires REA/Ghidra - runs in mock mode without code mode)
  decompile-everything full --binary target.exe --output ./out
  
  # Quick analysis
  decompile-everything analyze --binary target.exe
  
  # Note: For REAL Ghidra/REA integration, use in OpenCode code mode:
  #   result = await run_pipeline_with_tools(tools, "target.exe", "./out")
        """
    )
    
    subparsers = parser.add_subparsers(dest="mode", help="Mode to run")
    
    # Full pipeline
    full_parser = subparsers.add_parser("full", help="Run complete pipeline")
    full_parser.add_argument("--binary", required=True, help="Path to binary file")
    full_parser.add_argument("--output", help="Output directory")
    full_parser.add_argument("--formats", nargs="+", 
        choices=["c", "rust", "go", "python", "json", "markdown", "assembly"],
        default=["c", "json", "markdown"])
    full_parser.add_argument("--rename-mode", choices=["ai", "pattern", "manual"], default="pattern")
    full_parser.add_argument("--max-parallel", type=int, default=8)
    
    # Analyze only
    analyze_parser = subparsers.add_parser("analyze", help="Analyze binary structure")
    analyze_parser.add_argument("--binary", required=True, help="Path to binary file")
    
    # Decompile only
    decompile_parser = subparsers.add_parser("decompile", help="Decompile functions")
    decompile_parser.add_argument("--project", required=True, help="Project directory")
    decompile_parser.add_argument("--output", help="Output directory")
    
    # Rename only
    rename_parser = subparsers.add_parser("rename", help="Rename symbols")
    rename_parser.add_argument("--project", required=True, help="Project directory")
    rename_parser.add_argument("--mode", choices=["ai", "pattern", "manual"], default="pattern")
    rename_parser.add_argument("--dry-run", action="store_true")
    
    # Export only
    export_parser = subparsers.add_parser("export", help="Export decompilation")
    export_parser.add_argument("--project", required=True, help="Project directory")
    export_parser.add_argument("--format", choices=["c", "rust", "go", "python", "json", "markdown", "assembly"], default="c")
    export_parser.add_argument("--output", help="Output directory")
    
    # Track only
    track_parser = subparsers.add_parser("track", help="Track progress/parity")
    track_parser.add_argument("--project", required=True, help="Project directory")
    track_parser.add_argument("--report", help="Report output path")
    
    return parser


async def run_mock_pipeline(args) -> int:
    """Run pipeline in mock mode (for CLI without REA tools)."""
    print("⚠️  Running in MOCK mode - no actual REA/Ghidra connection")
    print("   For REAL analysis, use in OpenCode code mode with actual REA tools")
    print()
    
    # This would need the actual tools namespace - can't run fully without it
    print("Pipeline stages would be:")
    print("  1. open_binary - Open binary with Ghidra")
    print("  2. binary_overview - Get binary metadata")
    print("  3. list_procedures - List all functions")
    print("  4. list_names - List all symbols")
    print("  5. list_strings - Extract strings")
    print("  6. list_segments - Memory layout")
    print("  7. inspect_native_exports - PE/ELF exports")
    print("  8. decompile_all - Bulk decompilation")
    print("  9. analyze_function - Deep function analysis")
    print("  10. annotate_native_function/set_address_name - Rename symbols")
    print("  11. Export to formats:", args.formats if hasattr(args, 'formats') else "c, json, markdown")
    print("  12. Track parity - Generate progress report")
    print("  13. close_binary - Save snapshot")
    return 0


def main():
    """Main CLI entry point."""
    parser = create_parser()
    args = parser.parse_args()
    
    if not args.mode:
        parser.print_help()
        return 1
    
    # For CLI, run in mock mode (actual REA only available in code mode)
    if args.mode in ["full", "analyze", "decompile", "rename", "export", "track"]:
        return asyncio.run(run_mock_pipeline(args))
    
    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())