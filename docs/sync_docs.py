"""Inject the generated element table into docs/SIMULATION.md.

Run from the project root:  python docs/sync_docs.py
"""
import io
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOC = os.path.join(ROOT, "docs", "SIMULATION.md")
MARKER = "<!-- ELEMENT_TABLE -->"

def main():
    out = subprocess.run(
        ["cargo", "run", "--release", "--bin", "docgen"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if out.returncode != 0:
        sys.stderr.write(out.stderr)
        return 1
    table = out.stdout.strip()

    doc = io.open(DOC, encoding="utf-8").read()
    if MARKER not in doc:
        sys.stderr.write("marker %s not found\n" % MARKER)
        return 1
    start = doc.index(MARKER)
    end = doc.index("---", start)
    doc = doc[:start] + MARKER + "\n\n" + table + "\n\n" + doc[end:]
    io.open(DOC, "w", encoding="utf-8", newline="\n").write(doc)
    print("element table synced (%d rows)" % (table.count("\n") - 1))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
