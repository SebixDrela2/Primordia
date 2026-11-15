"""Turn a `cargo flamegraph` SVG into the "which function takes longest" table.

    cargo flamegraph --profile profiling --bin primordia -o flamegraph.svg
    python docs/flamegraph_summary.py flamegraph.svg

The SVG that inferno writes contains one frame per unique stack, with the
sample count in the frame's title and the absolute sample range in the
`fg:x` / `fg:w` attributes. That is enough to rebuild the call tree and
to work out, for every function:

  * inclusive time - samples in the function and everything it called
  * self time     - samples in the function itself

which is exactly the per-function table a sampling profiler is for.
No `#[instrument]`, no source changes.
"""

import collections
import html
import re
import sys

FRAME_RE = re.compile(
    r'<g><title>(?P<title>.*?)</title>'
    r'<rect x="(?P<x>[\d.]+)%" y="(?P<y>[\d.]+)" width="(?P<w>[\d.]+)%"'
    r' height="(?P<h>[\d.]+)"'
    r'[^>]*?fg:x="(?P<fx>\d+)" fg:w="(?P<fw>\d+)"'
)

TOTAL_RE = re.compile(r'total_samples="(\d+)"')
ROW_HEIGHT = 16.0


class Frame:
    __slots__ = ("name", "module", "samples", "start", "depth", "children",
                 "self_samples", "parent")

    def __init__(self, name, module, samples, start, depth):
        self.name = name
        self.module = module
        self.samples = samples
        self.start = start
        self.depth = depth
        self.children = []
        self.self_samples = samples
        self.parent = None

    def label(self):
        return self.name if not self.module else "%s`%s" % (self.module, self.name)


def split_module(label):
    """inferno writes `module`function` for native frames."""
    if "`" in label:
        module, _, rest = label.partition("`")
        return module, rest
    return "", label


def parse(path):
    with open(path, encoding="utf-8", errors="replace") as handle:
        svg = handle.read()

    raw = []
    for match in FRAME_RE.finditer(svg):
        raw.append(
            (
                html.unescape(match.group("title")),
                float(match.group("y")),
                int(match.group("fx")),
                int(match.group("fw")),
            )
        )
    if not raw:
        raise SystemExit("no frames found in " + path)

    # inferno stacks the rows: the root sits on the bottom row and every
    # level deeper is one row (16 px) higher.
    bottom = max(y for _, y, _, _ in raw)
    frames = []
    for title, y, start, samples in raw:
        label = re.sub(r"\s*\([\d,]+ samples?, [\d.]+%\)\s*$", "", title).strip()
        module, name = split_module(label)
        frames.append(
            Frame(name, module, samples, start, int(round((bottom - y) / ROW_HEIGHT)))
        )

    total_match = TOTAL_RE.search(svg)
    total = int(total_match.group(1)) if total_match else sum(f.samples for f in frames)

    by_depth = collections.defaultdict(list)
    for frame in frames:
        by_depth[frame.depth].append(frame)
    for level in by_depth:
        by_depth[level].sort(key=lambda f: f.start)

    roots = []
    for depth in sorted(by_depth):
        for frame in by_depth[depth]:
            if depth == 0:
                roots.append(frame)
                continue
            # The parent is the frame one row up whose span contains this
            # one. Several rows-up frames can contain the same span (a
            # parent and its only child have identical spans), so take the
            # tightest container rather than the first or last match.
            parent = None
            for candidate in by_depth[depth - 1]:
                if candidate.start <= frame.start < candidate.start + candidate.samples:
                    if parent is None or candidate.samples < parent.samples:
                        parent = candidate
            if parent is None:
                roots.append(frame)
            else:
                parent.children.append(frame)
                frame.parent = parent

    for frame in frames:
        frame.self_samples = frame.samples - sum(c.samples for c in frame.children)

    return frames, roots, total


def pct(part, whole):
    return 100.0 * part / whole if whole else 0.0


def compare(old_path, new_path):
    """Print the difference between two flame graphs.

        python docs/flamegraph_summary.py --compare before.svg after.svg
    """
    old_frames, _, old_total = parse(old_path)
    new_frames, _, new_total = parse(new_path)

    old_self = collections.Counter()
    new_self = collections.Counter()
    for f in old_frames:
        old_self[f.label()] += f.self_samples
    for f in new_frames:
        new_self[f.label()] += f.self_samples

    def pct(part, whole):
        return 100.0 * part / whole if whole else 0.0

    print("before: %d samples   after: %d samples" % (old_total, new_total))
    print()
    print("=== biggest movers (self time) ===")
    rows = []
    for name in set(old_self) | set(new_self):
        before = pct(old_self[name], old_total)
        after = pct(new_self[name], new_total)
        if before < 0.05 and after < 0.05:
            continue
        rows.append((after - before, before, after, name))
    rows.sort()
    for delta, before, after, name in rows[:20]:
        print("  %+6.2f%%   %6.2f%% -> %6.2f%%   %s" % (delta, before, after, name[:64]))
    print("  ...")
    for delta, before, after, name in rows[-20:]:
        print("  %+6.2f%%   %6.2f%% -> %6.2f%%   %s" % (delta, before, after, name[:64]))
    print()
    print("=== totals by area ===")

    def area(counter, total):
        buckets = collections.Counter()
        for name, hits in counter.items():
            if "brain" in name:
                buckets["brain"] += hits
            elif "render" in name:
                buckets["render"] += hits
            elif "creature" in name:
                buckets["creature"] += hits
            elif "world" in name:
                buckets["world"] += hits
            elif "macroquad" in name or "miniquad" in name:
                buckets["macroquad"] += hits
            else:
                buckets["other"] += hits
        return {k: pct(v, total) for k, v in buckets.items()}

    a = area(old_self, old_total)
    b = area(new_self, new_total)
    for key in sorted(set(a) | set(b)):
        print("  %-10s %6.2f%% -> %6.2f%%   (%+.2f%%)"
              % (key, a.get(key, 0.0), b.get(key, 0.0), b.get(key, 0.0) - a.get(key, 0.0)))

    return 0


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1

    if sys.argv[1] == "--compare":
        return compare(sys.argv[2], sys.argv[3])

    frames, roots, total = parse(sys.argv[1])

    incl = collections.Counter()
    self_ = collections.Counter()
    calls = collections.Counter()
    for frame in frames:
        incl[frame.label()] += frame.samples
        self_[frame.label()] += frame.self_samples
        calls[frame.label()] += 1

    print("total samples: %d   distinct frames: %d   distinct functions: %d"
          % (total, len(frames), len(incl)))
    print()

    top = int(sys.argv[2]) if len(sys.argv) > 2 else 30

    print("=== SELF time (samples in the function itself) ===")
    for name, hits in self_.most_common(top):
        if hits <= 0:
            continue
        print("  %6.2f%% %8d  %s" % (pct(hits, total), hits, name))
    print()

    print("=== INCLUSIVE time (function + everything it called) ===")
    for name, hits in incl.most_common(top):
        print("  %6.2f%% %8d  x%-4d %s" % (pct(hits, total), hits, calls[name], name))
    print()

    if roots:
        root = max(roots, key=lambda f: f.samples)
        path = []
        node = root
        while node is not None:
            path.append(node)
            node = max(node.children, key=lambda f: f.samples) if node.children else None
        print("=== hottest path (%.2f%% of all samples) ===" % pct(root.samples, total))
        for node in path:
            print("  %6.2f%%  %s" % (pct(node.samples, total), node.label()))
        print()

        print("=== top-level breakdown ===")
        for node in sorted(roots, key=lambda f: f.samples, reverse=True)[:15]:
            if node.samples <= 0:
                continue
            print("  %6.2f%% %8d  %s" % (pct(node.samples, total), node.samples, node.label()))
        print()

    print("=== hottest child of each hot function ===")
    for name, hits in incl.most_common(20):
        matches = [f for f in frames if f.label() == name]
        if not matches:
            continue
        best = max(matches, key=lambda f: f.samples)
        kids = sorted(best.children, key=lambda f: f.samples, reverse=True)
        detail = ", ".join("%s %.2f%%" % (c.label(), pct(c.samples, total)) for c in kids[:3])
        print("  %-52s %6.2f%%  ->  %s" % (name[:52], pct(best.samples, total), detail or "(leaf)"))

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
