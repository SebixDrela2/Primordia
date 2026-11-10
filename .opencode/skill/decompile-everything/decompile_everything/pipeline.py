#!/usr/bin/env python3
"""
Code-mode entry point for decompile-everything skill.

When invoked from OpenCode's code mode, this receives the `tools` namespace
and uses actual REA Ghidra tools for real decompilation.

Usage in code mode:
    result = await tools.skill.decompile_everything.run_pipeline_with_tools(
        tools,
        binary_path="D:\\target.exe",
        output_dir="./analysis",
        modes=["full"]
    )
"""

import asyncio
import json
import hashlib
import shutil
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Any, Set
from pathlib import Path
from datetime import datetime
from enum import Enum
import logging

logger = logging.getLogger(__name__)


class ExportFormat(Enum):
    C = "c"
    RUST = "rust"
    GO = "go"
    PYTHON = "python"
    JSON = "json"
    MARKDOWN = "markdown"
    ASSEMBLY = "assembly"


class RenameMode(Enum):
    AI = "ai"
    PATTERN = "pattern"
    MANUAL = "manual"


@dataclass
class SymbolInfo:
    address: str
    name: str
    symbol_type: str
    is_unknown: bool
    original_name: str = ""
    suggested_name: str = ""
    confidence: float = 0.0
    context: Dict[str, Any] = field(default_factory=dict)
    renamed: bool = False


@dataclass
class FunctionInfo:
    address: str
    name: str
    size: int
    pseudocode: str = ""
    assembly: List[str] = field(default_factory=list)
    callers: List[str] = field(default_factory=list)
    callees: List[str] = field(default_factory=list)
    strings: List[str] = field(default_factory=list)
    variables: List[SymbolInfo] = field(default_factory=list)
    signature: str = ""
    calling_convention: str = ""
    return_type: str = ""
    parameters: List[Dict[str, Any]] = field(default_factory=list)
    basic_blocks: List[Dict[str, Any]] = field(default_factory=list)
    limitations: List[str] = field(default_factory=list)
    analyzed: bool = False
    renamed: bool = False


@dataclass
class BinaryInfo:
    path: str
    sha256: str
    format: str
    architecture: str
    entry_point: Optional[str] = None
    provider: str = "ghidra"
    analysis_profile_digest: Optional[str] = None


@dataclass
class ProjectState:
    binary: BinaryInfo
    functions: Dict[str, FunctionInfo] = field(default_factory=dict)
    symbols: Dict[str, SymbolInfo] = field(default_factory=dict)
    strings: List[Dict[str, Any]] = field(default_factory=list)
    segments: List[Dict[str, Any]] = field(default_factory=list)
    exports: List[Dict[str, Any]] = field(default_factory=list)
    imports: List[Dict[str, Any]] = field(default_factory=list)
    call_graph: Dict[str, List[str]] = field(default_factory=dict)
    types: Dict[str, Any] = field(default_factory=dict)
    analysis_complete: bool = False
    decompilation_complete: bool = False
    rename_complete: bool = False
    export_complete: bool = False
    session_id: Optional[str] = None
    evidence_ids: Dict[str, str] = field(default_factory=dict)
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    updated_at: str = field(default_factory=lambda: datetime.now().isoformat())

    def to_dict(self) -> Dict:
        return asdict(self)


@dataclass
class ProgressTracker:
    total_functions: int = 0
    analyzed_functions: int = 0
    decompiled_functions: int = 0
    renamed_functions: int = 0
    total_symbols: int = 0
    renamed_symbols: int = 0
    unknown_symbols: int = 0
    resolved_unknowns: int = 0
    start_time: float = field(default_factory=lambda: __import__('time').time())
    errors: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)

    @property
    def analysis_progress(self) -> float:
        if self.total_functions == 0:
            return 0.0
        return (self.analyzed_functions / self.total_functions) * 100

    @property
    def decompilation_progress(self) -> float:
        if self.total_functions == 0:
            return 0.0
        return (self.decompiled_functions / self.total_functions) * 100

    @property
    def rename_progress(self) -> float:
        if self.total_symbols == 0:
            return 0.0
        return (self.renamed_symbols / self.total_symbols) * 100

    @property
    def unknown_resolution_rate(self) -> float:
        if self.unknown_symbols == 0:
            return 100.0
        return (self.resolved_unknowns / self.unknown_symbols) * 100

    @property
    def elapsed_time(self) -> float:
        return __import__('time').time() - self.start_time

    def to_dict(self) -> Dict:
        return {
            "total_functions": self.total_functions,
            "analyzed_functions": self.analyzed_functions,
            "decompiled_functions": self.decompiled_functions,
            "renamed_functions": self.renamed_functions,
            "total_symbols": self.total_symbols,
            "renamed_symbols": self.renamed_symbols,
            "unknown_symbols": self.unknown_symbols,
            "resolved_unknowns": self.resolved_unknowns,
            "analysis_progress_pct": round(self.analysis_progress, 2),
            "decompilation_progress_pct": round(self.decompilation_progress, 2),
            "rename_progress_pct": round(self.rename_progress, 2),
            "unknown_resolution_rate_pct": round(self.unknown_resolution_rate, 2),
            "elapsed_time_sec": round(self.elapsed_time, 2),
            "errors": self.errors,
            "warnings": self.warnings,
        }


class REAToolsClient:
    """Client that uses actual REA tools from OpenCode's code mode."""
    
    def __init__(self, tools):
        self.tools = tools
        self.session_active = False
        self.current_binary = None
    
    async def open_binary(self, path: str, provider_id: str = "ghidra", **kwargs) -> Dict:
        result = await self.tools.rea.open_binary({
            "path": path,
            "provider_id": provider_id,
            **kwargs
        })
        self.session_active = True
        self.current_binary = result.get("result")
        return self.current_binary
    
    async def close_binary(self, snapshot_path: Optional[str] = None, overwrite: bool = True) -> Dict:
        params = {"overwrite": overwrite}
        if snapshot_path:
            params["snapshot_path"] = snapshot_path
        result = await self.tools.rea.close_binary(params)
        self.session_active = False
        return result.get("result", {})
    
    async def binary_overview(self) -> Dict:
        result = await self.tools.rea.binary_overview({})
        return result.get("normalized_result", result)
    
    async def list_procedures(self, detail: str = "complete") -> List[Dict]:
        result = await self.tools.rea.list_procedures({"detail": detail})
        return result.get("normalized_result", [])
    
    async def list_names(self, detail: str = "summary") -> List[Dict]:
        result = await self.tools.rea.list_names({"detail": detail})
        return result.get("normalized_result", [])
    
    async def list_strings(self, detail: str = "summary") -> List[Dict]:
        result = await self.tools.rea.list_strings({"detail": detail})
        return result.get("normalized_result", [])
    
    async def list_segments(self, detail: str = "summary") -> List[Dict]:
        result = await self.tools.rea.list_segments({"detail": detail})
        return result.get("normalized_result", [])
    
    async def inspect_native_exports(self, path: str, detail: str = "summary") -> Dict:
        result = await self.tools.rea.inspect_native_exports({
            "path": path,
            "detail": detail
        })
        return result.get("normalized_result", {})
    
    async def decompile_all(self, output_directory: str, layout: str = "address") -> Dict:
        result = await self.tools.rea.decompile_all({
            "output_directory": output_directory,
            "layout": layout
        })
        return result.get("normalized_result", {})
    
    async def analyze_function(self, procedure: str, detail: str = "complete") -> Dict:
        result = await self.tools.rea.analyze_function({
            "procedure": procedure,
            "detail": detail
        })
        return result.get("normalized_result", {})
    
    async def get_call_graph(self, address: str, direction: str = "forward", detail: str = "summary") -> Dict:
        result = await self.tools.rea.get_call_graph({
            "address": address,
            "direction": direction,
            "detail": detail
        })
        return result.get("normalized_result", {})
    
    async def set_address_name(self, address: str, name: str) -> bool:
        result = await self.tools.rea.set_address_name({"address": address, "name": name})
        return result.get("normalized_result", False)
    
    async def set_addresses_names(self, names: Dict[str, str]) -> Dict[str, bool]:
        result = await self.tools.rea.set_addresses_names({"names": names})
        return result.get("normalized_result", {})
    
    async def annotate_native_function(self, procedure: str, name: Optional[str] = None, comment: Optional[str] = None) -> Dict:
        params = {"procedure": procedure}
        if name:
            params["name"] = name
        if comment:
            params["comment"] = comment
        result = await self.tools.rea.annotate_native_function(params)
        return result.get("normalized_result", {})
    
    async def inspect_native_api(self, procedure: str) -> Dict:
        result = await self.tools.rea.inspect_native_api({"procedure": procedure})
        return result.get("normalized_result", {})
    
    async def inspect_native_data_type(self, type: Optional[str] = None, address: Optional[str] = None) -> Dict:
        params = {}
        if type:
            params["type"] = type
        if address:
            params["address"] = address
        result = await self.tools.rea.inspect_native_data_type(params)
        return result.get("normalized_result", {})


class IntelligentRenamer:
    """
    Intelligent symbol renamer based on deep Ghidra analysis.
    
    Uses pseudocode, call graphs, strings, API patterns, and control flow
    to infer meaningful names for functions, variables, and types.
    """
    
    # Comprehensive function patterns based on actual reverse engineering
    FUNCTION_PATTERNS = {
        # Initialization/Startup
        "initialize": ["initialize", "init", "startup", "bootstrap", "construct", "ctor_"],
        "cleanup": ["cleanup", "destroy", "destruct", "dtor_", "shutdown", "finalize", "uninitialize"],
        "entry": ["entry", "main", "WinMain", "DllMain", "_start", "__start"],
        
        # Memory Management
        "allocate": ["allocate", "alloc", "malloc", "calloc", "realloc", "new", "create_", "createobject"],
        "free": ["free", "delete", "deallocate", "dealloc", "release", "destroy_", "destroyobject"],
        "memory_copy": ["memcpy", "memmove", "copy_", "strcpy", "strncpy", "wcscpy", "wcsncpy"],
        "memory_set": ["memset", "zeromemory", "fillmemory", "fill_"],
        
        # String Operations
        "string_format": ["sprintf", "snprintf", "vsprintf", "format_", "printf", "formatmessage"],
        "string_compare": ["strcmp", "strncmp", "strcmpi", "stricmp", "wcscmp", "wcsncmp", "compare_"],
        "string_search": ["strstr", "strchr", "strrchr", "find_", "search_", "indexof"],
        "string_concat": ["strcat", "strncat", "wcscat", "wcsncat", "concat_", "append_"],
        "string_length": ["strlen", "wcslen", "strlen_", "length_"],
        
        # File I/O
        "file_open": ["fopen", "open", "createfile", "open_", "create_"],
        "file_read": ["fread", "read", "readfile", "read_"],
        "file_write": ["fwrite", "write", "writefile", "write_"],
        "file_close": ["fclose", "close", "closehandle", "close_"],
        "file_seek": ["fseek", "seek", "setfilepointer", "seek_"],
        
        # Network
        "net_connect": ["connect", "socket", "accept", "bind", "listen", "connect_"],
        "net_send": ["send", "write", "sendto", "wsasend", "send_", "write_", "transmit"],
        "net_recv": ["recv", "read", "recvfrom", "wsarecv", "recv_", "read_", "receive"],
        "net_http": ["http", "curl", "winhttp", "url", "download", "upload", "request", "response"],
        
        # Registry
        "reg_open": ["regopenkey", "regcreatekey", "openkey", "open_"],
        "reg_read": ["regqueryvalue", "regetvalue", "query_", "read_", "get_"],
        "reg_write": ["regsetvalue", "regsetkey", "set_", "write_", "put_"],
        "reg_delete": ["regdeletekey", "regdeletevalue", "delete_", "remove_"],
        
        # Process/Thread
        "proc_create": ["createprocess", "createthread", "spawn", "fork", "exec", "launch"],
        "proc_wait": ["wait", "waitfor", "join", "waitfor_"],
        "proc_kill": ["terminateprocess", "kill", "exit", "terminate_"],
        "thread_sync": ["mutex", "critical_section", "event", "semaphore", "lock", "unlock", "acquire", "release"],
        
        # Crypto
        "crypto_hash": ["hash", "md5", "sha1", "sha256", "sha512", "crc", "checksum", "hmac"],
        "crypto_encrypt": ["encrypt", "cipher", "aes", "des", "rc4", "encrypt_", "encode"],
        "crypto_decrypt": ["decrypt", "decipher", "decrypt_", "decode"],
        "crypto_sign": ["sign", "signature", "verify", "rsa", "dsa", "ecdsa"],
        
        # Compression
        "compress": ["compress", "deflate", "zip", "gzip", "zlib", "lzma", "compress_"],
        "decompress": ["decompress", "inflate", "unzip", "gunzip", "decompress_"],
        
        # Parsing/Serialization
        "parse_json": ["json", "parse", "deserialize", "unmarshal", "load_"],
        "serialize_json": ["serialize", "marshal", "dump", "save_", "write_"],
        "parse_xml": ["xml", "parse", "dom", "sax", "xpath"],
        "parse_config": ["config", "ini", "registry", "settings", "preferences", "options"],
        
        # Error/Logging
        "error_handle": ["error", "err_", "exception", "throw", "raise", "handle_", "check_"],
        "logging": ["log", "debug", "trace", "printf", "outputdebug", "writelog", "log_"],
        
        # UI/Windows
        "window_create": ["createwindow", "createdialog", "create_", "init_"],
        "window_msg": ["sendmessage", "postmessage", "dispatch", "wndproc", "handle_"],
        "gui_update": ["update", "redraw", "invalidate", "paint", "draw", "render", "refresh"],
        
        # Data Processing
        "sort": ["sort", "qsort", "mergesort", "sort_"],
        "filter": ["filter", "find", "match", "where", "select_"],
        "transform": ["transform", "convert", "encode", "decode", "translate", "map_"],
        "validate": ["validate", "check", "verify", "isvalid", "assert"],
        
        # Math
        "math_random": ["rand", "random", "srand", "uuid", "guid", "generate_"],
        "math_calc": ["calculate", "compute", "eval", "eval_", "formula"],
        
        # Container/Collection
        "list_add": ["add", "push", "insert", "append", "push_back", "add_"],
        "list_remove": ["remove", "erase", "delete", "pop", "clear", "remove_"],
        "list_find": ["find", "search", "contains", "indexof", "find_"],
        "list_iter": ["iterate", "foreach", "enum", "next", "first", "last"],
    }
    
    VARIABLE_PATTERNS = {
        "counter": ["count", "cnt", "num", "index", "idx", "i", "j", "k", "pos", "offset"],
        "size": ["size", "len", "length", "sz", "cb", "dwSize", "capacity", "max"],
        "pointer": ["ptr", "p", "pp", "lp", "address", "addr", "ref", "handle"],
        "handle": ["handle", "h", "hndl", "fd", "socket", "sock", "hfile", "hkey"],
        "buffer": ["buf", "buffer", "data", "payload", "msg", "message", "chunk", "block"],
        "flag": ["flag", "flg", "enabled", "active", "valid", "ready", "done", "complete", "success", "error"],
        "config": ["config", "cfg", "settings", "options", "params", "args", "props", "properties"],
        "result": ["result", "res", "ret", "retval", "return", "status", "rc", "hr", "hresult"],
        "error": ["error", "err", "errno", "last_error", "err_code", "errno"],
        "timestamp": ["time", "timestamp", "ts", "tick", "clock", "date", "utc", "local"],
        "id": ["id", "uid", "gid", "pid", "tid", "handle", "key", "token"],
        "version": ["version", "ver", "major", "minor", "build", "revision"],
        "name": ["name", "title", "label", "caption", "text", "str"],
        "path": ["path", "dir", "folder", "file", "filename", "fullpath", "relpath"],
        "url": ["url", "uri", "link", "endpoint", "address"],
        "key": ["key", "secret", "password", "passwd", "pwd", "token", "auth"],
        "temp": ["temp", "tmp", "scratch", "work", "aux"],
    }
    
    # API function signatures for pattern matching
    API_SIGNATURES = {
        # Windows API
        "kernel32": ["CreateFile", "ReadFile", "WriteFile", "CloseHandle", "CreateThread", "WaitForSingleObject",
                     "VirtualAlloc", "VirtualFree", "GetLastError", "SetLastError", "GetCurrentProcess",
                     "GetCurrentThread", "Sleep", "GetTickCount", "GetSystemTime", "GetLocalTime",
                     "CreateFile", "DeleteFile", "MoveFile", "CopyFile", "CreateDirectory", "RemoveDirectory",
                     "FindFirstFile", "FindNextFile", "FindClose", "GetFileAttributes", "SetFileAttributes"],
        "user32": ["CreateWindow", "DestroyWindow", "ShowWindow", "UpdateWindow", "GetMessage", "DispatchMessage",
                   "SendMessage", "PostMessage", "DefWindowProc", "RegisterClass", "LoadIcon", "LoadCursor",
                   "GetDlgItem", "SetDlgItemText", "GetWindowText", "SetWindowText", "EnableWindow",
                   "ShowWindow", "UpdateWindow", "InvalidateRect", "BeginPaint", "EndPaint"],
        "advapi32": ["RegOpenKey", "RegCreateKey", "RegQueryValue", "RegSetValue", "RegDeleteKey",
                     "RegCloseKey", "RegEnumKey", "RegEnumValue", "OpenProcessToken", "AdjustTokenPrivileges"],
        "ws2_32": ["socket", "bind", "listen", "accept", "connect", "send", "recv", "sendto", "recvfrom",
                   "closesocket", "select", "ioctlsocket", "WSAStartup", "WSACleanup"],
        "wininet": ["InternetOpen", "InternetConnect", "HttpOpenRequest", "HttpSendRequest",
                    "InternetReadFile", "InternetCloseHandle", "InternetSetOption"],
        "shell32": ["SHGetFolderPath", "SHBrowseForFolder", "ShellExecute", "ShellExecuteEx",
                    "SHGetFileInfo", "SHCreateDirectory", "SHFileOperation"],
        "ole32": ["CoInitialize", "CoUninitialize", "CoCreateInstance", "CoGetClassObject",
                  "CreateStreamOnHGlobal", "GetHGlobalFromStream"],
        "comctl32": ["InitCommonControls", "CreateToolbarEx", "CreateStatusWindow", "CreateWindowEx"],
        
        # C Runtime
        "crt": ["malloc", "free", "calloc", "realloc", "memcpy", "memmove", "memset", "memcmp",
                "strcpy", "strncpy", "strcat", "strncat", "strcmp", "strncmp", "strlen",
                "sprintf", "snprintf", "printf", "fprintf", "fopen", "fclose", "fread", "fwrite",
                "fseek", "ftell", "rewind", "feof", "ferror", "exit", "abort", "atexit",
                "system", "getenv", "putenv", "setenv", "unsetenv", "time", "localtime", "gmtime"],
    }
    
    # Control flow patterns
    CONTROL_FLOW_PATTERNS = {
        "loop_for": ["for", "while", "do_while", "loop"],
        "loop_infinite": ["while(true)", "for(;;)", "infinite"],
        "conditional_if": ["if", "switch", "case", "ternary"],
        "conditional_error": ["if(!", "if (error", "if (fail", "if (null", "if (0 == "],
        "try_catch": ["try", "catch", "__try", "__except", "__finally", "finally"],
        "return_early": ["return", "early_return", "guard"],
        "goto": ["goto", "jump", "label"],
    }
    
    UNKNOWN_PREFIXES = [
        "FUN_", "VAR_", "DAT_", "LAB_", "UNK_",
        "sub_", "loc_", "off_", "ptr_",
        "stru_", "enum_", "type_",
        "field_", "param_", "local_",
        "g_", "p_", "v_", "u_",
        "dword_", "qword_", "byte_", "word_",
        "unk_", "null_", "zero_",
    ]
    
    def __init__(self):
        self.compiled_patterns = {}
        self._compile_patterns()
    
    def _compile_patterns(self):
        """Pre-compile regex patterns for efficiency."""
        import re
        for category, keywords in self.FUNCTION_PATTERNS.items():
            pattern = r'\b(' + '|'.join(re.escape(kw) for kw in keywords) + r')\b'
            self.compiled_patterns[category] = re.compile(pattern, re.IGNORECASE)
    
    def is_unknown(self, name: str) -> bool:
        """Check if name matches unknown/generated patterns."""
        if not name:
            return True
        return any(name.startswith(p) for p in self.UNKNOWN_PREFIXES)
    
    def analyze_function_context(self, analysis: Dict[str, Any]) -> Dict[str, Any]:
        """
        Deep analyze function context to infer purpose.
        
        Returns context features for naming.
        """
        pseudocode = analysis.get("pseudocode", "").lower()
        assembly = " ".join(analysis.get("assembly", [])).lower()
        strings = [s.lower() for s in analysis.get("strings", [])]
        callees = [c.lower() for c in analysis.get("callees", [])]
        callers = [c.lower() for c in analysis.get("callers", [])]
        basic_blocks = analysis.get("basic_blocks", [])
        params = analysis.get("parameters", [])
        locals_vars = analysis.get("locals", [])
        
        context = {
            "pseudocode": pseudocode,
            "assembly": assembly,
            "strings": strings,
            "callees": callees,
            "callers": callers,
            "bb_count": len(basic_blocks),
            "param_count": len(params),
            "local_count": len(locals_vars),
            "param_types": [p.get("data_type", "").lower() for p in params],
            "return_type": analysis.get("return_type", "").lower(),
            "calling_convention": analysis.get("calling_convention", "").lower(),
        }
        
        # Detect patterns
        context["patterns"] = self._detect_patterns(context)
        context["api_calls"] = self._detect_api_calls(callees)
        context["control_flow"] = self._detect_control_flow(pseudocode)
        context["string_hints"] = self._analyze_strings(strings)
        context["param_hints"] = self._analyze_params(params)
        
        return context
    
    def _detect_patterns(self, context: Dict[str, Any]) -> Dict[str, float]:
        """Detect function patterns with confidence scores."""
        scores = {}
        text = context["pseudocode"] + " " + context["assembly"]
        
        for category, pattern in self.compiled_patterns.items():
            matches = len(pattern.findall(text))
            if matches > 0:
                # Normalize by text length
                scores[category] = min(1.0, matches * 0.15 + 0.2)
        
        return scores
    
    def _detect_api_calls(self, callees: List[str]) -> Dict[str, List[str]]:
        """Detect known API calls by category."""
        api_calls = {}
        for cat, apis in self.API_SIGNATURES.items():
            matches = [c for c in callees if any(api.lower() in c for api in apis)]
            if matches:
                api_calls[cat] = matches
        return api_calls
    
    def _detect_control_flow(self, pseudocode: str) -> Dict[str, bool]:
        """Detect control flow patterns."""
        flow = {}
        for cat, keywords in self.CONTROL_FLOW_PATTERNS.items():
            flow[cat] = any(kw.lower() in pseudocode for kw in keywords)
        return flow
    
    def _analyze_strings(self, strings: List[str]) -> Dict[str, List[str]]:
        """Analyze referenced strings for hints."""
        hints = {
            "error_msgs": [],
            "format_strs": [],
            "file_paths": [],
            "registry_keys": [],
            "urls": [],
            "guids": [],
            "config_keys": [],
        }
        
        for s in strings:
            s_lower = s.lower()
            if any(kw in s_lower for kw in ["error", "fail", "invalid", "failed", "exception"]):
                hints["error_msgs"].append(s)
            if "%" in s and any(c in s for c in "sduxfgec"):
                hints["format_strs"].append(s)
            if "\\" in s or "/" in s:
                hints["file_paths"].append(s)
            if "hkey_" in s_lower or "software\\" in s_lower or "system\\" in s_lower:
                hints["registry_keys"].append(s)
            if s_lower.startswith(("http://", "https://", "ftp://")):
                hints["urls"].append(s)
            if len(s) == 36 and s.count("-") == 4:
                hints["guids"].append(s)
            if any(kw in s_lower for kw in ["config", "setting", "option", "pref"]):
                hints["config_keys"].append(s)
        
        return hints
    
    def _analyze_params(self, params: List[Dict]) -> Dict[str, Any]:
        """Analyze function parameters for hints."""
        if not params:
            return {"count": 0, "has_ptr": False, "has_handle": False, "has_size": False}
        
        has_ptr = any("*" in p.get("data_type", "") or "ptr" in p.get("data_type", "").lower() 
                      or "LP" in p.get("data_type", "") for p in params)
        has_handle = any("handle" in p.get("data_type", "").lower() or "hkey" in p.get("data_type", "").lower()
                         or "hwnd" in p.get("data_type", "").lower() or "hfile" in p.get("data_type", "").lower()
                         for p in params)
        has_size = any("size" in p.get("name", "").lower() or "len" in p.get("name", "").lower()
                       or "count" in p.get("name", "").lower() or "cb" in p.get("name", "").lower()
                       for p in params)
        
        return {
            "count": len(params),
            "has_ptr": has_ptr,
            "has_handle": has_handle,
            "has_size": has_size,
            "types": [p.get("data_type", "") for p in params],
            "names": [p.get("name", "") for p in params],
        }
    
    def suggest_function_name(self, analysis: Dict[str, Any]) -> Optional[str]:
        """
        Suggest a meaningful function name based on deep analysis.
        
        Returns (suggested_name, confidence, reasoning)
        """
        context = self.analyze_function_context(analysis)
        
        # Score all pattern categories
        best_category = None
        best_score = 0.0
        reasoning = []
        
        for category, score in context["patterns"].items():
            if score > best_score:
                best_score = score
                best_category = category
        
        # Boost score based on API calls
        if context["api_calls"]:
            for cat, apis in context["api_calls"].items():
                # Map API categories to function categories
                api_to_cat = {
                    "kernel32": ["file_open", "file_read", "file_write", "file_close", "memory_copy", 
                                 "memory_set", "proc_create", "proc_wait", "thread_sync"],
                }
                if cat in api_to_cat:
                    for api_cat in api_to_cat[cat]:
                        if api_cat in self.compiled_patterns:
                            context["patterns"][api_cat] = context["patterns"].get(api_cat, 0) + 0.3
        
        # Re-evaluate best category after API boost
        for category, score in context["patterns"].items():
            if score > best_score:
                best_score = score
                best_category = category
                reasoning.append(f"Pattern match: {category} (score: {score:.2f})")
        
        # String-based hints
        if context["string_hints"]["error_msgs"]:
            if best_category not in ["error_handle", "error_handle"]:
                best_category = "error_handle"
                best_score = max(best_score, 0.7)
                reasoning.append("Error messages in strings")
        
        if context["string_hints"]["config_keys"]:
            if best_category not in ["parse_config", "initialize"]:
                best_category = "parse_config"
                best_score = max(best_score, 0.65)
                reasoning.append("Config keys in strings")
        
        if context["string_hints"]["urls"]:
            best_category = "net_http"
            best_score = max(best_score, 0.8)
            reasoning.append("URLs in strings")
        
        if context["string_hints"]["file_paths"]:
            if best_category not in ["file_open", "file_read", "file_write"]:
                best_category = "file_open"
                best_score = max(best_score, 0.6)
                reasoning.append("File paths in strings")
        
        if context["string_hints"]["registry_keys"]:
            best_category = "reg_open"
            best_score = max(best_score, 0.7)
            reasoning.append("Registry keys in strings")
        
        # Control flow hints
        if context["control_flow"]["conditional_error"]:
            if best_category not in ["error_handle", "validate"]:
                best_category = "validate"
                best_score = max(best_score, 0.55)
                reasoning.append("Error-checking control flow")
        
        if context["control_flow"]["try_catch"]:
            best_category = "error_handle"
            best_score = max(best_score, 0.6)
            reasoning.append("Exception handling")
        
        # Parameter hints
        param_hints = context["param_hints"]
        if param_hints["has_handle"]:
            if best_category not in ["file_open", "reg_open", "net_connect", "thread_sync"]:
                best_category = "handle_operation"
                best_score = max(best_score, 0.5)
                reasoning.append("Handle parameter")
        
        if param_hints["has_size"]:
            if best_category not in ["memory_copy", "file_read", "file_write", "net_send", "net_recv"]:
                best_category = "buffer_operation"
                best_score = max(best_score, 0.5)
                reasoning.append("Size parameter")
        
        # Caller context
        if context["callers"]:
            # If called from entry/main, likely initialization
            for caller in context["callers"]:
                if any(kw in caller.lower() for kw in ["entry", "main", "startup", "init", "dllmain", "winmain"]):
                    if best_category not in ["initialize", "entry"]:
                        best_category = "initialize"
                        best_score = max(best_score, 0.7)
                        reasoning.append("Called from entry point")
        
        # Generate name
        if best_category and best_score > 0.35:
            # Add context suffix for disambiguation
            name = best_category
            if context["bb_count"] > 100:
                name += "_complex"
            elif context["bb_count"] > 50:
                name += "_large"
            elif context["bb_count"] > 20:
                name += "_medium"
            elif context["bb_count"] < 5:
                name += "_simple"
            
            return name
        
        # Fallback: use first significant string or callee
        if analysis.get("strings"):
            for s in analysis["strings"]:
                if len(s) > 3 and not s.startswith(("FUN_", "VAR_", "DAT_", "unk", "null")):
                    return self._string_to_func_name(s)
        
        if analysis.get("callees"):
            for c in analysis["callees"]:
                if not c.startswith(("FUN_", "thunk_", "EXTERNAL:")):
                    return f"call_{c.lower().replace('::', '_').replace('.', '_')}"
        
        return None
    
    def _string_to_func_name(self, s: str) -> str:
        """Convert a string to a valid function name."""
        import re
        # Remove non-alphanumeric
        name = re.sub(r'[^a-zA-Z0-9_]', '_', s)
        # Remove leading digits
        name = re.sub(r'^\d+', '', name)
        # Limit length
        name = name[:50]
        # Ensure starts with letter
        if name and not name[0].isalpha():
            name = "func_" + name
        return name.lower() or "unknown_func"
    
    def suggest_variable_name(self, var_info: Dict[str, Any], func_context: Dict[str, Any] = None) -> Optional[str]:
        """Suggest variable name based on type, usage, and function context."""
        name = var_info.get("name", "").lower()
        var_type = var_info.get("type", "").lower()
        stack_offset = var_info.get("stack_offset", "")
        usage = var_info.get("usage", {})
        
        # Type-based hints
        if "ptr" in var_type or "*" in var_type or "LP" in var_type:
            if "char" in var_type or "wchar" in var_type or "string" in var_type:
                return "str_ptr"
            if "void" in var_type:
                return "void_ptr"
            return "ptr"
        
        if "handle" in var_type or "hkey" in var_type or "hwnd" in var_type or "hfile" in var_type:
            return "handle"
        
        if "size" in var_type or "len" in var_type or "count" in var_type:
            return "size"
        
        if "bool" in var_type or "flag" in var_type:
            return "flag"
        
        if "int" in var_type or "dword" in var_type or "long" in var_type:
            return "value"
        
        if "float" in var_type or "double" in var_type:
            return "fvalue"
        
        # Usage-based hints
        reads = usage.get("reads", 0)
        writes = usage.get("writes", 0)
        
        if writes > reads * 2:
            prefix = "out_"
        elif reads > writes * 2:
            prefix = "in_"
        else:
            prefix = "var_"
        
        # Name-based hints
        for category, keywords in self.VARIABLE_PATTERNS.items():
            if any(kw in name for kw in keywords):
                return prefix + category
        
        # Stack offset hint (for stack variables)
        if stack_offset:
            try:
                offset = int(stack_offset.replace("0x", ""), 16) if "0x" in stack_offset else int(stack_offset)
                if offset < 32:
                    return prefix + "arg"
                elif offset < 128:
                    return prefix + "local"
                else:
                    return prefix + "large_local"
            except:
                pass
        
        # Function context hints
        if func_context:
            func_name = func_context.get("name", "").lower()
            if "alloc" in func_name or "create" in func_name:
                return prefix + "allocated"
            if "free" in func_name or "destroy" in func_name:
                return prefix + "to_free"
            if "init" in func_name:
                return prefix + "init"
            if "parse" in func_name:
                return prefix + "parsed"
        
        # Default based on type
        if "ptr" in var_type:
            return prefix + "ptr"
        if "handle" in var_type:
            return prefix + "handle"
        
        return prefix + "data"
    
    def suggest_type_name(self, type_info: Dict[str, Any]) -> Optional[str]:
        """Suggest type name for structs, enums, typedefs."""
        name = type_info.get("name", "")
        kind = type_info.get("kind", "")
        fields = type_info.get("fields", [])
        
        if self.is_unknown(name):
            # Infer from fields
            field_names = [f.get("name", "").lower() for f in fields]
            
            # Common patterns
            if any("next" in f and "prev" in f for f in field_names):
                return "linked_list_node"
            if any("left" in f and "right" in f for f in field_names):
                return "tree_node"
            if any("head" in f and "tail" in f for f in field_names):
                return "queue"
            if any("count" in f or "size" in f for f in field_names) and any("capacity" in f for f in field_names):
                return "dynamic_array"
            if any("key" in f and "value" in f for f in field_names):
                return "key_value_pair"
            if any("x" in f and "y" in f and "z" in f for f in field_names):
                return "vector3d"
            if any("r" in f and "g" in f and "b" in f for f in field_names):
                return "color_rgb"
            if any("width" in f and "height" in f for f in field_names):
                return "rect" if "left" in str(field_names) else "size"
            if any("guid" in f or "uuid" in f for f in field_names):
                return "guid"
            
            return "struct_" + str(len(fields)) + "fields"
        
        return None


async def run_pipeline_with_tools(
    tools,
    binary_path: str,
    output_dir: str,
    formats: List[str] = None,
    rename_mode: str = "pattern",
    max_parallel: int = 8,
    track_progress: bool = True
) -> Dict[str, Any]:
    """
    Main code-mode entry point. Uses actual REA tools via the tools namespace.
    
    Args:
        tools: OpenCode's tools namespace (provides tools.rea.*)
        binary_path: Path to binary file
        output_dir: Output directory for results
        formats: Export formats (c, rust, go, python, json, markdown, assembly)
        rename_mode: ai, pattern, or manual
        max_parallel: Max parallel functions
        track_progress: Whether to track and report progress
    
    Returns:
        Dictionary with pipeline results and paths
    """
    
    if formats is None:
        formats = ["c", "json", "markdown"]
    
    binary_path = Path(binary_path).resolve()
    output_dir = Path(output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Initialize
    rea = REAToolsClient(tools)
    intelligent_renamer = IntelligentRenamer()
    
    # Progress tracking
    progress = ProgressTracker()
    project = ProjectState(
        binary=BinaryInfo(
            path=str(binary_path),
            sha256=hashlib.sha256(binary_path.read_bytes()).hexdigest(),
            format="unknown",
            architecture="unknown",
        )
    )
    
    results = {
        "binary_path": str(binary_path),
        "output_dir": str(output_dir),
        "stages": {},
        "progress": {},
        "files_created": []
    }
    
    try:
        # ============================================================
        # STAGE 1: Open binary
        # ============================================================
        logger.info("[1/9] Opening binary with Ghidra...")
        binary_info = await rea.open_binary(str(binary_path))
        project.binary.format = binary_info.get("format", "pe")
        project.binary.architecture = binary_info.get("architecture", "x86_64")
        project.binary.entry_point = binary_info.get("entry_point")
        results["stages"]["open_binary"] = binary_info
        
        # ============================================================
        # STAGE 2: Binary overview
        # ============================================================
        logger.info("[2/9] Getting binary overview...")
        overview = await rea.binary_overview()
        results["stages"]["overview"] = overview
        
        # ============================================================
        # STAGE 3: List procedures
        # ============================================================
        logger.info("[3/9] Listing all procedures...")
        procedures = await rea.list_procedures(detail="complete")
        progress.total_functions = len(procedures)
        
        for proc in procedures:
            addr = proc.get("address", "")
            name = proc.get("value", "")
            is_unknown = intelligent_renamer.is_unknown(name)
            
            project.functions[addr] = FunctionInfo(address=addr, name=name, size=0)
            project.symbols[addr] = SymbolInfo(
                address=addr, name=name, symbol_type="function",
                is_unknown=is_unknown, original_name=name
            )
            progress.total_symbols += 1
            if is_unknown:
                progress.unknown_symbols += 1
        
        results["stages"]["procedures_count"] = len(procedures)
        logger.info(f"  Found {len(procedures)} procedures ({progress.unknown_symbols} unknown)")
        
        # ============================================================
        # STAGE 4: List symbols (summary mode to avoid size limits)
        # ============================================================
        logger.info("[4/9] Listing symbols...")
        symbols_summary = await rea.list_names(detail="summary")
        results["stages"]["symbols_summary"] = symbols_summary
        
        # ============================================================
        # STAGE 5: List strings
        # ============================================================
        logger.info("[5/9] Extracting strings...")
        strings_summary = await rea.list_strings(detail="summary")
        results["stages"]["strings_summary"] = strings_summary
        
        # ============================================================
        # STAGE 6: List segments
        # ============================================================
        logger.info("[6/9] Analyzing memory segments...")
        segments = await rea.list_segments(detail="summary")
        project.segments = segments
        results["stages"]["segments"] = segments
        
        # ============================================================
        # STAGE 7: Inspect exports
        # ============================================================
        logger.info("[7/9] Inspecting native exports...")
        exports = await rea.inspect_native_exports(str(binary_path), detail="summary")
        project.exports = exports.get("exports", [])
        results["stages"]["exports"] = exports
        
        # Mark exported functions as known
        for exp in project.exports:
            addr = exp.get("address", "")
            if addr in project.symbols and project.symbols[addr].is_unknown:
                project.symbols[addr].is_unknown = False
                progress.resolved_unknowns += 1
        
        # ============================================================
        # STAGE 8: Decompile ALL functions
        # ============================================================
        logger.info("[8/9] Decompiling ALL functions...")
        decompile_dir = output_dir / "decompilation"
        decompile_dir.mkdir(parents=True, exist_ok=True)
        
        decompile_result = await rea.decompile_all(str(decompile_dir), layout="address")
        counts = decompile_result.get("counts", {})
        progress.decompiled_functions = counts.get("written", 0)
        project.decompilation_complete = True
        results["stages"]["decompilation"] = decompile_result
        results["files_created"].append(str(decompile_dir))
        logger.info(f"  Decompiled {progress.decompiled_functions} functions")
        
        # ============================================================
        # STAGE 9: Deep analyze functions
        # ============================================================
        logger.info("[9/9] Deep analyzing functions...")
        analyzed = 0
        batch_size = max_parallel
        
        functions_to_analyze = [
            (addr, func) for addr, func in project.functions.items()
            if not func.analyzed
        ]
        
        for i in range(0, len(functions_to_analyze), batch_size):
            batch = functions_to_analyze[i:i+batch_size]
            tasks = [rea.analyze_function(addr) for addr, _ in batch]
            batch_results = await asyncio.gather(*tasks, return_exceptions=True)
            
            for j, (addr, func) in enumerate(batch):
                if isinstance(batch_results[j], Exception):
                    logger.warning(f"Failed to analyze {addr}: {batch_results[j]}")
                else:
                    data = batch_results[j]
                    func.analyzed = True
                    proc = data.get("procedure", {})
                    func.signature = proc.get("signature", "")
                    func.calling_convention = proc.get("calling_convention", "")
                    func.return_type = proc.get("return_type", "")
                    func.parameters = proc.get("parameters", [])
                    func.pseudocode = data.get("pseudocode", "")
                    func.assembly = data.get("assembly", [])
                    func.basic_blocks = data.get("basic_blocks", [])
                    func.callees = [c.get("address", "") for c in data.get("callees", [])]
                    func.callers = [c.get("address", "") for c in data.get("callers", [])]
                    func.strings = [s.get("value", "") for s in data.get("referenced_strings", [])]
                    func.variables = [
                        SymbolInfo(
                            address=loc.get("stack_displacement", ""),
                            name=loc.get("name", f"var_{loc.get('stack_displacement', 'unknown')}"),
                            symbol_type="variable",
                            is_unknown=True,
                        )
                        for loc in proc.get("locals", [])
                    ]
                    analyzed += 1
                    progress.analyzed_functions = analyzed
            
            logger.info(f"  Analyzed {analyzed}/{len(functions_to_analyze)} functions")
        
        # ============================================================
        # STAGE 10: Rename symbols (pattern-based)
        # ============================================================
        logger.info("Renaming symbols...")
        renamed = 0
        
        for addr, symbol in project.symbols.items():
            if symbol.is_unknown and not symbol.renamed:
                func = project.functions.get(addr)
                
                if func:
                    suggested = intelligent_renamer.suggest_function_name({
                        "strings": func.strings,
                        "callees": func.callees,
                        "callers": func.callers,
                        "basic_blocks": func.basic_blocks,
                        "parameters": func.parameters,
                        "return_type": func.return_type,
                        "pseudocode": func.pseudocode,
                        "assembly": func.assembly,
                        "locals": [{"name": v.name, "type": "unknown", "stack_offset": v.address} for v in func.variables],
                    })
                else:
                    suggested = intelligent_renamer.suggest_variable_name({
                        "name": symbol.name,
                        "type": "unknown",
                    }, func_context={"name": func.name} if func else None)
                
                if suggested and suggested != symbol.name:
                    symbol.suggested_name = suggested
                    symbol.confidence = 0.75
                    
                    # Apply rename via REA
                    if addr in project.functions:
                        await rea.annotate_native_function(addr, name=suggested)
                        func.name = suggested
                        func.renamed = True
                    else:
                        await rea.set_address_name(addr, suggested)
                    
                    symbol.name = suggested
                    symbol.renamed = True
                    renamed += 1
                    progress.renamed_symbols += 1
                    progress.resolved_unknowns += 1
                    logger.info(f"  Renamed {addr}: {symbol.original_name} -> {suggested}")
        
        logger.info(f"Renamed {renamed} symbols")
        
        # ============================================================
        # STAGE 11: Export to target formats
        # ============================================================
        for fmt in formats:
            logger.info(f"Exporting to {fmt}...")
            export_dir = output_dir / f"export_{fmt}"
            export_dir.mkdir(parents=True, exist_ok=True)
            
            if fmt == "c":
                await export_c(project, export_dir)
            elif fmt == "json":
                await export_json(project, export_dir)
            elif fmt == "markdown":
                await export_markdown(project, export_dir)
            elif fmt == "rust":
                await export_rust(project, export_dir)
            elif fmt == "go":
                await export_go(project, export_dir)
            elif fmt == "python":
                await export_python(project, export_dir)
            
            results["files_created"].append(str(export_dir))
        
        project.export_complete = True
        
        # ============================================================
        # STAGE 12: Generate parity report
        # ============================================================
        if track_progress:
            report = {
                "timestamp": datetime.now().isoformat(),
                "binary": asdict(project.binary),
                "progress": progress.to_dict(),
                "parity_check": {
                    "total_functions": progress.total_functions,
                    "functions_with_meaningful_names": progress.total_functions - progress.unknown_symbols,
                    "functions_with_unknown_names": progress.unknown_symbols,
                    "parity_achieved": progress.unknown_symbols == 0,
                },
                "unknown_symbols": [
                    {"address": addr, "original_name": sym.original_name, "current_name": sym.name}
                    for addr, sym in project.symbols.items() if sym.is_unknown
                ],
                "renamed_symbols": [
                    {"address": addr, "original_name": sym.original_name, "new_name": sym.name}
                    for addr, sym in project.symbols.items() if sym.renamed
                ],
            }
            
            report_path = output_dir / "parity_report.json"
            report_path.write_text(json.dumps(report, indent=2))
            results["files_created"].append(str(report_path))
            results["parity_report"] = report
        
        # ============================================================
        # STAGE 13: Save project state
        # ============================================================
        project_path = output_dir / "project_state.json"
        project_path.write_text(json.dumps(project.to_dict(), indent=2, default=str))
        results["files_created"].append(str(project_path))
        
        # ============================================================
        # STAGE 14: Close session with snapshot
        # ============================================================
        snapshot_path = output_dir / "snapshot.ghidra"
        await rea.close_binary(snapshot_path=str(snapshot_path), overwrite=True)
        results["files_created"].append(str(snapshot_path))
        
        results["progress"] = progress.to_dict()
        results["success"] = True
        
        logger.info(f"✅ Pipeline complete! Output in: {output_dir}")
        
    except Exception as e:
        logger.exception(f"Pipeline failed: {e}")
        results["success"] = False
        results["error"] = str(e)
        # Try to close session
        if rea.session_active:
            try:
                await rea.close_binary()
            except:
                pass
    
    return results


# ============================================================
# Export functions
# ============================================================

async def export_c(project: ProjectState, output_dir: Path):
    header = output_dir / "decompiled.h"
    content = "// Decompiled header\n// Generated by decompile-everything\n\n"
    content += "#ifndef DECOMPILED_H\n#define DECOMPILED_H\n\n"
    content += "#include <stdint.h>\n#include <stddef.h>\n\n"
    content += "// Function declarations\n"
    
    for addr, func in sorted(project.functions.items()):
        sig = func.signature or f"void {func.name}();"
        content += f"{sig}\n"
    
    content += "\n#endif\n"
    header.write_text(content)
    
    for addr, func in sorted(project.functions.items()):
        if func.pseudocode:
            src = output_dir / f"{func.name}.c"
            content = f"/* {func.name} at {addr} */\n#include \"decompiled.h\"\n\n{func.pseudocode}\n"
            src.write_text(content)


async def export_rust(project: ProjectState, output_dir: Path):
    lib = output_dir / "lib.rs"
    content = "//! Decompiled Rust module\n//! Generated by decompile-everything\n\n"
    content += "#![allow(non_camel_case_types)]\n"
    content += "#![allow(non_snake_case)]\n"
    content += "#![allow(dead_code)]\n\n"
    content += "use std::os::raw::{c_int, c_char, c_void};\n\n"
    
    for addr, func in sorted(project.functions.items()):
        rust_sig = func.signature.replace("void ", "").replace("int ", "c_int ").replace("char* ", "*mut c_char ")
        content += f'\nextern "C" {{\n    fn {func.name}({rust_sig});\n}}\n'
    
    lib.write_text(content)


async def export_go(project: ProjectState, output_dir: Path):
    main = output_dir / "decompiled.go"
    content = "// Decompiled Go package\n// Generated by decompile-everything\n\npackage decompiled\n\nimport \"unsafe\"\n\n"
    
    for addr, func in sorted(project.functions.items()):
        content += f"\n// {func.name} at {addr}\nfunc {func.name}() {{}}\n"
    
    main.write_text(content)


async def export_python(project: ProjectState, output_dir: Path):
    bindings = output_dir / "bindings.py"
    content = "# Decompiled Python bindings\n# Generated by decompile-everything\n\nfrom ctypes import *\n\n"
    
    for addr, func in sorted(project.functions.items()):
        content += f"\n# {func.name} at {addr}\n{func.name} = CFUNCTYPE(None)\n"
    
    bindings.write_text(content)


async def export_json(project: ProjectState, output_dir: Path):
    json_file = output_dir / "decompilation.json"
    data = {
        "binary": asdict(project.binary),
        "functions": {addr: asdict(func) for addr, func in project.functions.items()},
        "symbols": {addr: asdict(sym) for addr, sym in project.symbols.items()},
        "strings": project.strings,
        "segments": project.segments,
        "exports": project.exports,
    }
    json_file.write_text(json.dumps(data, indent=2, default=str))


async def export_markdown(project: ProjectState, output_dir: Path):
    md = output_dir / "DECOMPILATION.md"
    content = f"# Decompilation Report\n\n"
    content += f"**Binary:** {project.binary.path}\n"
    content += f"**SHA256:** {project.binary.sha256}\n"
    content += f"**Format:** {project.binary.format}\n"
    content += f"**Architecture:** {project.binary.architecture}\n\n"
    
    p = project  # shortcut
    content += f"## Summary\n\n"
    content += f"- **Total Functions:** {len(p.functions)}\n"
    content += f"- **Analyzed:** {sum(1 for f in p.functions.values() if f.analyzed)}\n"
    content += f"- **Decompiled:** {sum(1 for f in p.functions.values() if f.pseudocode)}\n"
    content += f"- **Renamed:** {sum(1 for f in p.functions.values() if f.renamed)}\n"
    content += f"- **Unknown Symbols:** {sum(1 for s in p.symbols.values() if s.is_unknown)}\n\n"
    
    content += "## Functions\n\n| Address | Name | Status |\n|---------|------|--------|\n"
    for addr, func in sorted(p.functions.items()):
        status = "✅" if func.analyzed else "⏳"
        if func.renamed:
            status += " 🏷️"
        content += f"| {addr} | {func.name} | {status} |\n"
    
    md.write_text(content)