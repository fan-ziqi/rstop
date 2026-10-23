#!/usr/bin/env python3
"""Check release artifacts and the public CLI without modifying host workloads."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SHELLS = {"bash": "rstop.bash", "zsh": "_rstop", "fish": "rstop.fish",
          "powershell": "rstop.ps1", "elvish": "rstop.elv"}


def workflow(check=False):
    """Gate cargo-dist releases on new tags and native installation acceptance."""
    path = ROOT / ".github/workflows/release.yml"
    text = path.read_text(encoding="utf-8")
    before, announce = text.split("\n  announce:\n", 1)
    original = "if: ${{ always() && needs.host.result == 'success' }}"
    guarded = "if: ${{ always() && needs.host.result == 'success' && needs.custom-install-checks.result == 'success' }}"
    if check:
        assert guarded in announce, "Release publication must require all native installation checks"
    elif original in announce:
        announce = announce.replace(original, guarded, 1)
    else:
        assert guarded in announce, "Unrecognized cargo-dist announce condition"
    original_title = 'ANNOUNCEMENT_TITLE: "${{ fromJson(needs.host.outputs.val).announcement_title }}"'
    tagged_title = 'ANNOUNCEMENT_TITLE: "${{ needs.plan.outputs.tag }}"'
    if check:
        assert tagged_title in announce, "Release title must match the v-prefixed tag"
    elif original_title in announce:
        announce = announce.replace(original_title, tagged_title, 1)
    else:
        assert tagged_title in announce, "Unrecognized cargo-dist release title"
    for job, first_step in (("plan", 'runs-on: "ubuntu-22.04"'),
                            ("custom-checks", "uses: ./.github/workflows/checks.yml")):
        original = f"\n  {job}:\n    {first_step}"
        guarded = (f"\n  {job}:\n"
                   "    if: ${{ github.event_name == 'pull_request' || github.event.created }}\n"
                   f"    {first_step}")
        if check:
            assert guarded in before, f"Release {job} must run only for PRs or newly created tags"
        elif guarded not in before:
            assert original in before, f"Unrecognized cargo-dist {job} job"
            before = before.replace(original, guarded, 1)
    if not check:
        path.write_text(before + "\n  announce:\n" + announce, encoding="utf-8")
    print("Release trigger and installation gates verified")


def run(binary, *args, env=None, check=True):
    result = subprocess.run([str(Path(binary).resolve()), *map(str, args)],
                            capture_output=True, timeout=120, env=env)
    if check and result.returncode:
        raise RuntimeError(result.stderr.decode(errors="replace"))
    return result


def docs(binary, check=False):
    outputs = {ROOT / "packaging/rstop.1": run(binary, "man").stdout}
    for shell, name in SHELLS.items():
        outputs[ROOT / "packaging/completions" / name] = run(binary, "completions", shell).stdout
    for path, data in outputs.items():
        data = data.replace(b"\r\n", b"\n")
        data = b"\n".join(line.rstrip(b" \t") for line in data.split(b"\n"))
        if check:
            if not path.exists() or path.read_bytes().replace(b"\r\n", b"\n") != data:
                raise RuntimeError(f"Stale generated documentation: {path.relative_to(ROOT)}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    print("Manual and five shell completions verified" if check else "Generated manual and completions")


def smoke(binary, version=None):
    version = version or re.search(r'^version = "([^"]+)"$', (ROOT / "Cargo.toml").read_text(encoding="utf-8"), re.M)[1]
    assert run(binary, "--version").stdout.decode().strip() == f"rstop {version}"
    help_text = run(binary, "--help").stdout.decode()
    for command in ("monitor", "snapshot", "sample", "record", "replay", "export", "gpu",
                    "config", "completions", "man", "update"):
        assert command in help_text, command
    with tempfile.TemporaryDirectory(prefix="rstop-cli-") as temp:
        root = Path(temp)
        config = root / "config.toml"
        common = ["--config", config, "--read-only", "--no-gpu", "--interval", "250"]
        # Commands unrelated to telemetry must not load a broken configuration.
        config.write_text("invalid = [")
        assert run(binary, "config", "path", "--config", config).stdout.decode().strip() == str(config)
        assert b".TH" in run(binary, "man", "--config", config).stdout
        assert b"rstop" in run(binary, "completions", "bash", "--config", config).stdout
        config.unlink()
        snapshot = json.loads(run(binary, "snapshot", *common).stdout)
        assert snapshot["timestamp"] > 0 and "processes" in snapshot and "memory" in snapshot
        samples = run(binary, "sample", "--samples", "2", *common).stdout.splitlines()
        assert len(samples) == 2
        recording = root / "session.jsonl"
        run(binary, "record", recording, "--samples", "2", *common)
        assert len(recording.read_text(encoding="utf-8").splitlines()) == 2
        replay = run(binary, "replay", recording, "--samples", "1", *common)
        assert json.loads(replay.stdout)["timestamp"] > 0
        csv = root / "metrics.csv"
        svg = root / "screen.svg"
        run(binary, "export", "csv", csv, "--input", recording, *common)
        run(binary, "export", "svg", svg, "--input", recording, *common)
        assert csv.read_text(encoding="utf-8").startswith("timestamp,path,value\n")
        assert "<svg" in svg.read_text(encoding="utf-8")
        # Export must never overwrite the source recording, even through another name.
        before = recording.read_bytes()
        assert run(binary, "export", "csv", recording, "--input", recording, *common,
                   check=False).returncode != 0
        assert recording.read_bytes() == before
        run(binary, "config", "write", *common)
        assert re.search(r'^interval_ms\s*=\s*250$', config.read_text(encoding="utf-8"), re.M)
        # No receipt or a receipt for a different executable must fail before networking.
        env = {**os.environ, "AXOUPDATER_CONFIG_PATH": temp,
               "HTTP_PROXY": "http://127.0.0.1:1", "HTTPS_PROXY": "http://127.0.0.1:1"}
        assert run(binary, "update", "--check", env=env, check=False).returncode != 0
        (root / "rstop-receipt.json").write_text(json.dumps({
            "install_prefix": str(root / "other-copy"), "binaries": ["rstop"],
            "source": {"app_name": "rstop", "owner": "fan-ziqi", "name": "rstop", "release_type": "github"},
            "version": version, "provider": {"source": "cargo-dist", "version": "0.33.1"}
        }))
        (root / "other-copy").mkdir()
        mismatch = run(binary, "update", env=env, check=False)
        assert mismatch.returncode != 0
        assert b"not the installer-managed copy" in mismatch.stderr
    print("Public commands, recording/replay, exports, configuration, and update guards passed")


def safe_member(name):
    path = PurePosixPath(name.replace("\\", "/"))
    if path.is_absolute() or ".." in path.parts or (path.parts and ":" in path.parts[0]):
        raise RuntimeError(f"Unsafe archive member: {name}")


def archive(directory, target, version=None, check_cli=True):
    directory = Path(directory)
    suffix = ".zip" if "windows" in target else ".tar.xz"
    path = directory / f"rstop-{target}{suffix}"
    checksum = path.with_name(path.name + ".sha256")
    expected = checksum.read_text().split()[0]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == expected, "Archive checksum mismatch"
    with tempfile.TemporaryDirectory(prefix="rstop-archive-") as temp:
        if suffix == ".zip":
            with zipfile.ZipFile(path) as source:
                for name in source.namelist():
                    safe_member(name)
                source.extractall(temp)
        else:
            with tarfile.open(path) as source:
                for member in source.getmembers():
                    safe_member(member.name)
                    if member.issym() or member.islnk():
                        raise RuntimeError(f"Unexpected archive link: {member.name}")
                source.extractall(temp, filter="data")
        files = list(Path(temp).rglob("*"))
        names = {file.name for file in files if file.is_file()}
        required = {"LICENSE", "NOTICE", "README.md", "THIRD-PARTY-LICENSES.html", "rstop.1",
                    "rstop.bash", "_rstop", "rstop.fish", "rstop.ps1", "rstop.elv"}
        assert required <= names, f"Missing release files: {required - names}"
        assert "rstop-update" not in names and "rstop-update.exe" not in names
        executable = "rstop.exe" if "windows" in target else "rstop"
        binaries = [file for file in files if file.is_file() and file.name == executable]
        assert len(binaries) == 1
        if check_cli:
            smoke(binaries[0], version)
        result = {"binary_sha256": hashlib.sha256(binaries[0].read_bytes()).hexdigest(),
                  "licenses_sha256": hashlib.sha256(next(file for file in files if file.name == "THIRD-PARTY-LICENSES.html").read_bytes()).hexdigest()}
    print(f"Verified release archive and checksum: {target}")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    command = commands.add_parser("workflow")
    command.add_argument("--check", action="store_true")
    for name in ("docs", "smoke"):
        command = commands.add_parser(name)
        command.add_argument("--binary", required=True)
        if name == "docs":
            command.add_argument("--check", action="store_true")
        else:
            command.add_argument("--expected-version")
    command = commands.add_parser("archive")
    command.add_argument("--directory", required=True)
    command.add_argument("--target", required=True)
    command.add_argument("--expected-version")
    args = parser.parse_args()
    if args.command == "workflow":
        workflow(args.check)
    elif args.command == "docs":
        docs(args.binary, args.check)
    elif args.command == "smoke":
        smoke(args.binary, args.expected_version)
    else:
        archive(args.directory, args.target, args.expected_version)


if __name__ == "__main__":
    main()
