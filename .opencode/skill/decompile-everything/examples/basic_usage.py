#!/usr/bin/env python3
"""
Basic usage example for decompile-everything skill.

This example demonstrates how to use the skill programmatically.
"""

import asyncio
from pathlib import Path
import tempfile

# Import the skill modules
import sys
sys.path.insert(0, str(Path(__file__).parent.parent))

from decompile_everything import (
    DecompileEverything,
    Mode,
    ExportFormat,
    RenameMode,
)
from pipeline import decompile_everything, quick_analyze


async def example_full_pipeline():
    """Run the full decompilation pipeline."""
    print("=" * 60)
    print("Example: Full Pipeline")
    print("=" * 60)
    
    # Create a dummy binary for demonstration
    with tempfile.TemporaryDirectory() as tmpdir:
        binary_path = Path(tmpdir) / "sample.exe"
        # Write a minimal PE header
        binary_path.write_bytes(b"MZ\x90\x00" + b"\x00" * 500)
        
        output_dir = Path(tmpdir) / "output"
        
        print(f"Binary: {binary_path}")
        print(f"Output: {output_dir}")
        print()
        
        # This would run the full pipeline with a real binary
        # For demo, we'll show the API usage
        print("Running full pipeline (would use real Ghidra/REA)...")
        print("  decompile_everything(")
        print(f'      binary_path="{binary_path}",')
        print(f'      output_dir="{output_dir}",')
        print('      formats=[ExportFormat.C, ExportFormat.RUST, ExportFormat.JSON],')
        print('      rename_mode=RenameMode.AI,')
        print('      max_parallel=8')
        print("  )")
        print()
        print("Expected outputs:")
        print(f"  {output_dir}/project_state.json")
        print(f"  {output_dir}/decompilation/")
        print(f"  {output_dir}/export_c/")
        print(f"  {output_dir}/export_rust/")
        print(f"  {output_dir}/export_json/")
        print(f"  {output_dir}/parity_report.json")
        print(f"  {output_dir}/snapshot.ghidra")


async def example_quick_analysis():
    """Quick analysis without full decompilation."""
    print("=" * 60)
    print("Example: Quick Analysis")
    print("=" * 60)
    
    print("quick_analyze(binary_path)")
    print()
    print("Returns:")
    print("  {")
    print('    "overview": {...},')
    print('    "procedures_count": 1247,')
    print('    "symbols_count": 3421,')
    print('    "strings_count": 567,')
    print('    "exports_count": 42,')
    print("  }")


async def example_iterative_renaming():
    """Iterative renaming workflow."""
    print("=" * 60)
    print("Example: Iterative Renaming")
    print("=" * 60)
    
    print("# 1. Initial decompilation")
    print('await decompile_everything("target.exe", output_dir="./proj")')
    print()
    print("# 2. Dry-run AI rename to preview changes")
    print('tool = DecompileEverything()')
    print('await tool.run(Mode.RENAME, Args(project="./proj", mode="ai", dry_run=True))')
    print()
    print("# 3. Apply renames after review")
    print('await tool.run(Mode.RENAME, Args(project="./proj", mode="ai"))')
    print()
    print("# 4. Verify parity")
    print('await tool.run(Mode.TRACK, Args(project="./proj", report="./parity.json"))')


async def example_cross_language_export():
    """Export to multiple languages."""
    print("=" * 60)
    print("Example: Cross-Language Export")
    print("=" * 60)
    
    formats = [ExportFormat.C, ExportFormat.RUST, ExportFormat.GO, ExportFormat.PYTHON]
    
    for fmt in formats:
        print(f'# Export to {fmt.value.upper()}')
        print(f'await tool.run(Mode.EXPORT, Args(project="./proj", format="{fmt.value}", output="./{fmt.value}_src"))')
        print()


async def example_binary_comparison():
    """Compare two binary versions."""
    print("=" * 60)
    print("Example: Binary Comparison")
    print("=" * 60)
    
    print('# Compare v1.0 vs v1.1')
    print('await tool.run(Mode.COMPARE, Args(left="v1.0.exe", right="v1.1.exe", mode="full"))')
    print()
    print('# Outputs diff report with:')
    print('# - Matched functions (identical)')
    print('# - Changed functions (modified)')
    print('# - Added/removed functions')
    print('# - Symbol differences')


async def example_custom_workflow():
    """Custom workflow using low-level API."""
    print("=" * 60)
    print("Example: Custom Workflow")
    print("=" * 60)
    
    print("""
from decompile_everything import DecompileEverything
from rea_client import REAGhidraClient
from ai_renamer import create_renamer

async def custom_analysis(binary_path):
    # Initialize components
    client = REAGhidraClient()
    renamer = create_renamer("pattern")  # No API key needed
    
    # Open binary
    await client.open_binary(binary_path)
    
    # Get overview
    overview = await client.binary_overview()
    print(f"Architecture: {overview.data['architecture']}")
    
    # List procedures
    procedures = await client.list_procedures()
    print(f"Found {len(procedures.data)} functions")
    
    # Analyze specific functions
    for proc in procedures.data[:5]:  # First 5
        addr = proc['address']
        analysis = await client.analyze_function(addr)
        print(f"  {addr}: {analysis.data['procedure']['signature']}")
    
    # Custom renaming logic
    for proc in procedures.data:
        if proc['value'].startswith('FUN_'):
            # Use pattern renamer
            context = {
                'type': 'function',
                'function': {'address': proc['address'], 'name': proc['value'], 'strings': [], 'callees': [], 'callers': [], 'basic_blocks': [{}]}
            }
            suggestions = await renamer.analyze_function(context)
            if suggestions:
                await client.annotate_native_function(proc['address'], name=suggestions[0].suggested_name)
                print(f"  Renamed {proc['value']} -> {suggestions[0].suggested_name}")
    
    # Close session
    await client.close_binary()

# Run custom analysis
await custom_analysis("target.exe")
""")


async def main():
    """Run all examples."""
    await example_full_pipeline()
    print()
    await example_quick_analysis()
    print()
    await example_iterative_renaming()
    print()
    await example_cross_language_export()
    print()
    await example_binary_comparison()
    print()
    await example_custom_workflow()


if __name__ == "__main__":
    asyncio.run(main())