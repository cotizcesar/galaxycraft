#!/usr/bin/env python3
"""Split a module built from merged wave files into one file per block.

A wave merge appends each agent's staging file to a module behind a banner

    // ===========================================================================
    // Merged from src/wave31/agent07.rs
    // ===========================================================================

This tool turns `src/<module>.rs` into a directory:

    src/<module>/mod.rs          the head of the old file (inner attributes,
                                 imports, hand-written code) plus the module
                                 list and `pub use` re-exports, so every public
                                 path (`crate::<module>::name`) is unchanged
    src/<module>/w31/mod.rs      `pub mod a07; pub use a07::*;` ...
    src/<module>/w31/a07.rs      the block, verbatim, behind
                                 `use super::super::*;`

Only moves code.  Blocks that used a private item of another block (the old
single file let them) need that item raised to `pub(crate)`; run with
`--fix-visibility` to do that from the compiler's errors, repeatedly, until
the crate builds.

    python3 scripts/split_merged.py engine-rust/src/runtime.rs
    python3 scripts/split_merged.py engine-rust/src/runtime.rs --fix-visibility

Only blocks after the first banner are moved; for a file whose head is
hand-written (sim.rs) pass `--into translated` to put the blocks under
`src/<module>/translated/`.
"""
import argparse
import json
import re
import subprocess
from pathlib import Path

BANNER = re.compile(r"^// =+\n// Merged from (\S+)\n// =+\n", re.M)


def block_path(source: str) -> tuple[str, str]:
    """`src/wave31/agent07.rs` -> ("w31", "a07"); `src/translated/agent41.rs`
    -> ("translated", "a41")."""
    parts = Path(source).parts
    folder, stem = parts[-2], Path(parts[-1]).stem
    m = re.fullmatch(r"wave(\d+)", folder)
    group = f"w{int(m.group(1)):02d}" if m else re.sub(r"\W", "_", folder)
    m = re.fullmatch(r"agent(\d+)", stem)
    name = f"a{int(m.group(1)):02d}" if m else re.sub(r"\W", "_", stem)
    return group, name


def split(module_file: Path, into: str | None) -> None:
    text = module_file.read_text()
    matches = list(BANNER.finditer(text))
    if not matches:
        raise SystemExit(f"{module_file}: no `// Merged from` banners")
    head = text[: matches[0].start()].rstrip() + "\n"
    out_dir = module_file.with_suffix("")
    base = out_dir / into if into else out_dir
    groups: dict[str, list[str]] = {}
    for k, m in enumerate(matches):
        end = matches[k + 1].start() if k + 1 < len(matches) else len(text)
        body = text[m.start() : end].rstrip() + "\n"
        group, name = block_path(m.group(1))
        # `src/translated/agentNN.rs` blocks under `--into translated`.
        if group == into:
            group = "batches"
        while name in groups.get(group, []):
            name += "_b"
        groups.setdefault(group, []).append(name)
        depth = "super::super::super::*" if into else "super::super::*"
        path = base / group / f"{name}.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            "#![allow(unused_imports)]\n"
            f"use {depth};\n\n" + body
        )
    for group, names in groups.items():
        lines = [f"mod {n};\n" for n in names]
        lines += ["\n"] + [f"pub use {n}::*;\n" for n in names]
        # A block with only `impl World` methods or tests exports nothing.
        (base / group / "mod.rs").write_text("#![allow(unused_imports)]\n\n" + "".join(lines))
    listing = []
    if into:
        listing.append(f"\nmod {into};\npub use {into}::*;\n")
        sub = "".join(f"mod {g};\n" for g in groups) + "\n"
        sub += "".join(f"pub use {g}::*;\n" for g in groups)
        (base / "mod.rs").write_text(
            "#![allow(unused_imports)]\nuse super::*;\n\n" + sub
        )
    else:
        listing.append("\n// Wave blocks, one file each (scripts/split_merged.py).\n")
        listing += [f"mod {g};\n" for g in groups]
        listing.append("\n")
        listing += [f"pub use {g}::*;\n" for g in groups]
    out_dir.mkdir(exist_ok=True)
    (out_dir / "mod.rs").write_text(head + "".join(listing))
    module_file.unlink()
    total = sum(len(v) for v in groups.values())
    print(f"{module_file}: {total} blocks in {len(groups)} groups -> {out_dir}/")


ITEM = re.compile(
    r"^(\s*)(?:(?:unsafe|const|async|extern \"C\")\s+)*(fn|const|static|struct|enum|type|trait|mod)\s+(\w+)"
)


def raise_item(files: list[Path], name: str) -> bool:
    """Make the private item `name` `pub(crate)` where it is defined."""
    changed = False
    pat = re.compile(
        rf"^(\s*)((?:#\[[^\]]*\]\s*)*)((?:(?:unsafe|const|async)\s+)*(?:fn|const|static|struct|enum|type|trait)\s+{re.escape(name)}\b)",
        re.M,
    )
    for f in files:
        text = f.read_text()
        new = pat.sub(lambda m: f"{m.group(1)}{m.group(2)}pub(crate) {m.group(3)}", text)
        if new != text:
            f.write_text(new)
            changed = True
    return changed


def fix_visibility(crate_dir: Path, module_dir: Path) -> None:
    files = sorted(module_dir.rglob("*.rs"))
    for round_ in range(20):
        out = subprocess.run(
            ["cargo", "build", "--all-targets", "--message-format=json", "-q"],
            cwd=crate_dir,
            capture_output=True,
            text=True,
        ).stdout
        names: set[str] = set()
        errors = 0
        for line in out.splitlines():
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            m = msg.get("message") or {}
            if m.get("level") != "error":
                continue
            errors += 1
            for pat in (
                r"function `(\w+)` is private",
                r"constant `(\w+)` is private",
                r"struct `(\w+)` is private",
                r"static `(\w+)` is private",
                r"type alias `(\w+)` is private",
                r"enum `(\w+)` is private",
                r"method `(\w+)` is private",
                r"associated function `(\w+)` is private",
                r"cannot find (?:function|value|type|struct|constant) `(\w+)` in this scope",
                r"cannot find (?:function|value|type|struct|constant) `(\w+)` in module",
            ):
                for found in re.findall(pat, m.get("message", "")):
                    names.add(found)
        if errors == 0:
            print(f"builds after {round_} round(s)")
            return
        fixed = [n for n in sorted(names) if raise_item(files, n)]
        print(f"round {round_}: {errors} errors, raised {fixed}")
        if not fixed:
            raise SystemExit("errors left that are not visibility problems")
    raise SystemExit("did not converge")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("module", type=Path)
    ap.add_argument("--into")
    ap.add_argument("--fix-visibility", action="store_true")
    a = ap.parse_args()
    if a.fix_visibility:
        module_dir = a.module.with_suffix("")
        crate_dir = module_dir
        while not (crate_dir / "Cargo.toml").exists():
            crate_dir = crate_dir.parent
        fix_visibility(crate_dir, module_dir)
    else:
        split(a.module, a.into)


if __name__ == "__main__":
    main()
