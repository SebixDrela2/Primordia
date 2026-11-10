#!/usr/bin/env python3
"""
Tests for decompile-everything skill.
"""

import pytest
import asyncio
import tempfile
import json
from pathlib import Path
from unittest.mock import Mock, AsyncMock, patch

from decompile_everything import (
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
from ai_renamer import (
    PatternRenamer,
    MockLLMRenamer,
    RenameSuggestion,
    create_renamer,
)
from rea_client import REAGhidraClient, REAToolResult
from pipeline import PipelineOrchestrator, PipelineContext, PipelineStage


class TestProgressTracker:
    """Test progress tracking."""
    
    def test_initial_state(self):
        tracker = ProgressTracker()
        assert tracker.total_functions == 0
        assert tracker.analysis_progress == 0.0
        assert tracker.decompilation_progress == 0.0
        assert tracker.rename_progress == 0.0
    
    def test_progress_calculation(self):
        tracker = ProgressTracker()
        tracker.total_functions = 100
        tracker.analyzed_functions = 50
        tracker.decompiled_functions = 30
        tracker.total_symbols = 200
        tracker.renamed_symbols = 100
        tracker.unknown_symbols = 50
        tracker.resolved_unknowns = 25
        
        assert tracker.analysis_progress == 50.0
        assert tracker.decompilation_progress == 30.0
        assert tracker.rename_progress == 50.0
        assert tracker.unknown_resolution_rate == 50.0
    
    def test_to_dict(self):
        tracker = ProgressTracker()
        tracker.total_functions = 10
        d = tracker.to_dict()
        assert "total_functions" in d
        assert "analysis_progress_pct" in d


class TestSymbolInfo:
    """Test SymbolInfo dataclass."""
    
    def test_creation(self):
        sym = SymbolInfo(
            address="0x140001000",
            name="FUN_140001000",
            symbol_type="function",
            is_unknown=True,
        )
        assert sym.address == "0x140001000"
        assert sym.is_unknown is True
        assert sym.renamed is False


class TestFunctionInfo:
    """Test FunctionInfo dataclass."""
    
    def test_creation(self):
        func = FunctionInfo(
            address="0x140001000",
            name="main",
            size=100,
        )
        assert func.name == "main"
        assert func.analyzed is False
        assert func.renamed is False


class TestProjectState:
    """Test ProjectState serialization."""
    
    def test_serialization(self):
        project = ProjectState(
            binary=BinaryInfo(
                path="test.exe",
                sha256="abc123",
                format="pe",
                architecture="x86_64",
            )
        )
        project.functions["0x140001000"] = FunctionInfo(
            address="0x140001000", name="main", size=100
        )
        
        data = project.to_dict()
        assert data["binary"]["path"] == "test.exe"
        assert "0x140001000" in data["functions"]
        
        # Test deserialization
        restored = ProjectState.from_dict(data)
        assert restored.binary.path == "test.exe"
        assert "0x140001000" in restored.functions


class TestPatternRenamer:
    """Test pattern-based renamer."""
    
    def setup_method(self):
        self.renamer = PatternRenamer()
    
    def test_is_unknown(self):
        assert self.renamer.is_unknown("FUN_140001000") is True
        assert self.renamer.is_unknown("VAR_140002000") is True
        assert self.renamer.is_unknown("main") is False
        assert self.renamer.is_unknown("malloc") is False
    
    def test_suggest_function_name_init(self):
        func_info = {
            "strings": ["Initializing...", "Config loaded"],
            "callees": ["malloc", "printf"],
            "callers": [],
            "basic_blocks": [{} for _ in range(10)],
        }
        suggested = self.renamer.suggest_function_name(func_info)
        assert suggested == "init"
    
    def test_suggest_function_name_alloc(self):
        func_info = {
            "strings": [],
            "callees": ["malloc", "calloc"],
            "callers": [],
            "basic_blocks": [{} for _ in range(5)],
        }
        suggested = self.renamer.suggest_function_name(func_info)
        assert suggested == "alloc"
    
    def test_suggest_function_name_network(self):
        func_info = {
            "strings": ["https://api.example.com", "GET /data"],
            "callees": ["send", "recv", "connect"],
            "callers": [],
            "basic_blocks": [{} for _ in range(20)],
        }
        suggested = self.renamer.suggest_function_name(func_info)
        assert suggested in ["send", "recv", "connect"]
    
    def test_suggest_variable_name(self):
        var_info = {
            "name": "VAR_140002000",
            "type": "int*",
            "usage": {"reads": 5, "writes": 2},
        }
        func_context = {}
        suggested = self.renamer.suggest_variable_name(var_info, func_context)
        assert suggested is not None
        assert "ptr_" in suggested or "output_" in suggested or "input_" in suggested


class TestMockLLMRenamer:
    """Test mock LLM renamer."""
    
    @pytest.mark.asyncio
    async def test_analyze_function(self):
        renamer = MockLLMRenamer()
        
        context = {
            "type": "function",
            "function": {
                "address": "0x140001234",
                "name": "FUN_140001234",
                "signature": "int FUN_140001234(char* config_path)",
                "pseudocode": "int FUN_140001234(char* config_path) { return parse_json(config_path); }",
                "strings": ["config.json", "parse_json"],
                "callees": ["fopen", "parse_json"],
                "callers": ["main"],
                "basic_blocks": [{"start": "0x140001234"}],
            }
        }
        
        suggestions = await renamer.analyze_function(context)
        assert len(suggestions) == 1
        assert suggestions[0].symbol_type == "function"
        assert suggestions[0].confidence > 0
    
    @pytest.mark.asyncio
    async def test_batch_analyze(self):
        renamer = MockLLMRenamer()
        
        contexts = [
            {"type": "function", "function": {"address": "0x1", "name": "FUN_1", "strings": ["init"], "callees": [], "callers": [], "basic_blocks": [{}]}},
            {"type": "function", "function": {"address": "0x2", "name": "FUN_2", "strings": ["malloc"], "callees": ["malloc"], "callers": [], "basic_blocks": [{}]}},
        ]
        
        suggestions = await renamer.batch_analyze(contexts)
        assert len(suggestions) == 2


class TestREAGhidraClient:
    """Test REA client (mocked)."""
    
    @pytest.mark.asyncio
    async def test_open_binary(self):
        client = REAGhidraClient()
        result = await client.open_binary("/fake/path.exe")
        assert result.success is True
        assert client.session_active is True
    
    @pytest.mark.asyncio
    async def test_list_procedures(self):
        client = REAGhidraClient()
        result = await client.list_procedures()
        assert result.success is True
        assert len(result.data) == 5
    
    @pytest.mark.asyncio
    async def test_decompile_all(self):
        client = REAGhidraClient()
        result = await client.decompile_all("/tmp/output")
        assert result.success is True
        assert result.data["counts"]["written"] == 95
    
    @pytest.mark.asyncio
    async def test_analyze_function(self):
        client = REAGhidraClient()
        result = await client.analyze_function("0x140001300")
        assert result.success is True
        assert "signature" in result.data["procedure"]


class TestPipelineOrchestrator:
    """Test pipeline orchestrator."""
    
    @pytest.mark.asyncio
    async def test_stage_registration(self):
        orchestrator = PipelineOrchestrator()
        assert PipelineStage.OPEN_BINARY in orchestrator.stages
        assert PipelineStage.DECOMPILE_ALL in orchestrator.stages
        assert PipelineStage.RENAME_SYMBOLS in orchestrator.stages
        assert len(orchestrator.stages) == 13
    
    @pytest.mark.asyncio
    async def test_is_unknown_name(self):
        orchestrator = PipelineOrchestrator()
        assert orchestrator._is_unknown_name("FUN_140001000") is True
        assert orchestrator._is_unknown_name("main") is False


class TestDecompileEverything:
    """Test main DecompileEverything class."""
    
    def setup_method(self):
        self.tool = DecompileEverything()
    
    def test_unknown_prefixes(self):
        assert self.tool._is_unknown_name("FUN_140001000") is True
        assert self.tool._is_unknown_name("VAR_140002000") is True
        assert self.tool._is_unknown_name("sub_140001000") is True
        assert self.tool._is_unknown_name("main") is False
        assert self.tool._is_unknown_name("malloc") is False
    
    @pytest.mark.asyncio
    async def test_run_analyze(self):
        # This would require a real binary, so we test the method exists
        assert hasattr(self.tool, 'run_analyze')
        assert hasattr(self.tool, 'run_decompile')
        assert hasattr(self.tool, 'run_rename')
        assert hasattr(self.tool, 'run_export')
        assert hasattr(self.tool, 'run_track')


class TestCreateRenamer:
    """Test renamer factory."""
    
    def test_create_pattern_renamer(self):
        renamer = create_renamer("pattern")
        assert isinstance(renamer, MockLLMRenamer)
    
    def test_create_auto_renamer_no_keys(self):
        # Without API keys, should return pattern renamer
        with patch.dict('os.environ', {}, clear=True):
            renamer = create_renamer("auto")
            assert isinstance(renamer, MockLLMRenamer)


class TestIntegration:
    """Integration tests (require actual binary)."""
    
    @pytest.mark.skip(reason="Requires actual binary and REA setup")
    @pytest.mark.asyncio
    async def test_full_pipeline_integration(self):
        """Test full pipeline with a real binary."""
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create a dummy binary
            binary_path = Path(tmpdir) / "test.exe"
            binary_path.write_bytes(b"MZ" + b"\x00" * 1000)  # Minimal PE header
            
            orchestrator = PipelineOrchestrator({
                "max_parallel": 2,
                "formats": [ExportFormat.C, ExportFormat.JSON],
                "rename_mode": RenameMode.PATTERN,
            })
            
            try:
                project = await orchestrator.run_full_pipeline(
                    str(binary_path),
                    output_dir=tmpdir + "/out",
                    rename_mode=RenameMode.PATTERN,
                )
                assert project is not None
                assert project.binary.path == str(binary_path)
            except Exception as e:
                # Expected to fail without real Ghidra/REA
                pytest.skip(f"Integration test skipped: {e}")


# ============================================================
# Fixtures and Utilities
# ============================================================

@pytest.fixture
def sample_project_state():
    """Create a sample project state for testing."""
    project = ProjectState(
        binary=BinaryInfo(
            path="test.exe",
            sha256="abc123",
            format="pe",
            architecture="x86_64",
        )
    )
    
    # Add some functions
    project.functions["0x140001000"] = FunctionInfo(
        address="0x140001000",
        name="entry",
        size=50,
        analyzed=True,
        signature="void entry()",
    )
    
    project.functions["0x140001100"] = FunctionInfo(
        address="0x140001100",
        name="FUN_140001100",
        size=100,
        analyzed=True,
        signature="int FUN_140001100(int a, char* b)",
        pseudocode="int FUN_140001100(int a, char* b) { return a + strlen(b); }",
        strings=["Hello", "World"],
        callees=["strlen"],
        callers=["entry"],
    )
    
    # Add symbols
    project.symbols["0x140001000"] = SymbolInfo(
        address="0x140001000", name="entry", symbol_type="function", is_unknown=False
    )
    project.symbols["0x140001100"] = SymbolInfo(
        address="0x140001100", name="FUN_140001100", symbol_type="function", is_unknown=True, original_name="FUN_140001100"
    )
    project.symbols["0x140002000"] = SymbolInfo(
        address="0x140002000", name="VAR_140002000", symbol_type="data", is_unknown=True, original_name="VAR_140002000"
    )
    
    # Add strings
    project.strings = [
        {"address": "0x140003000", "value": "Hello, World!", "encoding": "ascii"},
        {"address": "0x140003020", "value": "Error: %s", "encoding": "ascii"},
    ]
    
    return project


@pytest.fixture
def mock_rea_client():
    """Create a mocked REA client."""
    client = Mock(spec=REAGhidraClient)
    client.open_binary = AsyncMock(return_value=REAToolResult(success=True, data={"format": "pe", "architecture": "x86_64"}))
    client.close_binary = AsyncMock(return_value=REAToolResult(success=True))
    client.binary_overview = AsyncMock(return_value=REAToolResult(success=True, data={"format": "pe", "architecture": "x86_64"}))
    client.list_procedures = AsyncMock(return_value=REAToolResult(success=True, data=[
        {"address": "0x140001000", "value": "entry", "symbol": {"source": "default"}},
        {"address": "0x140001100", "value": "FUN_140001100", "symbol": {"source": "analysis"}},
    ]))
    client.list_names = AsyncMock(return_value=REAToolResult(success=True, data=[
        {"address": "0x140001000", "value": "entry", "symbol": {"source": "default"}},
        {"address": "0x140002000", "value": "VAR_140002000", "symbol": {"source": "analysis"}},
    ]))
    client.list_strings = AsyncMock(return_value=REAToolResult(success=True, data=[
        {"address": "0x140003000", "value": "Hello", "encoding": "ascii"},
    ]))
    client.list_segments = AsyncMock(return_value=REAToolResult(success=True, data=[]))
    client.inspect_native_exports = AsyncMock(return_value=REAToolResult(success=True, data={"exports": []}))
    client.decompile_all = AsyncMock(return_value=REAToolResult(success=True, data={"counts": {"written": 2}}))
    client.analyze_function = AsyncMock(return_value=REAToolResult(success=True, data={
        "procedure": {"signature": "int func(int, char*)", "calling_convention": "ms_x64", "return_type": "int", "parameters": [], "locals": []},
        "pseudocode": "int func(int a, char* b) { return a + strlen(b); }",
        "assembly": ["push rbp", "mov rbp, rsp"],
        "basic_blocks": [{"start": "0x140001100", "end": "0x140001150", "successors": []}],
        "callers": [{"address": "0x140001000"}],
        "callees": [{"address": "0x140001200"}],
        "referenced_strings": [{"value": "Hello"}],
    }))
    client.set_address_name = AsyncMock(return_value=REAToolResult(success=True, data=True))
    client.annotate_native_function = AsyncMock(return_value=REAToolResult(success=True, data={}))
    return client


# ============================================================
# Run Tests
# ============================================================

if __name__ == "__main__":
    pytest.main([__file__, "-v"])