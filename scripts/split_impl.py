#!/usr/bin/env python3
"""Move the methods of one big `impl` block into child module files.

    python3 scripts/split_impl.py engine-rust/src/sim/mod.rs World \\
        --plan scripts/sim_split.json

The plan maps a child file to name patterns (regexes matched against the
method name, first match wins); methods matching nothing stay in the
original block.  Each child file gets `use super::*;` and its own
`impl World { ... }`.  Methods that were private become `pub(super)` (a
child module's private method is not visible to its siblings), which keeps
them inside the parent module, as before.  `#[cfg(test)] mod tests { .. }`
can be moved to `tests.rs` with `--tests`.

Only moves code; comments, attributes and doc comments travel with the
method that follows them.
"""
import argparse
import json
import re
from pathlib import Path


def scan(text: str):
    """Yield (index, char) for code characters, skipping comments, strings,
    char literals and raw strings."""
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        if text.startswith("/*", i):
            depth, i = 1, i + 2
            while i < n and depth:
                if text.startswith("/*", i):
                    depth, i = depth + 1, i + 2
                elif text.startswith("*/", i):
                    depth, i = depth - 1, i + 2
                else:
                    i += 1
            continue
        m = re.match(r'b?r(#*)"', text[i : i + 12])
        if m and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            end = '"' + m.group(1)
            j = text.find(end, i + len(m.group(0)))
            i = n if j < 0 else j + len(end)
            continue
        if c == '"' or (c == "b" and text.startswith('b"', i)):
            i += 2 if c == "b" else 1
            while i < n and text[i] != '"':
                i += 2 if text[i] == "\\" else 1
            i += 1
            continue
        if c == "'":
            # char literal vs lifetime
            m = re.match(r"'(\\.[^']*|[^\\'])'", text[i : i + 12])
            if m:
                i += len(m.group(0))
                continue
        yield i, c
        i += 1


def matching_brace(text: str, open_idx: int) -> int:
    depth = 0
    for i, c in scan_from(text, open_idx):
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced")


def scan_from(text: str, start: int):
    for i, c in scan(text[start:]):
        yield start + i, c


def methods(body: str):
    """Split an impl body into (leading_text, name, item_text) at depth 0."""
    out = []
    depth = 0
    nest = 0  # ( and [ outside braces: `-> [i32; 8]` is not an item end
    item_start = 0
    for i, c in scan(body):
        if c in "([" and depth == 0:
            nest += 1
        elif c in ")]" and depth == 0:
            nest -= 1
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                out.append(body[item_start:end])
                item_start = end
        elif c == ";" and depth == 0 and nest == 0:
            end = i + 1
            out.append(body[item_start:end])
            item_start = end
    tail = body[item_start:]
    items = []
    for chunk in out:
        m = re.search(r"\bfn\s+(\w+)", chunk)
        name = m.group(1) if m else None
        items.append((name, chunk))
    return items, tail


def make_visible(chunk: str) -> str:
    """`fn x` -> `pub(super) fn x` for a private method (after its doc
    comments and attributes)."""
    lines = chunk.split("\n")
    for k, line in enumerate(lines):
        s = line.lstrip()
        if not s or s.startswith("//") or s.startswith("#"):
            continue
        if re.match(r"(pub\b|pub\()", s):
            return chunk
        if re.match(r"((const|unsafe|async)\s+)*fn\s", s):
            indent = line[: len(line) - len(s)]
            lines[k] = indent + "pub(super) " + s
        return "\n".join(lines)
    return chunk


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("file", type=Path)
    ap.add_argument("type_name")
    ap.add_argument("--plan", type=Path, required=True)
    ap.add_argument("--tests", action="store_true")
    a = ap.parse_args()
    plan = json.loads(a.plan.read_text())
    text = a.file.read_text()
    m = re.search(rf"^impl {re.escape(a.type_name)} \{{", text, re.M)
    if not m:
        raise SystemExit("impl block not found")
    open_idx = m.end() - 1
    close_idx = matching_brace(text, open_idx)
    body = text[open_idx + 1 : close_idx]
    items, tail = methods(body)
    keep, groups = [], {g: [] for g in plan}
    for name, chunk in items:
        target = None
        if name:
            for g, pats in plan.items():
                if any(re.fullmatch(p, name) for p in pats):
                    target = g
                    break
        (groups[target] if target else keep).append((name, chunk))
    new_body = "".join(c for _, c in keep) + tail
    text = text[: open_idx + 1] + new_body + text[close_idx:]
    out_dir = a.file.parent
    mods = []
    for g, lst in groups.items():
        if not lst:
            continue
        chunks = "".join(make_visible(c) for _, c in lst).strip("\n")
        (out_dir / f"{g}.rs").write_text(
            f"//! `impl {a.type_name}`: {plan[g] and g.replace('_', ' ')} "
            "(moved out of `mod.rs` by scripts/split_impl.py).\n\n"
            "use super::*;\n\n"
            f"impl {a.type_name} {{\n{chunks}\n}}\n"
        )
        mods.append(g)
        print(f"{g}.rs: {len(lst)} methods")
    print(f"kept in {a.file.name}: {sum(1 for n, _ in keep if n)} methods")
    if a.tests:
        tm = re.search(r"^#\[cfg\(test\)\]\nmod tests \{", text, re.M)
        if tm:
            o = tm.end() - 1
            c = matching_brace(text, o)
            inner = text[o + 1 : c]
            inner = "\n".join(
                line[4:] if line.startswith("    ") else line for line in inner.strip("\n").split("\n")
            )
            (out_dir / "tests.rs").write_text(inner + "\n")
            text = text[: tm.start()] + "#[cfg(test)]\nmod tests;" + text[c + 1 :]
            print("tests -> tests.rs")
    decl = "".join(f"mod {g};\n" for g in mods)
    text = text.rstrip("\n") + "\n\n// `impl World` split by subsystem (scripts/split_impl.py).\n" + decl
    a.file.write_text(text)


if __name__ == "__main__":
    main()
