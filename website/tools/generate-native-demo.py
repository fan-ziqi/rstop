#!/usr/bin/env python3
"""Compile the real rstop UI into browser-readable terminal cells."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from importlib.util import module_from_spec, spec_from_file_location

ROOT = Path(__file__).resolve().parents[2]
spec = spec_from_file_location("demo_fixture", Path(__file__).with_name("generate-demo.py"))
fixture = module_from_spec(spec)
spec.loader.exec_module(fixture)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "website/dist/assets/native")
    parser.add_argument("--queries-only", action="store_true")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="rstop-native-demo-") as directory:
        work = Path(directory)
        samples = work / "samples.jsonl"
        samples.write_text("".join(json.dumps(fixture.snapshot(t)) + "\n" for t in range(120)))
        source = Path(__file__).with_name("native-demo.rs")
        (work / "Cargo.toml").write_text(f'''[package]
name = "rstop-native-demo"
version = "0.1.0"
edition = "2024"
[[bin]]
name = "rstop-native-demo"
path = {json.dumps(str(source))}
[dependencies]
rstop = {{path = {json.dumps(str(ROOT))}}}
ratatui = {{version = "0.30.2", default-features = false, features = ["crossterm", "layout-cache", "unstable-rendered-line-info"]}}
serde_json = "1"
[patch.crates-io]
crossterm = {{path = {json.dumps(str(ROOT / 'vendor/crossterm'))}}}
''')
        (work / "Cargo.lock").write_bytes((ROOT / "Cargo.lock").read_bytes())
        command = ["cargo", "run", "--offline", "--manifest-path", str(work / "Cargo.toml"),
                   "--target-dir", str(ROOT / "target"), "--", str(samples), str(args.output)]
        if args.queries_only:
            command.append("--queries-only")
        subprocess.run(command, check=True, cwd=ROOT)
        if args.queries_only:
            return
        metadata = args.output / "renderer.json"
        info = json.loads(metadata.read_text())
        info["version"] = next(line.split('"')[1] for line in (ROOT / "Cargo.toml").read_text().splitlines() if line.startswith("version ="))
        info["fixture_sha256"] = hashlib.sha256(samples.read_bytes()).hexdigest()
        source_hash = hashlib.sha256()
        for path in sorted((ROOT / "src/ui").rglob("*.rs")):
            source_hash.update(path.relative_to(ROOT).as_posix().encode())
            source_hash.update(path.read_bytes())
        info["ui_sha256"] = source_hash.hexdigest()
        metadata.write_text(json.dumps(info, separators=(",", ":")) + "\n")

if __name__ == "__main__":
    main()
