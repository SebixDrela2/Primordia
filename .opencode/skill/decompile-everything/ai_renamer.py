#!/usr/bin/env python3
"""
AI-assisted symbol renaming module for decompile-everything.

Integrates with LLM providers (OpenAI, Anthropic) to analyze decompiled code
and suggest meaningful symbol names.
"""

import asyncio
import json
import os
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Dict, List, Optional, Any
from pathlib import Path


@dataclass
class RenameSuggestion:
    address: str
    original_name: str
    suggested_name: str
    confidence: float
    reasoning: str
    symbol_type: str  # function, variable, type, etc.


class LLMRenamer(ABC):
    """Abstract base class for LLM-based renamers."""
    
    @abstractmethod
    async def analyze_function(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        """Analyze function context and return rename suggestions."""
        pass
    
    @abstractmethod
    async def analyze_variable(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        """Analyze variable context and return rename suggestions."""
        pass
    
    @abstractmethod
    async def batch_analyze(self, contexts: List[Dict[str, Any]]) -> List[RenameSuggestion]:
        """Batch analyze multiple contexts."""
        pass


class PatternRenamer:
    """Heuristic pattern-based renamer (no LLM required)."""
    
    # Common patterns for function naming
    FUNCTION_PATTERNS = {
        # Error handling
        "error": ["error", "fail", "err", "exception", "abort", "panic"],
        "init": ["init", "initialize", "setup", "configure", "bootstrap"],
        "cleanup": ["cleanup", "destroy", "free", "release", "close", "shutdown"],
        # Memory
        "alloc": ["malloc", "alloc", "calloc", "realloc", "new", "create"],
        "free": ["free", "delete", "destroy", "dealloc", "release"],
        # String
        "strcpy": ["strcpy", "strncpy", "strcat", "strncat", "sprintf", "snprintf"],
        "strcmp": ["strcmp", "strncmp", "stricmp", "strcasecmp"],
        "strlen": ["strlen", "strnlen"],
        # Network
        "send": ["send", "write", "transmit", "push", "emit"],
        "recv": ["recv", "read", "receive", "pull", "fetch"],
        "connect": ["connect", "dial", "link", "attach"],
        # Crypto
        "encrypt": ["encrypt", "cipher", "encode", "hash", "sign"],
        "decrypt": ["decrypt", "decipher", "decode", "verify"],
        # File I/O
        "open": ["open", "create", "fopen", "openat"],
        "close": ["close", "fclose", "closehandle"],
        "read": ["read", "fread", "pread", "readfile"],
        "write": ["write", "fwrite", "pwrite", "writefile"],
        # Threading
        "thread_create": ["thread", "pthread_create", "_beginthread", "CreateThread"],
        "thread_join": ["join", "pthread_join", "WaitForSingleObject"],
        "lock": ["lock", "mutex", "critical_section", "spinlock", "acquire"],
        "unlock": ["unlock", "release", "leave_critical_section"],
    }
    
    # Variable naming patterns
    VARIABLE_PATTERNS = {
        "counter": ["count", "cnt", "num", "index", "idx", "i", "j", "k"],
        "size": ["size", "len", "length", "sz", "cb", "dwSize"],
        "pointer": ["ptr", "p", "pp", "lp", "address", "addr"],
        "handle": ["handle", "h", "hndl", "fd", "socket", "sock"],
        "buffer": ["buf", "buffer", "data", "payload", "msg", "message"],
        "flag": ["flag", "flg", "enabled", "active", "valid", "ready", "done"],
        "config": ["config", "cfg", "settings", "options", "params", "args"],
        "result": ["result", "res", "ret", "retval", "return", "status", "rc"],
        "error": ["error", "err", "errno", "last_error", "hr", "hresult"],
        "timestamp": ["time", "timestamp", "ts", "tick", "clock", "date"],
        "id": ["id", "uid", "gid", "pid", "tid", "handle"],
        "version": ["version", "ver", "major", "minor", "build", "revision"],
    }
    
    def __init__(self):
        self.unknown_prefixes = [
            "FUN_", "VAR_", "DAT_", "LAB_", "UNK_",
            "sub_", "loc_", "off_", "ptr_",
            "stru_", "enum_", "type_",
            "field_", "param_", "local_",
        ]
    
    def is_unknown(self, name: str) -> bool:
        return any(name.startswith(p) for p in self.unknown_prefixes)
    
    def suggest_function_name(self, func_info: Dict[str, Any]) -> Optional[str]:
        """Suggest function name based on patterns."""
        # Check strings referenced
        strings = func_info.get("strings", [])
        for s in strings:
            s_lower = s.lower()
            for category, keywords in self.FUNCTION_PATTERNS.items():
                if any(kw in s_lower for kw in keywords):
                    return category
        
        # Check callee names
        callees = func_info.get("callees", [])
        for callee in callees:
            callee_lower = callee.lower()
            for category, keywords in self.FUNCTION_PATTERNS.items():
                if any(kw in callee_lower for kw in keywords):
                    return category
        
        # Check caller names
        callers = func_info.get("callers", [])
        for caller in callers:
            caller_lower = caller.lower()
            for category, keywords in self.FUNCTION_PATTERNS.items():
                if any(kw in caller_lower for kw in keywords):
                    return f"callback_{category}"
        
        # Check basic block count (complexity)
        bb_count = len(func_info.get("basic_blocks", []))
        if bb_count > 50:
            return "complex_routine"
        elif bb_count > 20:
            return "medium_routine"
        elif bb_count > 5:
            return "simple_routine"
        else:
            return "trivial_routine"
    
    def suggest_variable_name(self, var_info: Dict[str, Any], func_context: Dict[str, Any]) -> Optional[str]:
        """Suggest variable name based on usage patterns."""
        name = var_info.get("name", "").lower()
        var_type = var_info.get("type", "").lower()
        usage = var_info.get("usage", {})
        
        # Check type-based patterns
        if "ptr" in var_type or "*" in var_type:
            return "ptr_" + self._guess_pointer_purpose(var_info, func_context)
        if "size" in var_type or "len" in var_type:
            return "size"
        if "count" in var_type or "num" in var_type:
            return "count"
        if "flag" in var_type or "bool" in var_type:
            return "flag"
        if "handle" in var_type or "hndl" in var_type:
            return "handle"
        
        # Check usage patterns
        reads = usage.get("reads", 0)
        writes = usage.get("writes", 0)
        
        if writes > reads * 2:
            return "output_" + self._guess_purpose(var_info)
        elif reads > writes * 2:
            return "input_" + self._guess_purpose(var_info)
        else:
            return "var_" + self._guess_purpose(var_info)
    
    def _guess_pointer_purpose(self, var_info: Dict[str, Any], func_context: Dict[str, Any]) -> str:
        """Guess what a pointer points to."""
        # Check if it's used as a buffer
        if var_info.get("used_as_buffer", False):
            return "buffer"
        # Check if it's a string
        if var_info.get("used_as_string", False):
            return "str"
        # Check if it's a struct
        if var_info.get("struct_access", False):
            return "struct"
        return "data"
    
    def _guess_purpose(self, var_info: Dict[str, Any]) -> str:
        """Guess variable purpose from name/usage."""
        name = var_info.get("name", "").lower()
        
        for category, keywords in self.VARIABLE_PATTERNS.items():
            if any(kw in name for kw in keywords):
                return category
        
        return "unknown"


class MockLLMRenamer(LLMRenamer):
    """Mock LLM renamer for testing without API keys."""
    
    def __init__(self):
        self.pattern_renamer = PatternRenamer()
    
    async def analyze_function(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        func_info = context.get("function", {})
        suggested = self.pattern_renamer.suggest_function_name(func_info)
        
        if suggested:
            addr = func_info.get("address", "unknown")
            original = func_info.get("name", "unknown")
            
            return [RenameSuggestion(
                address=addr,
                original_name=original,
                suggested_name=suggested,
                confidence=0.75,
                reasoning=f"Pattern match: {suggested}",
                symbol_type="function"
            )]
        return []
    
    async def analyze_variable(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        var_info = context.get("variable", {})
        func_context = context.get("function_context", {})
        suggested = self.pattern_renamer.suggest_variable_name(var_info, func_context)
        
        if suggested:
            return [RenameSuggestion(
                address=var_info.get("address", "unknown"),
                original_name=var_info.get("name", "unknown"),
                suggested_name=suggested,
                confidence=0.65,
                reasoning=f"Usage pattern: {suggested}",
                symbol_type="variable"
            )]
        return []
    
    async def batch_analyze(self, contexts: List[Dict[str, Any]]) -> List[RenameSuggestion]:
        results = []
        for ctx in contexts:
            if ctx.get("type") == "function":
                results.extend(await self.analyze_function(ctx))
            elif ctx.get("type") == "variable":
                results.extend(await self.analyze_variable(ctx))
        return results


class OpenAIRenamer(LLMRenamer):
    """OpenAI GPT-based renamer."""
    
    def __init__(self, api_key: Optional[str] = None, model: str = "gpt-4"):
        self.api_key = api_key or os.getenv("OPENAI_API_KEY")
        self.model = model
        self.client = None
        if self.api_key:
            try:
                from openai import AsyncOpenAI
                self.client = AsyncOpenAI(api_key=self.api_key)
            except ImportError:
                print("Warning: openai package not installed. Install with: pip install openai")
    
    async def analyze_function(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        if not self.client:
            return []
        
        prompt = self._build_function_prompt(context)
        
        try:
            response = await self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": self._get_system_prompt()},
                    {"role": "user", "content": prompt}
                ],
                temperature=0.3,
                max_tokens=500,
                response_format={"type": "json_object"}
            )
            
            return self._parse_response(response.choices[0].message.content, context, "function")
        except Exception as e:
            print(f"OpenAI API error: {e}")
            return []
    
    async def analyze_variable(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        if not self.client:
            return []
        
        prompt = self._build_variable_prompt(context)
        
        try:
            response = await self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": self._get_system_prompt()},
                    {"role": "user", "content": prompt}
                ],
                temperature=0.3,
                max_tokens=300,
                response_format={"type": "json_object"}
            )
            
            return self._parse_response(response.choices[0].message.content, context, "variable")
        except Exception as e:
            print(f"OpenAI API error: {e}")
            return []
    
    async def batch_analyze(self, contexts: List[Dict[str, Any]]) -> List[RenameSuggestion]:
        # Batch in groups of 10
        results = []
        for i in range(0, len(contexts), 10):
            batch = contexts[i:i+10]
            # For batch, we'd need a different prompt structure
            for ctx in batch:
                if ctx.get("type") == "function":
                    results.extend(await self.analyze_function(ctx))
                elif ctx.get("type") == "variable":
                    results.extend(await self.analyze_variable(ctx))
        return results
    
    def _get_system_prompt(self) -> str:
        return """You are an expert reverse engineer analyzing decompiled code to suggest meaningful symbol names.

Your task: Given decompiled function/variable context, suggest a clear, descriptive name.

Guidelines:
- Use snake_case for functions and variables
- Be specific: "parse_json_config" not "parse_data"
- Indicate purpose: "validate_input", "calculate_checksum", "send_http_request"
- For variables: indicate role: "input_buffer", "output_size", "error_code", "config_ptr"
- Return JSON only with: suggested_name, confidence (0-1), reasoning

Common patterns:
- FUN_XXXX -> descriptive verb phrase
- VAR_XXXX -> noun phrase indicating role
- DAT_XXXX -> data_<purpose>
- sub_XXXX -> sub_<purpose>"""
    
    def _build_function_prompt(self, context: Dict[str, Any]) -> str:
        func = context.get("function", {})
        
        prompt = f"""Analyze this decompiled function and suggest a meaningful name.

Function: {func.get('name', 'unknown')} at {func.get('address', 'unknown')}
Signature: {func.get('signature', 'unknown')}
Pseudocode:
```
{func.get('pseudocode', '// No pseudocode available')[:2000]}
```

Strings referenced: {func.get('strings', [])[:10]}
Called functions: {func.get('callees', [])[:10]}
Calling functions: {func.get('callers', [])[:10]}
Basic blocks: {len(func.get('basic_blocks', []))}
Variables: {len(func.get('variables', []))}

Return JSON: {{"suggested_name": "...", "confidence": 0.0-1.0, "reasoning": "..."}}"""
        return prompt
    
    def _build_variable_prompt(self, context: Dict[str, Any]) -> str:
        var = context.get("variable", {})
        func = context.get("function_context", {})
        
        prompt = f"""Analyze this variable and suggest a meaningful name.

Variable: {var.get('name', 'unknown')} at {var.get('address', 'unknown')}
Type: {var.get('type', 'unknown')}
Function context: {func.get('name', 'unknown')}
Usage: reads={var.get('usage', {}).get('reads', 0)}, writes={var.get('usage', {}).get('writes', 0)}
Stack offset: {var.get('stack_offset', 'unknown')}
Access patterns: {var.get('access_patterns', [])}

Return JSON: {{"suggested_name": "...", "confidence": 0.0-1.0, "reasoning": "..."}}"""
        return prompt
    
    def _parse_response(self, content: str, context: Dict[str, Any], symbol_type: str) -> List[RenameSuggestion]:
        try:
            data = json.loads(content)
            addr = context.get("function", {}).get("address") or context.get("variable", {}).get("address", "unknown")
            original = context.get("function", {}).get("name") or context.get("variable", {}).get("name", "unknown")
            
            return [RenameSuggestion(
                address=addr,
                original_name=original,
                suggested_name=data.get("suggested_name", ""),
                confidence=data.get("confidence", 0.5),
                reasoning=data.get("reasoning", ""),
                symbol_type=symbol_type
            )]
        except json.JSONDecodeError:
            return []


class AnthropicRenamer(LLMRenamer):
    """Anthropic Claude-based renamer."""
    
    def __init__(self, api_key: Optional[str] = None, model: str = "claude-3-opus-20240229"):
        self.api_key = api_key or os.getenv("ANTHROPIC_API_KEY")
        self.model = model
        self.client = None
        if self.api_key:
            try:
                from anthropic import AsyncAnthropic
                self.client = AsyncAnthropic(api_key=self.api_key)
            except ImportError:
                print("Warning: anthropic package not installed. Install with: pip install anthropic")
    
    async def analyze_function(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        if not self.client:
            return []
        
        prompt = self._build_function_prompt(context)
        
        try:
            response = await self.client.messages.create(
                model=self.model,
                max_tokens=500,
                temperature=0.3,
                system=self._get_system_prompt(),
                messages=[{"role": "user", "content": prompt}]
            )
            
            return self._parse_response(response.content[0].text, context, "function")
        except Exception as e:
            print(f"Anthropic API error: {e}")
            return []
    
    async def analyze_variable(self, context: Dict[str, Any]) -> List[RenameSuggestion]:
        if not self.client:
            return []
        
        prompt = self._build_variable_prompt(context)
        
        try:
            response = await self.client.messages.create(
                model=self.model,
                max_tokens=300,
                temperature=0.3,
                system=self._get_system_prompt(),
                messages=[{"role": "user", "content": prompt}]
            )
            
            return self._parse_response(response.content[0].text, context, "variable")
        except Exception as e:
            print(f"Anthropic API error: {e}")
            return []
    
    async def batch_analyze(self, contexts: List[Dict[str, Any]]) -> List[RenameSuggestion]:
        results = []
        for ctx in contexts:
            if ctx.get("type") == "function":
                results.extend(await self.analyze_function(ctx))
            elif ctx.get("type") == "variable":
                results.extend(await self.analyze_variable(ctx))
        return results
    
    def _get_system_prompt(self) -> str:
        return """You are an expert reverse engineer. Analyze decompiled code and suggest meaningful symbol names.

Return ONLY valid JSON with: suggested_name, confidence (0-1), reasoning.

Naming conventions:
- Functions: verb_phrase (parse_config, validate_input, send_request)
- Variables: noun_phrase indicating role (input_buffer, error_code, config_ptr)
- Use snake_case
- Be specific and descriptive"""
    
    def _build_function_prompt(self, context: Dict[str, Any]) -> str:
        func = context.get("function", {})
        return f"""Function: {func.get('name')} at {func.get('address')}
Signature: {func.get('signature')}
Pseudocode: {func.get('pseudocode', '')[:2000]}
Strings: {func.get('strings', [])[:10]}
Callees: {func.get('callees', [])[:10]}"""
    
    def _build_variable_prompt(self, context: Dict[str, Any]) -> str:
        var = context.get("variable", {})
        func = context.get("function_context", {})
        return f"""Variable: {var.get('name')} at {var.get('address')}
Type: {var.get('type')}
Function: {func.get('name')}
Usage: reads={var.get('usage', {}).get('reads', 0)}, writes={var.get('usage', {}).get('writes', 0)}"""
    
    def _parse_response(self, content: str, context: Dict[str, Any], symbol_type: str) -> List[RenameSuggestion]:
        try:
            # Extract JSON from response
            import re
            json_match = re.search(r'\{.*\}', content, re.DOTALL)
            if json_match:
                data = json.loads(json_match.group())
            else:
                data = json.loads(content)
            
            addr = context.get("function", {}).get("address") or context.get("variable", {}).get("address", "unknown")
            original = context.get("function", {}).get("name") or context.get("variable", {}).get("name", "unknown")
            
            return [RenameSuggestion(
                address=addr,
                original_name=original,
                suggested_name=data.get("suggested_name", ""),
                confidence=data.get("confidence", 0.5),
                reasoning=data.get("reasoning", ""),
                symbol_type=symbol_type
            )]
        except json.JSONDecodeError:
            return []


def create_renamer(provider: str = "auto", **kwargs) -> LLMRenamer:
    """Factory function to create appropriate renamer."""
    if provider == "openai":
        return OpenAIRenamer(**kwargs)
    elif provider == "anthropic":
        return AnthropicRenamer(**kwargs)
    elif provider == "pattern":
        return MockLLMRenamer()  # Uses pattern matching
    else:
        # Auto-detect
        if os.getenv("OPENAI_API_KEY"):
            return OpenAIRenamer()
        elif os.getenv("ANTHROPIC_API_KEY"):
            return AnthropicRenamer()
        else:
            return MockLLMRenamer()


# Example usage
async def main():
    """Demo the renamer."""
    renamer = create_renamer("pattern")
    
    # Test function context
    func_context = {
        "type": "function",
        "function": {
            "address": "0x140001234",
            "name": "FUN_140001234",
            "signature": "int FUN_140001234(char* config_path)",
            "pseudocode": """
int FUN_140001234(char* config_path) {
    FILE* fp = fopen(config_path, "r");
    if (!fp) return -1;
    char buffer[1024];
    fread(buffer, 1, 1024, fp);
    fclose(fp);
    return parse_json(buffer);
}
""",
            "strings": ["config.json", "Failed to open config", "parse_json"],
            "callees": ["fopen", "fread", "fclose", "parse_json"],
            "callers": ["main"],
            "basic_blocks": [{"start": "0x140001234"}, {"start": "0x140001250"}],
            "variables": []
        }
    }
    
    suggestions = await renamer.analyze_function(func_context)
    for s in suggestions:
        print(f"Function: {s.original_name} -> {s.suggested_name} (confidence: {s.confidence})")
        print(f"  Reasoning: {s.reasoning}")


if __name__ == "__main__":
    asyncio.run(main())