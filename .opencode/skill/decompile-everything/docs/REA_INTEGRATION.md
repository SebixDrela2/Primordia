# REA Integration Guide

This guide explains how to use the decompile-everything skill with REA's Ghidra provider within OpenCode's code mode.

## Overview

The REA (Reverse Engineering Assistant) MCP server provides access to Ghidra's analysis capabilities through a set of tools. In OpenCode's code mode, these tools are available via the `tools.rea.*` namespace.

## Available REA Ghidra Tools

| Tool | Description |
|------|-------------|
| `open_binary` | Open executable with Ghidra |
| `close_binary` | Close session, optional snapshot |
| `binary_overview` | Get binary metadata |
| `list_procedures` | List all functions |
| `list_names` | List all symbols |
| `list_strings` | Extract strings |
| `list_segments` | Memory layout |
| `inspect_native_exports` | PE/ELF exports |
| `decompile_all` | Bulk decompilation |
| `analyze_function` | Deep function analysis |
| `annotate_native_function` | Rename function + comments |
| `set_address_name` | Rename symbol |
| `set_addresses_names` | Batch rename |
| `inspect_native_api` | Calling conventions, jump tables |
| `inspect_native_data_type` | Type recovery |
| `get_call_graph` | Call graph |
| `match_native_functions` | Binary diffing |
| `compare_functions` | Function comparison |

## Basic Usage in Code Mode

```python
# In OpenCode code mode, the tools namespace is available
# Open a binary
result = await tools.rea.open_binary({
    "path": "/path/to/target.exe",
    "provider_id": "ghidra"
})

# Get binary overview
overview = await tools.rea.binary_overview({})

# List all functions
procedures = await tools.rea.list_procedures({"detail": "complete"})

# Decompile all functions
decompile_result = await tools.rea.decompile_all({
    "output_directory": "/path/to/output",
    "layout": "address"
})

# Analyze specific function
analysis = await tools.rea.analyze_function({
    "procedure": "0x140001300",
    "detail": "complete"
})

# Rename function
await tools.rea.annotate_native_function({
    "procedure": "0x140001300",
    "name": "main_entry",
    "comment": "Main entry point"
})

# Close session with snapshot
await tools.rea.close_binary({
    "snapshot_path": "/path/to/snapshot.ghidra",
    "overwrite": True
})
```

## Complete Workflow Example

```python
async def decompile_binary(tools, binary_path: str, output_dir: str):
    """Complete decompilation workflow."""
    from pathlib import Path
    import json
    
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    # 1. Open binary
    binary_info = await tools.rea.open_binary({"path": binary_path, "provider_id": "ghidra"})
    print(f"Opened: {binary_info}")
    
    # 2. Get overview
    overview = await tools.rea.binary_overview({})
    print(f"Architecture: {overview.get('architecture')}")
    
    # 3. List procedures
    procs_result = await tools.rea.list_procedures({"detail": "complete"})
    functions = procs_result.get("normalized_result", [])
    print(f"Found {len(functions)} functions")
    
    # 4. List symbols
    names_result = await tools.rea.list_names({"detail": "complete"})
    symbols = names_result.get("normalized_result", [])
    
    # 5. List strings
    strings_result = await tools.rea.list_strings({})
    strings = strings_result.get("normalized_result", [])
    
    # 6. Decompile all
    decompile_dir = output_path / "decompilation"
    decompile_result = await tools.rea.decompile_all({
        "output_directory": str(decompile_dir),
        "layout": "address"
    })
    print(f"Decompiled {decompile_result.get('counts', {}).get('written', 0)} functions")
    
    # 7. Deep analyze functions
    analyses = []
    for func in functions[:50]:  # Limit for demo
        addr = func.get("address")
        if addr:
            analysis = await tools.rea.analyze_function({
                "procedure": addr,
                "detail": "complete"
            })
            analyses.append(analysis.get("normalized_result", {}))
    
    # 8. Export results
    export_data = {
        "binary_info": binary_info,
        "overview": overview,
        "functions": functions,
        "symbols": symbols,
        "strings": strings,
        "analyses": analyses
    }
    
    (output_path / "decompilation.json").write_text(json.dumps(export_data, indent=2, default=str))
    
    # 9. Close with snapshot
    snapshot_path = output_path / "snapshot.ghidra"
    await tools.rea.close_binary({"snapshot_path": str(snapshot_path), "overwrite": True})
    
    return export_data

# Run the workflow
result = await decompile_binary(tools, "target.exe", "./analysis_output")
```

## Symbol Renaming Workflow

```python
async def rename_symbols(tools, project_dir: str):
    """Rename unknown symbols using pattern analysis."""
    import json
    from pathlib import Path
    
    project_path = Path(project_dir)
    project_file = project_path / "project_state.json"
    
    # Load project state
    project = json.loads(project_file.read_text())
    functions = project.get("functions", {})
    symbols = project.get("symbols", {})
    
    # Find unknown symbols (FUN_, VAR_, etc.)
    unknown = {addr: sym for addr, sym in symbols.items() 
               if sym.get("is_unknown") and not sym.get("renamed")}
    
    print(f"Found {len(unknown)} unknown symbols")
    
    for addr, sym in unknown.items():
        func = functions.get(addr)
        
        # Generate suggested name
        suggested = generate_name(sym, func)
        
        if suggested and suggested != sym["name"]:
            # Apply rename
            if sym["symbol_type"] == "function":
                await tools.rea.annotate_native_function({
                    "procedure": addr,
                    "name": suggested
                })
            else:
                await tools.rea.set_address_name({
                    "address": addr,
                    "name": suggested
                })
            
            print(f"  {addr}: {sym['name']} -> {suggested}")

def generate_name(symbol, function):
    """Generate meaningful name from context."""
    name = symbol["name"]
    strings = function.get("strings", []) if function else []
    callees = function.get("callees", []) if function else []
    
    # Pattern matching
    for s in strings:
        if "error" in s.lower(): return "handle_error"
        if "init" in s.lower(): return "initialize"
        if "config" in s.lower(): return "load_config"
    
    for c in callees:
        if "malloc" in c.lower(): return "allocate_memory"
        if "free" in c.lower(): return "free_memory"
    
    return None
```

## Advanced Analysis

### Native API Boundary Analysis

```python
# Analyze calling conventions, parameters, jump tables
api_result = await tools.rea.inspect_native_api({
    "procedure": "0x140001300"
})

boundary = api_result.get("normalized_result", {}).get("boundary", {})
if boundary.get("available"):
    print(f"Calling convention: {boundary.get('calling_convention')}")
    print(f"Return type: {boundary.get('return_type', {}).get('data_type')}")
    for param in boundary.get("parameters", []):
        print(f"  Param {param['ordinal']}: {param['name']} ({param['data_type']})")
    for jt in boundary.get("jump_tables", []):
        print(f"  Jump table at {jt['dispatch_address']}: {len(jt['mappings'])} cases")
```

### Data Type Recovery

```python
# Inspect recovered types
type_result = await tools.rea.inspect_native_data_type({
    "type": "struct_MyStruct",
    # or
    "address": "0x140002000"
})

type_data = type_result.get("normalized_result", {})
if type_data.get("status") == "available":
    print(f"Type: {type_data.get('name')}")
    print(f"Kind: {type_data.get('kind')}")
    print(f"Size: {type_data.get('size_bytes')} bytes")
    for field in type_data.get("fields", []):
        print(f"  Field {field['ordinal']}: {field['name']} @ offset {field['offset_bytes']} ({field['type_id']})")
```

### Call Graph Analysis

```python
# Get full call graph
graph = await tools.rea.get_call_graph({})
graph_data = graph.get("normalized_result", {})

nodes = graph_data.get("nodes", [])
edges = graph_data.get("edges", [])

print(f"Call graph: {len(nodes)} functions, {len(edges)} calls")

# Find entry points (no callers)
called = {e["to"] for e in edges}
entry_points = [n for n in nodes if n["address"] not in called]
print(f"Entry points: {[n['name'] for n in entry_points]}")
```

## Binary Comparison

```python
# Compare two binaries
# First, open both binaries and get their procedure lists
left_result = await tools.rea.open_binary({"path": "v1.exe"})
left_procs = await tools.rea.list_procedures({"detail": "complete"})

right_result = await tools.rea.open_binary({"path": "v2.exe"})
right_procs = await tools.rea.list_procedures({"detail": "complete"})

# Match functions
match_result = await tools.rea.match_native_functions({
    "left": left_procs.get("evidence_id"),
    "right": right_procs.get("evidence_id"),
    "detail": "summary"
})

match_data = match_result.get("normalized_result", {})
summary = match_data.get("summary", {})
print(f"Matched: {summary.get('matched')}")
print(f"Only in v1: {summary.get('only_left')}")
print(f"Only in v2: {summary.get('only_right')}")
print(f"Unresolved: {summary.get('unresolved')}")

# Compare specific functions
for match in match_data.get("matches", [])[:5]:
    if match["state"] == "matched" and match.get("compare_functions_input"):
        compare = await tools.rea.compare_functions({
            "left": match["compare_functions_input"]["left"],
            "right": match["compare_functions_input"]["right"]
        })
        print(f"  Similarity: {compare.get('normalized_result', {}).get('similarity')}")
```

## Session Management

```python
# Open with snapshot
await tools.rea.open_binary({
    "path": "target.exe",
    "snapshot_path": "previous_snapshot.ghidra"
})

# ... do analysis ...

# Save new snapshot
await tools.rea.close_binary({
    "snapshot_path": "new_snapshot.ghidra",
    "overwrite": True
})

# Check session status
status = await tools.rea.binary_session({})
print(f"Active: {status.get('active')}")
```

## Error Handling

```python
async def safe_tool_call(tools, tool_name, params):
    """Safely call a REA tool with error handling."""
    try:
        result = await getattr(tools.rea, tool_name)(params)
        if result.get("error"):
            return {"success": False, "error": result["error"]}
        return {"success": True, "data": result}
    except Exception as e:
        return {"success": False, "error": str(e)}

# Usage
result = await safe_tool_call(tools, "analyze_function", {"procedure": "0x140001300"})
if result["success"]:
    print(result["data"])
else:
    print(f"Failed: {result['error']}")
```

## Performance Tips

1. **Batch operations**: Use `set_addresses_names` for bulk renames instead of individual `set_address_name` calls.

2. **Limit decompilation**: Use `limit` parameter in `decompile_all` to process in chunks.

3. **Resume capability**: Set `resume: true` in `decompile_all` to continue interrupted decompilation.

4. **Parallel analysis**: Process multiple functions concurrently using `asyncio.gather`.

5. **Use snapshots**: Save snapshots to avoid re-analyzing from scratch.

```python
# Parallel function analysis
async def analyze_batch(tools, addresses):
    tasks = [
        tools.rea.analyze_function({"procedure": addr, "detail": "complete"})
        for addr in addresses
    ]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return [r for r in results if not isinstance(r, Exception)]
```

## Integration with decompile-everything Skill

The skill provides high-level workflows that use these REA tools internally:

```bash
# Full pipeline (uses all the above tools)
decompile-everything full --binary target.exe --output ./analysis

# Individual stages
decompile-everything analyze --binary target.exe
decompile-everything decompile --project ./analysis
decompile-everything rename --project ./analysis --mode ai
decompile-everything export --project ./analysis --format c
decompile-everything track --project ./analysis
```

## Troubleshooting

### Common Issues

1. **"Provider not found"**: Ensure Ghidra provider is configured in REA
2. **"Session already active"**: Close previous session with `close_binary`
3. **"Decompilation timeout"**: Increase timeout or use `limit` parameter
4. **"Memory error"**: Reduce batch size or use 64-bit Python

### Debugging

Enable detailed logging:
```python
import logging
logging.basicConfig(level=logging.DEBUG)
```

Check REA provider status:
```python
status = await tools.rea.binary_session({})
print(json.dumps(status, indent=2))
```

## Resources

- [REA Documentation](https://github.com/rea/rea)
- [Ghidra API](https://ghidra.re/ghidra_docs/api/)
- [OpenCode Code Mode](https://opencode.ai/docs/code-mode)