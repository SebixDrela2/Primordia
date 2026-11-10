#!/usr/bin/env python3
"""
Advanced REA integration example for OpenCode code mode.

This example shows how to use the REA Ghidra tools directly within
OpenCode's code execution environment where the `tools.rea.*` namespace
is available.
"""

import asyncio
import json
from typing import Dict, List, Any, Optional
from dataclasses import dataclass, asdict
from pathlib import Path


# This code is designed to run within OpenCode's code mode
# where the `tools` namespace provides access to REA tools

@dataclass
class FunctionAnalysis:
    address: str
    name: str
    signature: str
    pseudocode: str
    assembly: List[str]
    callers: List[str]
    callees: List[str]
    strings: List[str]
    variables: List[Dict[str, Any]]
    basic_blocks: List[Dict[str, Any]]
    limitations: List[str]


class CodeModeDecompiler:
    """
    Decompiler that uses REA tools directly in OpenCode code mode.
    
    Usage in OpenCode:
    
    ```python
    # In code mode, tools are available via the execute tool
    decompiler = CodeModeDecompiler(tools)
    await decompiler.open_binary("/path/to/binary.exe")
    functions = await decompiler.list_all_functions()
    for func in functions:
        analysis = await decompiler.analyze_function(func.address)
        print(analysis.pseudocode)
    ```
    """
    
    def __init__(self, tools_namespace):
        """
        Initialize with the tools namespace from code mode.
        
        Args:
            tools_namespace: The `tools` object from code mode execution
        """
        self.tools = tools_namespace
        self.session_active = False
        self.binary_info = None
        self.functions_cache = []
    
    async def open_binary(
        self,
        path: str,
        format: Optional[str] = None,
        architecture: Optional[str] = None,
        entry_point: Optional[str] = None,
        provider_id: str = "ghidra",
        snapshot_path: Optional[str] = None
    ) -> Dict[str, Any]:
        """Open a binary for analysis."""
        params = {"path": path, "provider_id": provider_id}
        if format:
            params["format"] = format
        if architecture:
            params["architecture"] = architecture
        if entry_point:
            params["entry_point"] = entry_point
        if snapshot_path:
            params["snapshot_path"] = snapshot_path
        
        result = await self.tools.rea.open_binary(params)
        self.session_active = True
        self.binary_info = result.get("result")
        return self.binary_info
    
    async def close_binary(self, snapshot_path: Optional[str] = None, overwrite: bool = True) -> Dict[str, Any]:
        """Close the analysis session."""
        params = {"overwrite": overwrite}
        if snapshot_path:
            params["snapshot_path"] = snapshot_path
        
        result = await self.tools.rea.close_binary(params)
        self.session_active = False
        return result.get("result", {})
    
    async def get_binary_overview(self) -> Dict[str, Any]:
        """Get binary overview information."""
        result = await self.tools.rea.binary_overview({})
        return result
    
    async def list_all_functions(self) -> List[Dict[str, Any]]:
        """List all analyzed procedures/functions."""
        result = await self.tools.rea.list_procedures({"detail": "complete"})
        functions = result.get("normalized_result", [])
        self.functions_cache = functions
        return functions
    
    async def list_all_symbols(self) -> List[Dict[str, Any]]:
        """List all symbols/names."""
        result = await self.tools.rea.list_names({"detail": "complete"})
        return result.get("normalized_result", [])
    
    async def list_all_strings(self) -> List[Dict[str, Any]]:
        """List all strings in the binary."""
        result = await self.tools.rea.list_strings({})
        return result.get("normalized_result", [])
    
    async def list_segments(self) -> List[Dict[str, Any]]:
        """List memory segments."""
        result = await self.tools.rea.list_segments({})
        return result.get("normalized_result", [])
    
    async def inspect_exports(self, binary_path: str) -> Dict[str, Any]:
        """Inspect PE/ELF exports."""
        result = await self.tools.rea.inspect_native_exports({
            "path": binary_path,
            "detail": "complete"
        })
        return result.get("normalized_result", {})
    
    async def decompile_all_functions(
        self,
        output_directory: str,
        layout: str = "address",
        resume: bool = False,
        limit: Optional[int] = None
    ) -> Dict[str, Any]:
        """Decompile all functions to files."""
        params = {
            "output_directory": output_directory,
            "layout": layout,
            "resume": resume
        }
        if limit:
            params["limit"] = limit
        
        result = await self.tools.rea.decompile_all(params)
        return result.get("normalized_result", {})
    
    async def analyze_function(self, procedure: str, detail: str = "complete") -> FunctionAnalysis:
        """Deep analyze a single function."""
        result = await self.tools.rea.analyze_function({
            "procedure": procedure,
            "detail": detail
        })
        
        data = result.get("normalized_result", {})
        proc = data.get("procedure", {})
        
        return FunctionAnalysis(
            address=procedure,
            name=proc.get("name", ""),
            signature=proc.get("signature", ""),
            pseudocode=data.get("pseudocode", ""),
            assembly=data.get("assembly", []),
            callers=[c.get("address", "") for c in data.get("callers", [])],
            callees=[c.get("address", "") for c in data.get("callees", [])],
            strings=[s.get("value", "") for s in data.get("referenced_strings", [])],
            variables=proc.get("locals", []),
            basic_blocks=data.get("basic_blocks", []),
            limitations=data.get("limitations", [])
        )
    
    async def rename_function(self, procedure: str, new_name: str, comment: Optional[str] = None) -> bool:
        """Rename a function and optionally add a comment."""
        params = {"procedure": procedure, "name": new_name}
        if comment:
            params["comment"] = comment
        
        result = await self.tools.rea.annotate_native_function(params)
        return result.get("normalized_result", {}).get("effects", {}).get("scope") == "session-analysis-database"
    
    async def rename_symbol(self, address: str, new_name: str) -> bool:
        """Rename a symbol (data or function)."""
        result = await self.tools.rea.set_address_name({
            "address": address,
            "name": new_name
        })
        return result.get("normalized_result", False)
    
    async def batch_rename_symbols(self, names: Dict[str, str]) -> Dict[str, bool]:
        """Batch rename multiple symbols."""
        result = await self.tools.rea.set_addresses_names({"names": names})
        return result.get("normalized_result", {})
    
    async def analyze_api_boundary(self, procedure: str) -> Dict[str, Any]:
        """Analyze native API boundary for a function."""
        result = await self.tools.rea.inspect_native_api({"procedure": procedure})
        return result.get("normalized_result", {})
    
    async def inspect_data_type(self, type_name: Optional[str] = None, address: Optional[str] = None) -> Dict[str, Any]:
        """Inspect a recovered data type."""
        params = {}
        if type_name:
            params["type"] = type_name
        if address:
            params["address"] = address
        
        result = await self.tools.rea.inspect_native_data_type(params)
        return result.get("normalized_result", {})
    
    async def get_call_graph(self) -> Dict[str, Any]:
        """Get the call graph."""
        result = await self.tools.rea.get_call_graph({})
        return result.get("normalized_result", {})
    
    async def set_comment(self, address: str, comment: str) -> bool:
        """Set a regular comment at an address."""
        result = await self.tools.rea.set_comment({"address": address, "comment": comment})
        return result.get("normalized_result", False)
    
    async def set_inline_comment(self, address: str, comment: str) -> bool:
        """Set an inline comment at an address."""
        result = await self.tools.rea.set_inline_comment({"address": address, "comment": comment})
        return result.get("normalized_result", False)
    
    async def set_bookmark(self, address: str, name: Optional[str] = None) -> bool:
        """Set a bookmark at an address."""
        params = {"address": address}
        if name:
            params["name"] = name
        result = await self.tools.rea.set_bookmark(params)
        return result.get("normalized_result", False)
    
    async def find_xrefs_to(self, name: str) -> Dict[str, Any]:
        """Find cross-references to a name."""
        result = await self.tools.rea.find_xrefs_to_name({"name": name})
        return result.get("normalized_result", {})
    
    async def trace_call_path(self, from_proc: str, to_proc: str) -> Dict[str, Any]:
        """Trace call path between two functions."""
        result = await self.tools.rea.trace_call_path({"from_proc": from_proc, "to_proc": to_proc})
        return result.get("normalized_result", {})


# ============================================================
# High-level workflow functions
# ============================================================

async def full_decompilation_workflow(
    tools,
    binary_path: str,
    output_dir: str,
    formats: List[str] = None
) -> Dict[str, Any]:
    """
    Complete decompilation workflow using REA tools directly.
    
    This function can be called from OpenCode code mode:
    
    ```python
    result = await full_decompilation_workflow(tools, "target.exe", "./output")
    ```
    """
    if formats is None:
        formats = ["c", "json", "markdown"]
    
    decompiler = CodeModeDecompiler(tools)
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    results = {
        "binary_path": binary_path,
        "output_dir": output_dir,
        "stages": {}
    }
    
    try:
        # Stage 1: Open binary
        print("Opening binary...")
        binary_info = await decompiler.open_binary(binary_path)
        results["stages"]["open_binary"] = binary_info
        print(f"  Format: {binary_info.get('format')}, Arch: {binary_info.get('architecture')}")
        
        # Stage 2: Binary overview
        print("Getting binary overview...")
        overview = await decompiler.get_binary_overview()
        results["stages"]["overview"] = overview
        
        # Stage 3: List functions
        print("Listing functions...")
        functions = await decompiler.list_all_functions()
        results["stages"]["functions_count"] = len(functions)
        print(f"  Found {len(functions)} functions")
        
        # Stage 4: List symbols
        print("Listing symbols...")
        symbols = await decompiler.list_all_symbols()
        results["stages"]["symbols_count"] = len(symbols)
        
        # Stage 5: List strings
        print("Extracting strings...")
        strings = await decompiler.list_all_strings()
        results["stages"]["strings_count"] = len(strings)
        
        # Stage 6: List segments
        segments = await decompiler.list_segments()
        results["stages"]["segments"] = segments
        
        # Stage 7: Inspect exports
        exports = await decompiler.inspect_exports(binary_path)
        results["stages"]["exports_count"] = len(exports.get("exports", []))
        
        # Stage 8: Decompile all
        print("Decompiling all functions...")
        decompile_dir = output_path / "decompilation"
        decompile_result = await decompiler.decompile_all_functions(str(decompile_dir))
        results["stages"]["decompilation"] = decompile_result
        print(f"  Decompiled: {decompile_result.get('counts', {}).get('written', 0)} functions")
        
        # Stage 9: Deep analyze functions
        print("Analyzing functions...")
        analyses = []
        for func in functions[:20]:  # Limit for demo
            addr = func.get("address", "")
            if addr:
                analysis = await decompiler.analyze_function(addr)
                analyses.append(asdict(analysis))
        results["stages"]["analyzed_functions"] = len(analyses)
        results["analyses"] = analyses
        
        # Stage 10: Export to target formats
        print("Exporting...")
        for fmt in formats:
            export_dir = output_path / f"export_{fmt}"
            export_dir.mkdir(parents=True, exist_ok=True)
            # Export logic would go here
            results["stages"][f"export_{fmt}"] = str(export_dir)
        
        # Stage 11: Generate parity report
        unknown_count = sum(1 for f in functions if f.get("value", "").startswith(("FUN_", "sub_")))
        results["stages"]["parity"] = {
            "total_functions": len(functions),
            "unknown_functions": unknown_count,
            "parity_achieved": unknown_count == 0
        }
        
        # Stage 12: Close session with snapshot
        print("Closing session...")
        snapshot_path = output_path / "snapshot.ghidra"
        await decompiler.close_binary(snapshot_path=str(snapshot_path))
        
        results["success"] = True
        print("Workflow complete!")
        
    except Exception as e:
        results["success"] = False
        results["error"] = str(e)
        print(f"Workflow failed: {e}")
        # Try to close session
        if decompiler.session_active:
            try:
                await decompiler.close_binary()
            except:
                pass
    
    return results


async def iterative_rename_workflow(
    tools,
    project_dir: str,
    confidence_threshold: float = 0.7
) -> Dict[str, Any]:
    """
    Iterative symbol renaming workflow.
    
    This can be run multiple times to progressively improve names.
    """
    decompiler = CodeModeDecompiler(tools)
    project_path = Path(project_dir)
    
    # Load project state
    project_file = project_path / "project_state.json"
    if not project_file.exists():
        return {"success": False, "error": "Project state not found"}
    
    project_state = json.loads(project_file.read_text())
    functions = project_state.get("functions", {})
    symbols = project_state.get("symbols", {})
    
    results = {
        "renamed_functions": 0,
        "renamed_symbols": 0,
        "changes": []
    }
    
    # Find unknown symbols
    unknown_symbols = {
        addr: sym for addr, sym in symbols.items()
        if sym.get("is_unknown", False) and not sym.get("renamed", False)
    }
    
    print(f"Found {len(unknown_symbols)} unknown symbols to rename")
    
    for addr, sym in unknown_symbols.items():
        # Get function context for better naming
        func = functions.get(addr)
        
        # Build context for naming
        context = {
            "address": addr,
            "name": sym["name"],
            "symbol_type": sym["symbol_type"],
        }
        
        if func:
            context.update({
                "pseudocode": func.get("pseudocode", ""),
                "strings": func.get("strings", []),
                "callees": func.get("callees", []),
                "callers": func.get("callers", []),
            })
        
        # Simple pattern-based naming (would use AI in production)
        suggested = suggest_name_from_context(context)
        
        if suggested and suggested != sym["name"]:
            # Apply rename
            if sym["symbol_type"] == "function":
                success = await decompiler.rename_function(addr, suggested)
            else:
                success = await decompiler.rename_symbol(addr, suggested)
            
            if success:
                results["changes"].append({
                    "address": addr,
                    "old_name": sym["name"],
                    "new_name": suggested,
                    "type": sym["symbol_type"]
                })
                if sym["symbol_type"] == "function":
                    results["renamed_functions"] += 1
                else:
                    results["renamed_symbols"] += 1
    
    print(f"Renamed {results['renamed_functions']} functions and {results['renamed_symbols']} symbols")
    return results


def suggest_name_from_context(context: Dict[str, Any]) -> Optional[str]:
    """Simple pattern-based name suggestion."""
    name = context.get("name", "")
    strings = context.get("strings", [])
    callees = context.get("callees", [])
    
    # Check strings for clues
    for s in strings:
        s_lower = s.lower()
        if "error" in s_lower or "fail" in s_lower:
            return "handle_error"
        if "init" in s_lower or "setup" in s_lower:
            return "initialize"
        if "config" in s_lower or "setting" in s_lower:
            return "load_config"
        if "http" in s_lower or "api" in s_lower:
            return "network_request"
        if "malloc" in s_lower or "alloc" in s_lower:
            return "allocate_memory"
    
    # Check callees
    for c in callees:
        c_lower = c.lower()
        if "malloc" in c_lower or "alloc" in c_lower:
            return "allocate_resource"
        if "free" in c_lower:
            return "free_resource"
        if "send" in c_lower or "write" in c_lower:
            return "send_data"
        if "recv" in c_lower or "read" in c_lower:
            return "receive_data"
    
    # Default based on address
    addr = context.get("address", "")
    if addr:
        return f"func_{addr.replace('0x', '')}"
    
    return None


# ============================================================
# Code mode execution example
# ============================================================

async def code_mode_example():
    """
    Example of how to use this in OpenCode code mode.
    
    In OpenCode code mode, you would run:
    
    ```python
    # The tools namespace is automatically available
    result = await full_decompilation_workflow(tools, "malware.exe", "./analysis")
    print(json.dumps(result, indent=2))
    ```
    """
    pass


# ============================================================
# CLI entry point for direct execution
# ============================================================

if __name__ == "__main__":
    print("This module is designed to run within OpenCode's code mode.")
    print("The 'tools' namespace provides access to REA Ghidra tools.")
    print()
    print("Usage in code mode:")
    print("  result = await full_decompilation_workflow(tools, 'target.exe', './output')")
    print()
    print("Or import the classes for custom workflows:")
    print("  from examples.advanced_rea_integration import CodeModeDecompiler")
    print("  decompiler = CodeModeDecompiler(tools)")