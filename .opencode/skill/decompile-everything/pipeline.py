#!/usr/bin/env python3
"""
Pipeline orchestration for decompile-everything.

This module provides the high-level workflow orchestration that coordinates
all the REA Ghidra tools to achieve the complete decompilation pipeline.
"""

import asyncio
import json
import hashlib
import shutil
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Any, Set, Callable
from pathlib import Path
from datetime import datetime
from enum import Enum
import logging

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
from .rea_client import REAGhidraClient, REAToolResult
from .ai_renamer import create_renamer, LLMRenamer, RenameSuggestion

logger = logging.getLogger(__name__)


class PipelineStage(Enum):
    """Pipeline stages for tracking."""
    INIT = "init"
    OPEN_BINARY = "open_binary"
    BINARY_OVERVIEW = "binary_overview"
    LIST_PROCEDURES = "list_procedures"
    LIST_NAMES = "list_names"
    LIST_STRINGS = "list_strings"
    LIST_SEGMENTS = "list_segments"
    INSPECT_EXPORTS = "inspect_exports"
    DECOMPILE_ALL = "decompile_all"
    ANALYZE_FUNCTIONS = "analyze_functions"
    RENAME_SYMBOLS = "rename_symbols"
    EXPORT_DECOMPILATION = "export_decompilation"
    TRACK_PARITY = "track_parity"
    CLOSE_BINARY = "close_binary"
    COMPLETE = "complete"
    ERROR = "error"


@dataclass
class PipelineContext:
    """Context passed through pipeline stages."""
    binary_path: Path
    output_dir: Path
    project_state: Optional[ProjectState] = None
    progress: ProgressTracker = field(default_factory=ProgressTracker)
    rea_client: Optional[REAGhidraClient] = None
    ai_renamer: Optional[LLMRenamer] = None
    config: Dict[str, Any] = field(default_factory=dict)
    stage_results: Dict[str, Any] = field(default_factory=dict)
    errors: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    current_stage: PipelineStage = PipelineStage.INIT
    
    def add_error(self, stage: PipelineStage, error: str):
        self.errors.append(f"[{stage.value}] {error}")
    
    def add_warning(self, stage: PipelineStage, warning: str):
        self.warnings.append(f"[{stage.value}] {warning}")


class PipelineOrchestrator:
    """Orchestrates the complete decompilation pipeline."""
    
    def __init__(self, config: Optional[Dict] = None):
        self.config = config or {}
        self.stages: Dict[PipelineStage, Callable] = {}
        self._register_stages()
    
    def _register_stages(self):
        """Register all pipeline stages."""
        self.stages = {
            PipelineStage.OPEN_BINARY: self._stage_open_binary,
            PipelineStage.BINARY_OVERVIEW: self._stage_binary_overview,
            PipelineStage.LIST_PROCEDURES: self._stage_list_procedures,
            PipelineStage.LIST_NAMES: self._stage_list_names,
            PipelineStage.LIST_STRINGS: self._stage_list_strings,
            PipelineStage.LIST_SEGMENTS: self._stage_list_segments,
            PipelineStage.INSPECT_EXPORTS: self._stage_inspect_exports,
            PipelineStage.DECOMPILE_ALL: self._stage_decompile_all,
            PipelineStage.ANALYZE_FUNCTIONS: self._stage_analyze_functions,
            PipelineStage.RENAME_SYMBOLS: self._stage_rename_symbols,
            PipelineStage.EXPORT_DECOMPILATION: self._stage_export_decompilation,
            PipelineStage.TRACK_PARITY: self._stage_track_parity,
            PipelineStage.CLOSE_BINARY: self._stage_close_binary,
        }
    
    async def run_full_pipeline(
        self,
        binary_path: str,
        output_dir: Optional[str] = None,
        resume: bool = False,
        formats: Optional[List[ExportFormat]] = None,
        rename_mode: RenameMode = RenameMode.AI,
        max_parallel: int = 8
    ) -> ProjectState:
        """Run the complete decompilation pipeline."""
        
        binary_path = Path(binary_path).resolve()
        output_dir = Path(output_dir).resolve() if output_dir else binary_path.parent / f"{binary_path.stem}_decompiled"
        output_dir.mkdir(parents=True, exist_ok=True)
        
        # Initialize context
        ctx = PipelineContext(
            binary_path=binary_path,
            output_dir=output_dir,
            config={
                "max_parallel": max_parallel,
                "formats": formats or [ExportFormat.C, ExportFormat.JSON, ExportFormat.MARKDOWN],
                "rename_mode": rename_mode,
                "resume": resume,
            }
        )
        
        # Initialize REA client
        ctx.rea_client = REAGhidraClient()
        
        # Initialize AI renamer
        ctx.ai_renamer = create_renamer("auto")
        
        # Load existing project if resuming
        project_file = output_dir / "project_state.json"
        if resume and project_file.exists():
            ctx.project_state = await self._load_project_state(project_file)
            logger.info(f"Resumed from existing project: {project_file}")
        else:
            ctx.project_state = ProjectState(
                binary=BinaryInfo(
                    path=str(binary_path),
                    sha256=self._calculate_sha256(binary_path),
                    format="unknown",
                    architecture="unknown",
                )
            )
        
        # Execute pipeline stages
        stages_to_run = [
            PipelineStage.OPEN_BINARY,
            PipelineStage.BINARY_OVERVIEW,
            PipelineStage.LIST_PROCEDURES,
            PipelineStage.LIST_NAMES,
            PipelineStage.LIST_STRINGS,
            PipelineStage.LIST_SEGMENTS,
            PipelineStage.INSPECT_EXPORTS,
            PipelineStage.DECOMPILE_ALL,
            PipelineStage.ANALYZE_FUNCTIONS,
            PipelineStage.RENAME_SYMBOLS,
            PipelineStage.EXPORT_DECOMPILATION,
            PipelineStage.TRACK_PARITY,
            PipelineStage.CLOSE_BINARY,
        ]
        
        for stage in stages_to_run:
            ctx.current_stage = stage
            logger.info(f"Starting stage: {stage.value}")
            
            try:
                await self.stages[stage](ctx)
                logger.info(f"Completed stage: {stage.value}")
            except Exception as e:
                ctx.add_error(stage, str(e))
                logger.exception(f"Stage {stage.value} failed: {e}")
                if stage in [PipelineStage.OPEN_BINARY, PipelineStage.DECOMPILE_ALL]:
                    # Critical stages - abort
                    ctx.current_stage = PipelineStage.ERROR
                    raise
        
        ctx.current_stage = PipelineStage.COMPLETE
        
        # Save final project state
        await self._save_project_state(ctx.project_state, project_file)
        
        return ctx.project_state
    
    # ============================================================
    # Pipeline Stages
    # ============================================================
    
    async def _stage_open_binary(self, ctx: PipelineContext):
        """Open binary with Ghidra."""
        result = await ctx.rea_client.open_binary(
            path=str(ctx.binary_path),
            provider_id="ghidra"
        )
        
        if not result.success:
            raise RuntimeError(f"Failed to open binary: {result.error}")
        
        # Update project state with binary info
        if result.data:
            ctx.project_state.binary.format = result.data.get("format", "pe")
            ctx.project_state.binary.architecture = result.data.get("architecture", "x86_64")
            ctx.project_state.binary.entry_point = result.data.get("entry_point")
        
        ctx.stage_results["open_binary"] = result.data
    
    async def _stage_binary_overview(self, ctx: PipelineContext):
        """Get binary overview."""
        result = await ctx.rea_client.binary_overview()
        
        if result.success and result.data:
            ctx.project_state.binary.format = result.data.get("format", ctx.project_state.binary.format)
            ctx.project_state.binary.architecture = result.data.get("architecture", ctx.project_state.binary.architecture)
            ctx.project_state.binary.entry_point = result.data.get("entry_point", ctx.project_state.binary.entry_point)
        
        ctx.stage_results["binary_overview"] = result.data
    
    async def _stage_list_procedures(self, ctx: PipelineContext):
        """List all procedures."""
        result = await ctx.rea_client.list_procedures(detail="complete")
        
        if result.success and result.data:
            procedures = result.data
            ctx.progress.total_functions = len(procedures)
            
            for proc in procedures:
                addr = proc.get("address", "")
                name = proc.get("value", "")
                is_unknown = self._is_unknown_name(name)
                
                ctx.project_state.functions[addr] = FunctionInfo(
                    address=addr,
                    name=name,
                    size=0,
                )
                
                ctx.project_state.symbols[addr] = SymbolInfo(
                    address=addr,
                    name=name,
                    symbol_type="function",
                    is_unknown=is_unknown,
                    original_name=name,
                )
                
                ctx.progress.total_symbols += 1
                if is_unknown:
                    ctx.progress.unknown_symbols += 1
            
            logger.info(f"Found {len(procedures)} procedures ({ctx.progress.unknown_symbols} unknown)")
        
        ctx.stage_results["list_procedures"] = result.data
    
    async def _stage_list_names(self, ctx: PipelineContext):
        """List all names/symbols."""
        result = await ctx.rea_client.list_names(detail="complete")
        
        if result.success and result.data:
            for name_info in result.data:
                addr = name_info.get("address", "")
                name = name_info.get("value", "")
                is_unknown = self._is_unknown_name(name)
                
                if addr not in ctx.project_state.symbols:
                    ctx.project_state.symbols[addr] = SymbolInfo(
                        address=addr,
                        name=name,
                        symbol_type="data",
                        is_unknown=is_unknown,
                        original_name=name,
                    )
                    ctx.progress.total_symbols += 1
                    if is_unknown:
                        ctx.progress.unknown_symbols += 1
        
        ctx.stage_results["list_names"] = result.data
    
    async def _stage_list_strings(self, ctx: PipelineContext):
        """List all strings."""
        result = await ctx.rea_client.list_strings()
        
        if result.success and result.data:
            ctx.project_state.strings = result.data
            logger.info(f"Found {len(result.data)} strings")
        
        ctx.stage_results["list_strings"] = result.data
    
    async def _stage_list_segments(self, ctx: PipelineContext):
        """List memory segments."""
        result = await ctx.rea_client.list_segments()
        
        if result.success and result.data:
            ctx.project_state.segments = result.data
        
        ctx.stage_results["list_segments"] = result.data
    
    async def _stage_inspect_exports(self, ctx: PipelineContext):
        """Inspect native exports."""
        result = await ctx.rea_client.inspect_native_exports(str(ctx.binary_path))
        
        if result.success and result.data:
            exports = result.data.get("exports", [])
            ctx.project_state.exports = exports
            
            # Mark exported functions as known
            for exp in exports:
                addr = exp.get("address", "")
                if addr in ctx.project_state.symbols:
                    if ctx.project_state.symbols[addr].is_unknown:
                        ctx.project_state.symbols[addr].is_unknown = False
                        ctx.progress.resolved_unknowns += 1
        
        ctx.stage_results["inspect_exports"] = result.data
    
    async def _stage_decompile_all(self, ctx: PipelineContext):
        """Decompile all functions."""
        decompile_dir = ctx.output_dir / "decompilation"
        decompile_dir.mkdir(parents=True, exist_ok=True)
        
        result = await ctx.rea_client.decompile_all(
            output_directory=str(decompile_dir),
            layout="address",
            limit=ctx.config.get("max_parallel", 8)
        )
        
        if result.success and result.data:
            counts = result.data.get("counts", {})
            ctx.progress.decompiled_functions = counts.get("written", 0)
            ctx.project_state.decompilation_complete = True
            logger.info(f"Decompiled {ctx.progress.decompiled_functions} functions")
        
        ctx.stage_results["decompile_all"] = result.data
    
    async def _stage_analyze_functions(self, ctx: PipelineContext):
        """Deep analyze functions in batches."""
        batch_size = ctx.config.get("max_parallel", 8)
        analyzed = 0
        
        # Get functions that need analysis
        functions_to_analyze = [
            (addr, func) for addr, func in ctx.project_state.functions.items()
            if not func.analyzed
        ]
        
        logger.info(f"Analyzing {len(functions_to_analyze)} functions...")
        
        for i in range(0, len(functions_to_analyze), batch_size):
            batch = functions_to_analyze[i:i+batch_size]
            
            # Process batch in parallel
            tasks = []
            for addr, func in batch:
                tasks.append(self._analyze_single_function(ctx, addr, func))
            
            results = await asyncio.gather(*tasks, return_exceptions=True)
            
            for j, (addr, func) in enumerate(batch):
                if isinstance(results[j], Exception):
                    logger.warning(f"Failed to analyze {addr}: {results[j]}")
                else:
                    analyzed += 1
                    ctx.progress.analyzed_functions = analyzed
            
            logger.info(f"  Analyzed {analyzed}/{len(functions_to_analyze)} functions")
        
        ctx.stage_results["analyze_functions"] = {"analyzed": analyzed}
    
    async def _analyze_single_function(self, ctx: PipelineContext, address: str, func: FunctionInfo) -> FunctionInfo:
        """Analyze a single function."""
        result = await ctx.rea_client.analyze_function(address, detail="complete")
        
        if result.success and result.data:
            data = result.data
            proc_data = data.get("procedure", {})
            
            func.signature = proc_data.get("signature", "")
            func.calling_convention = proc_data.get("calling_convention", "")
            func.return_type = proc_data.get("return_type", "")
            func.parameters = proc_data.get("parameters", [])
            
            # Extract pseudocode
            func.pseudocode = data.get("pseudocode", "")
            
            # Extract assembly
            func.assembly = data.get("assembly", [])
            
            # Extract basic blocks
            func.basic_blocks = data.get("basic_blocks", [])
            
            # Extract variables
            locals_data = proc_data.get("locals", [])
            func.variables = [
                SymbolInfo(
                    address=loc.get("stack_displacement", ""),
                    name=loc.get("name", f"var_{loc.get('stack_displacement', 'unknown')}"),
                    symbol_type="variable",
                    is_unknown=True,
                )
                for loc in locals_data
            ]
            
            # Extract callers/callees
            func.callers = [c.get("address", "") for c in data.get("callers", [])]
            func.callees = [c.get("address", "") for c in data.get("callees", [])]
            
            # Extract strings
            func.strings = [s.get("value", "") for s in data.get("referenced_strings", [])]
            
            func.analyzed = True
            
            # Update project state
            ctx.project_state.functions[address] = func
        
        return func
    
    async def _stage_rename_symbols(self, ctx: PipelineContext):
        """Rename symbols using AI."""
        rename_mode = ctx.config.get("rename_mode", RenameMode.AI)
        
        logger.info(f"Renaming symbols (mode: {rename_mode.value})...")
        
        renamed = 0
        symbols_to_rename = [
            (addr, sym) for addr, sym in ctx.project_state.symbols.items()
            if sym.is_unknown and not sym.renamed
        ]
        
        # Process in batches
        batch_size = 50
        for i in range(0, len(symbols_to_rename), batch_size):
            batch = symbols_to_rename[i:i+batch_size]
            
            # Prepare contexts for AI
            contexts = []
            for addr, sym in batch:
                func = ctx.project_state.functions.get(addr)
                contexts.append({
                    "type": "function" if func else "variable",
                    "function": {
                        "address": addr,
                        "name": sym.name,
                        "signature": func.signature if func else "",
                        "pseudocode": func.pseudocode if func else "",
                        "strings": func.strings if func else [],
                        "callees": func.callees if func else [],
                        "callers": func.callers if func else [],
                        "basic_blocks": func.basic_blocks if func else [],
                    } if func else None,
                    "variable": {
                        "address": addr,
                        "name": sym.name,
                        "type": "unknown",
                    } if not func else None,
                    "function_context": {
                        "name": func.name if func else "",
                    } if func else None,
                })
            
            # Get AI suggestions
            suggestions = await ctx.ai_renamer.batch_analyze(contexts)
            
            # Apply renames
            for suggestion in suggestions:
                if suggestion.confidence >= 0.7 and suggestion.suggested_name:
                    addr = suggestion.address
                    
                    # Apply rename via REA
                    if addr in ctx.project_state.functions:
                        # It's a function - use annotate_native_function
                        result = await ctx.rea_client.annotate_native_function(
                            procedure=addr,
                            name=suggestion.suggested_name
                        )
                    else:
                        # It's a data symbol - use set_address_name
                        result = await ctx.rea_client.set_address_name(
                            address=addr,
                            name=suggestion.suggested_name
                        )
                    
                    if result.success:
                        # Update local state
                        sym = ctx.project_state.symbols[addr]
                        sym.name = suggestion.suggested_name
                        sym.suggested_name = suggestion.suggested_name
                        sym.confidence = suggestion.confidence
                        sym.renamed = True
                        
                        if addr in ctx.project_state.functions:
                            ctx.project_state.functions[addr].name = suggestion.suggested_name
                            ctx.project_state.functions[addr].renamed = True
                        
                        renamed += 1
                        ctx.progress.renamed_symbols += 1
                        ctx.progress.resolved_unknowns += 1
                        
                        logger.info(f"  Renamed {addr}: {sym.original_name} -> {suggestion.suggested_name}")
        
        logger.info(f"Renamed {renamed} symbols")
        ctx.stage_results["rename_symbols"] = {"renamed": renamed}
    
    async def _stage_export_decompilation(self, ctx: PipelineContext):
        """Export decompilation to target formats."""
        formats = ctx.config.get("formats", [ExportFormat.C])
        
        for fmt in formats:
            logger.info(f"Exporting to {fmt.value}...")
            
            export_dir = ctx.output_dir / f"export_{fmt.value}"
            export_dir.mkdir(parents=True, exist_ok=True)
            
            if fmt == ExportFormat.C:
                await self._export_c(ctx, export_dir)
            elif fmt == ExportFormat.RUST:
                await self._export_rust(ctx, export_dir)
            elif fmt == ExportFormat.GO:
                await self._export_go(ctx, export_dir)
            elif fmt == ExportFormat.PYTHON:
                await self._export_python(ctx, export_dir)
            elif fmt == ExportFormat.JSON:
                await self._export_json(ctx, export_dir)
            elif fmt == ExportFormat.MARKDOWN:
                await self._export_markdown(ctx, export_dir)
            elif fmt == ExportFormat.ASSEMBLY:
                await self._export_assembly(ctx, export_dir)
        
        ctx.project_state.export_complete = True
        ctx.stage_results["export_decompilation"] = {"formats": [f.value for f in formats]}
    
    async def _export_c(self, ctx: PipelineContext, output_dir: Path):
        """Export as C code."""
        # Header file
        header = output_dir / "decompiled.h"
        header_content = "// Decompiled header file\n"
        header_content += "// Generated by decompile-everything\n\n"
        header_content += "#ifndef DECOMPILED_H\n#define DECOMPILED_H\n\n"
        header_content += "#include <stdint.h>\n#include <stddef.h>\n\n"
        header_content += "// Function declarations\n"
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            sig = func.signature or f"void {func.name}();"
            header_content += f"{sig}\n"
        
        header_content += "\n#endif // DECOMPILED_H\n"
        header.write_text(header_content)
        
        # Source files
        for addr, func in sorted(ctx.project_state.functions.items()):
            if func.pseudocode:
                src_file = output_dir / f"{func.name}.c"
                content = f"""/*
 * Function: {func.name}
 * Address: {addr}
 * Size: {func.size} bytes
 * Calling Convention: {func.calling_convention}
 * Return Type: {func.return_type}
 */

#include "decompiled.h"

{func.pseudocode}
"""
                src_file.write_text(content)
        
        # Types header if we have types
        if ctx.project_state.types:
            types_header = output_dir / "types.h"
            types_content = "// Recovered types\n"
            for name, type_info in ctx.project_state.types.items():
                types_content += f"\n// {name}: {type_info}\n"
            types_header.write_text(types_content)
    
    async def _export_rust(self, ctx: PipelineContext, output_dir: Path):
        """Export as Rust code."""
        lib_rs = output_dir / "lib.rs"
        content = """//! Decompiled Rust module
//! Generated by decompile-everything

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::os::raw::{c_int, c_char, c_void};

// Type definitions

"""
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            rust_sig = func.signature.replace("void ", "").replace("int ", "c_int ").replace("char* ", "*mut c_char ")
            content += f'\nextern "C" {{\n    fn {func.name}({rust_sig});\n}}\n'
        
        lib_rs.write_text(content)
    
    async def _export_go(self, ctx: PipelineContext, output_dir: Path):
        """Export as Go code."""
        main_go = output_dir / "decompiled.go"
        content = """// Decompiled Go package
// Generated by decompile-everything

package decompiled

import "unsafe"

// Function declarations

"""
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            content += f"\n// {func.name} at {addr}\nfunc {func.name}() {{}}\n"
        
        main_go.write_text(content)
    
    async def _export_python(self, ctx: PipelineContext, output_dir: Path):
        """Export as Python ctypes bindings."""
        bindings_py = output_dir / "bindings.py"
        content = """# Decompiled Python bindings
# Generated by decompile-everything

from ctypes import *

# Function bindings

"""
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            content += f"\n# {func.name} at {addr}\n{func.name} = CFUNCTYPE(None)\n"
        
        bindings_py.write_text(content)
    
    async def _export_json(self, ctx: PipelineContext, output_dir: Path):
        """Export as JSON."""
        json_file = output_dir / "decompilation.json"
        data = {
            "binary": asdict(ctx.project_state.binary),
            "functions": {addr: asdict(func) for addr, func in ctx.project_state.functions.items()},
            "symbols": {addr: asdict(sym) for addr, sym in ctx.project_state.symbols.items()},
            "strings": ctx.project_state.strings,
            "segments": ctx.project_state.segments,
            "exports": ctx.project_state.exports,
            "progress": ctx.progress.to_dict(),
        }
        json_file.write_text(json.dumps(data, indent=2, default=str))
    
    async def _export_markdown(self, ctx: PipelineContext, output_dir: Path):
        """Export as Markdown documentation."""
        md_file = output_dir / "DECOMPILATION.md"
        content = f"""# Decompilation Report

**Binary:** {ctx.project_state.binary.path}  
**SHA256:** {ctx.project_state.binary.sha256}  
**Format:** {ctx.project_state.binary.format}  
**Architecture:** {ctx.project_state.binary.architecture}  
**Generated:** {datetime.now().isoformat()}

## Summary

- **Total Functions:** {ctx.progress.total_functions}
- **Analyzed:** {ctx.progress.analyzed_functions} ({ctx.progress.analysis_progress:.1f}%)
- **Decompiled:** {ctx.progress.decompiled_functions} ({ctx.progress.decompilation_progress:.1f}%)
- **Renamed:** {ctx.progress.renamed_functions} ({ctx.progress.rename_progress:.1f}%)
- **Unknown Symbols:** {ctx.progress.unknown_symbols}
- **Resolved Unknowns:** {ctx.progress.resolved_unknowns} ({ctx.progress.unknown_resolution_rate:.1f}%)

## Functions

| Address | Name | Size | Status |
|---------|------|------|--------|
"""
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            status = "✅ Analyzed" if func.analyzed else "⏳ Pending"
            if func.renamed:
                status += " | 🏷️ Renamed"
            content += f"| {addr} | {func.name} | {func.size} | {status} |\n"
        
        content += "\n## Strings\n\n"
        for s in ctx.project_state.strings:
            content += f"- `{s['address']}`: `{s['value']}` ({s['encoding']})\n"
        
        md_file.write_text(content)
    
    async def _export_assembly(self, ctx: PipelineContext, output_dir: Path):
        """Export assembly listings."""
        asm_dir = output_dir / "assembly"
        asm_dir.mkdir(parents=True, exist_ok=True)
        
        for addr, func in sorted(ctx.project_state.functions.items()):
            if func.assembly:
                asm_file = asm_dir / f"{func.name}.asm"
                asm_file.write_text(f"; Assembly for {func.name} at {addr}\n\n" + "\n".join(func.assembly))
    
    async def _stage_track_parity(self, ctx: PipelineContext):
        """Track parity and generate report."""
        report = {
            "timestamp": datetime.now().isoformat(),
            "binary": asdict(ctx.project_state.binary),
            "progress": ctx.progress.to_dict(),
            "parity_check": {
                "total_functions": ctx.progress.total_functions,
                "functions_with_meaningful_names": ctx.progress.total_functions - ctx.progress.unknown_symbols,
                "functions_with_unknown_names": ctx.progress.unknown_symbols,
                "parity_achieved": ctx.progress.unknown_symbols == 0,
            },
            "unknown_symbols": [],
            "renamed_symbols": [],
        }
        
        for addr, symbol in ctx.project_state.symbols.items():
            if symbol.is_unknown:
                report["unknown_symbols"].append({
                    "address": addr,
                    "original_name": symbol.original_name,
                    "current_name": symbol.name,
                    "suggested_name": symbol.suggested_name,
                    "confidence": symbol.confidence,
                })
            elif symbol.renamed:
                report["renamed_symbols"].append({
                    "address": addr,
                    "original_name": symbol.original_name,
                    "new_name": symbol.name,
                })
        
        report_path = ctx.output_dir / "parity_report.json"
        report_path.write_text(json.dumps(report, indent=2, default=str))
        
        logger.info(f"Parity report saved to: {report_path}")
        logger.info(f"Parity achieved: {report['parity_check']['parity_achieved']}")
        
        ctx.stage_results["track_parity"] = report
    
    async def _stage_close_binary(self, ctx: PipelineContext):
        """Close binary session."""
        snapshot_path = ctx.output_dir / "snapshot.ghidra"
        result = await ctx.rea_client.close_binary(snapshot_path=str(snapshot_path), overwrite=True)
        
        if result.success:
            logger.info(f"Session closed, snapshot saved to: {snapshot_path}")
        
        ctx.stage_results["close_binary"] = result.data
    
    # ============================================================
    # Helper Methods
    # ============================================================
    
    def _is_unknown_name(self, name: str) -> bool:
        """Check if name matches unknown patterns."""
        unknown_prefixes = [
            "FUN_", "VAR_", "DAT_", "LAB_", "UNK_",
            "sub_", "loc_", "off_", "ptr_",
            "stru_", "enum_", "type_",
            "field_", "param_", "local_",
        ]
        return any(name.startswith(p) for p in unknown_prefixes)
    
    def _calculate_sha256(self, path: Path) -> str:
        """Calculate SHA256 of file."""
        sha256 = hashlib.sha256()
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(8192), b""):
                sha256.update(chunk)
        return sha256.hexdigest()
    
    async def _save_project_state(self, project: ProjectState, path: Path):
        """Save project state to file."""
        path.write_text(json.dumps(asdict(project), indent=2, default=str))
        logger.info(f"Project state saved to: {path}")
    
    async def _load_project_state(self, path: Path) -> ProjectState:
        """Load project state from file."""
        data = json.loads(path.read_text())
        return ProjectState(**data)


# ============================================================
# High-level API
# ============================================================

async def decompile_everything(
    binary_path: str,
    output_dir: Optional[str] = None,
    **kwargs
) -> ProjectState:
    """
    High-level function to run complete decompilation pipeline.
    
    Args:
        binary_path: Path to binary file
        output_dir: Output directory (default: <binary>_decompiled)
        **kwargs: Additional configuration options
    
    Returns:
        ProjectState with complete decompilation results
    """
    orchestrator = PipelineOrchestrator(kwargs)
    return await orchestrator.run_full_pipeline(binary_path, output_dir, **kwargs)


async def quick_analyze(binary_path: str) -> Dict[str, Any]:
    """Quick analysis without full decompilation."""
    client = REAGhidraClient()
    
    await client.open_binary(binary_path)
    overview = await client.binary_overview()
    procedures = await client.list_procedures()
    names = await client.list_names()
    strings = await client.list_strings()
    exports = await client.inspect_native_exports(binary_path)
    await client.close_binary()
    
    return {
        "overview": overview.data,
        "procedures_count": len(procedures.data) if procedures.success else 0,
        "symbols_count": len(names.data) if names.success else 0,
        "strings_count": len(strings.data) if strings.success else 0,
        "exports_count": len(exports.data.get("exports", [])) if exports.success else 0,
    }


# Example usage
async def main():
    """Example usage."""
    # Full pipeline
    project = await decompile_everything(
        "target.exe",
        output_dir="./output",
        formats=[ExportFormat.C, ExportFormat.RUST, ExportFormat.JSON],
        rename_mode=RenameMode.AI,
        max_parallel=8
    )
    
    print(f"Complete! Functions: {len(project.functions)}")
    print(f"Renamed: {sum(1 for f in project.functions.values() if f.renamed)}")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())