#!/usr/bin/env python3
"""
Command-line interface for decompile-everything skill.
Provides individual command entry points for each mode.
"""

import argparse
import asyncio
import sys
from pathlib import Path

# Import the main tool
from decompile_everything import DecompileEverything, Mode, ExportFormat, RenameMode


def create_main_parser() -> argparse.ArgumentParser:
    """Create the main parser with all subcommands."""
    parser = argparse.ArgumentParser(
        prog="decompile-everything",
        description="Comprehensive Ghidra-powered decompilation and symbol recovery tool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    
    subparsers = parser.add_subparsers(dest="command", help="Available commands")
    
    # ============================================================
    # FULL PIPELINE
    # ============================================================
    full = subparsers.add_parser("full", help="Run complete decompilation pipeline")
    full.add_argument("binary", help="Path to binary file (EXE, DLL, ELF, Mach-O)")
    full.add_argument("-o", "--output", help="Output directory (default: <binary>_decompiled)")
    full.add_argument("--provider", default="ghidra", help="Analysis provider (default: ghidra)")
    full.add_argument("--architecture", help="Force architecture (auto-detected if omitted)")
    full.add_argument("--parallel", type=int, default=8, help="Max parallel functions (default: 8)")
    full.add_argument("--timeout", type=int, default=300, help="Decompilation timeout per function (seconds)")
    full.add_argument("--no-rename", action="store_true", help="Skip AI renaming phase")
    full.add_argument("--no-export", action="store_true", help="Skip export phase")
    full.add_argument("--formats", nargs="+", choices=["c", "rust", "go", "python", "json", "markdown", "assembly"], 
                      default=["c", "json", "markdown"], help="Export formats")
    full.add_argument("--resume", action="store_true", help="Resume from existing project state")
    
    # ============================================================
    # ANALYZE
    # ============================================================
    analyze = subparsers.add_parser("analyze", help="Analyze binary structure")
    analyze.add_argument("binary", help="Path to binary file")
    analyze.add_argument("--provider", default="ghidra", help="Analysis provider")
    analyze.add_argument("--architecture", help="Force architecture")
    analyze.add_argument("--deep", action="store_true", help="Deep analysis (list segments, exports, imports)")
    analyze.add_argument("--output", help="Save analysis report to file")
    
    # ============================================================
    # DECOMPILE
    # ============================================================
    decompile = subparsers.add_parser("decompile", help="Decompile all functions")
    decompile.add_argument("project", help="Project directory (with project_state.json)")
    decompile.add_argument("-o", "--output", help="Output directory for decompilation files")
    decompile.add_argument("--layout", choices=["address", "dwarf"], default="address", help="Output layout")
    decompile.add_argument("--filter", help="Filter functions by name substring")
    decompile.add_argument("--namespace", help="Filter functions by namespace prefix")
    decompile.add_argument("--address-range", help="Address range (start:end in hex)")
    decompile.add_argument("--limit", type=int, help="Max functions to decompile")
    decompile.add_argument("--resume", action="store_true", help="Resume previous decompilation")
    
    # ============================================================
    # RENAME
    # ============================================================
    rename = subparsers.add_parser("rename", help="Rename symbols using AI analysis")
    rename.add_argument("project", help="Project directory")
    rename.add_argument("--mode", choices=["ai", "pattern", "manual"], default="ai", help="Renaming mode")
    rename.add_argument("--dry-run", action="store_true", help="Show proposed changes without applying")
    rename.add_argument("--batch-size", type=int, default=50, help="Batch size for renaming operations")
    rename.add_argument("--confidence", type=float, default=0.7, help="Minimum confidence threshold")
    rename.add_argument("--only-unknown", action="store_true", help="Only rename unknown symbols (FUN_, VAR_, etc.)")
    rename.add_argument("--interactive", action="store_true", help="Interactive confirmation for each rename")
    
    # ============================================================
    # EXPORT
    # ============================================================
    export = subparsers.add_parser("export", help="Export decompilation to target language")
    export.add_argument("project", help="Project directory")
    export.add_argument("-f", "--format", choices=["c", "rust", "go", "python", "json", "markdown", "assembly"], 
                        default="c", help="Export format")
    export.add_argument("-o", "--output", help="Output directory")
    export.add_argument("--split", action="store_true", help="Split into multiple files (one per function)")
    export.add_argument("--include-headers", action="store_true", help="Generate header files (C/Rust)")
    export.add_argument("--include-types", action="store_true", help="Include recovered type definitions")
    export.add_argument("--compile-commands", action="store_true", help="Generate compile_commands.json")
    
    # ============================================================
    # TRACK
    # ============================================================
    track = subparsers.add_parser("track", help="Track progress and verify parity")
    track.add_argument("project", help="Project directory")
    track.add_argument("-r", "--report", help="Output report file (JSON)")
    track.add_argument("--detailed", action="store_true", help="Include detailed per-symbol status")
    track.add_argument("--show-unknown", action="store_true", help="List all unknown symbols")
    track.add_argument("--show-renamed", action="store_true", help="List all renamed symbols")
    
    # ============================================================
    # RECONSTRUCT
    # ============================================================
    reconstruct = subparsers.add_parser("reconstruct", help="Reconstruct compilable source code")
    reconstruct.add_argument("project", help="Project directory")
    reconstruct.add_argument("-o", "--output", help="Output directory")
    reconstruct.add_argument("-l", "--language", choices=["c", "rust", "go", "python"], default="c", help="Target language")
    reconstruct.add_argument("--fix-signatures", action="store_true", help="Attempt to fix function signatures")
    reconstruct.add_argument("--resolve-types", action="store_true", help="Resolve and reconstruct types")
    reconstruct.add_argument("--generate-build", action="store_true", help="Generate build system files (CMake, Cargo.toml, etc.)")
    
    # ============================================================
    # API
    # ============================================================
    api = subparsers.add_parser("api", help="Analyze native API boundaries")
    api.add_argument("project", help="Project directory")
    api.add_argument("--function", help="Analyze specific function (address or name)")
    api.add_argument("--all", action="store_true", help="Analyze all functions")
    api.add_argument("--output", help="Save API analysis to file")
    
    # ============================================================
    # TYPES
    # ============================================================
    types = subparsers.add_parser("types", help="Recover and reconstruct data types")
    types.add_argument("project", help="Project directory")
    types.add_argument("-o", "--output", help="Output directory for type definitions")
    types.add_argument("--format", choices=["c", "rust", "json"], default="c", help="Output format")
    types.add_argument("--include-enums", action="store_true", help="Include enum definitions")
    types.add_argument("--include-structs", action="store_true", help="Include struct/union definitions")
    
    # ============================================================
    # COMPARE
    # ============================================================
    compare = subparsers.add_parser("compare", help="Compare two binaries")
    compare.add_argument("left", help="First binary")
    compare.add_argument("right", help="Second binary")
    compare.add_argument("--mode", choices=["functions", "symbols", "bytes", "full"], default="functions", help="Comparison mode")
    compare.add_argument("-o", "--output", help="Output directory for diff report")
    
    # ============================================================
    # SESSION
    # ============================================================
    session = subparsers.add_parser("session", help="Manage Ghidra analysis session")
    session.add_argument("action", choices=["open", "close", "status", "snapshot"])
    session.add_argument("--binary", help="Binary path (for open)")
    session.add_argument("--snapshot", help="Snapshot path (for close/snapshot)")
    session.add_argument("--provider", default="ghidra", help="Provider to use")
    
    return parser


async def cmd_full(args) -> int:
    """Run full pipeline."""
    tool = DecompileEverything({
        "max_parallel_functions": args.parallel,
        "decompilation_timeout_ms": args.timeout * 1000,
    })
    
    # Convert to namespace expected by run_full_pipeline
    class Args:
        binary = args.binary
        output = args.output
    
    return await tool.run(Mode.FULL, Args())


async def cmd_analyze(args) -> int:
    """Analyze binary."""
    tool = DecompileEverything()
    
    class Args:
        binary = args.binary
    
    return await tool.run(Mode.ANALYZE, Args())


async def cmd_decompile(args) -> int:
    """Decompile functions."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        output = args.output
    
    return await tool.run(Mode.DECOMPILE, Args())


async def cmd_rename(args) -> int:
    """Rename symbols."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        mode = args.mode
        dry_run = args.dry_run
    
    return await tool.run(Mode.RENAME, Args())


async def cmd_export(args) -> int:
    """Export decompilation."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        format = args.format
        output = args.output
    
    return await tool.run(Mode.EXPORT, Args())


async def cmd_track(args) -> int:
    """Track progress."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        report = args.report
    
    return await tool.run(Mode.TRACK, Args())


async def cmd_reconstruct(args) -> int:
    """Reconstruct source."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        output = args.output
        language = args.language
    
    return await tool.run(Mode.RECONSTRUCT, Args())


async def cmd_api(args) -> int:
    """Analyze API."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        function = args.function
    
    return await tool.run(Mode.API, Args())


async def cmd_types(args) -> int:
    """Recover types."""
    tool = DecompileEverything()
    
    class Args:
        project = args.project
        output = args.output
    
    return await tool.run(Mode.TYPES, Args())


async def cmd_compare(args) -> int:
    """Compare binaries."""
    tool = DecompileEverything()
    
    class Args:
        left = args.left
        right = args.right
    
    return await tool.run(Mode.COMPARE, Args())


async def cmd_session(args) -> int:
    """Manage session."""
    tool = DecompileEverything()
    
    class Args:
        action = args.action
        snapshot = args.snapshot
    
    return await tool.run(Mode.SESSION, Args())


COMMAND_MAP = {
    "full": cmd_full,
    "analyze": cmd_analyze,
    "decompile": cmd_decompile,
    "rename": cmd_rename,
    "export": cmd_export,
    "track": cmd_track,
    "reconstruct": cmd_reconstruct,
    "api": cmd_api,
    "types": cmd_types,
    "compare": cmd_compare,
    "session": cmd_session,
}


async def main():
    """Main entry point."""
    parser = create_main_parser()
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return 1
    
    handler = COMMAND_MAP.get(args.command)
    if not handler:
        print(f"Unknown command: {args.command}")
        return 1
    
    return await handler(args)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))