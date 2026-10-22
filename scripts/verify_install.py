#!/usr/bin/env python3
"""Verify a native release through its actual installer, without building Rust."""
import argparse
import contextlib
import functools
import hashlib
import http.server
import json
import os
from pathlib import Path
import platform
import re
import struct
import subprocess
import tempfile
import threading
import time

import distribution
import terminal_smoke

RELEASES = "https://github.com/fan-ziqi/rstop/releases"


def run(command, env=None):
    result = subprocess.run(list(map(str, command)), env=env, capture_output=True, timeout=120)
    if result.returncode:
        raise RuntimeError(f"{command[0]} failed ({result.returncode}): "
                           + result.stdout.decode(errors="replace")[-2000:]
                           + result.stderr.decode(errors="replace")[-4000:])
    return result.stdout.decode(errors="replace")


def download(url, path):
    protocols = "=http,https" if url.startswith("http://127.0.0.1:") else "=https"
    run(["curl.exe" if os.name == "nt" else "curl", "--proto", protocols, "--tlsv1.2",
         "-fLsS", "--retry", "2", "--connect-timeout", "15", "--max-time", "90", url, "-o", path])


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def architecture(binary, target):
    with binary.open("rb") as file:
        data = file.read(4096)
    arm = target.startswith("aarch64-")
    if data.startswith(b"\x7fELF"):
        machine = struct.unpack_from("<H" if data[5] == 1 else ">H", data, 18)[0]
        assert machine == (183 if arm else 62), f"Wrong ELF machine: {machine}"
    elif data.startswith(b"MZ"):
        offset = struct.unpack_from("<I", data, 0x3c)[0]
        assert data[offset:offset + 4] == b"PE\0\0"
        machine = struct.unpack_from("<H", data, offset + 4)[0]
        assert machine == (0xaa64 if arm else 0x8664), f"Wrong PE machine: {machine:#x}"
    else:
        magic, machine = struct.unpack_from("<II", data)
        assert magic == 0xfeedfacf and machine == (0x100000c if arm else 0x1000007), "Wrong Mach-O architecture"


@contextlib.contextmanager
def artifact_server(directory):
    if directory is None:
        yield None
        return
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(directory.resolve()))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            thread.join(timeout=5)


def install(root, version, base, custom=False):
    root.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    for key in ("RSTOP_DOWNLOAD_URL", "INSTALLER_DOWNLOAD_URL", "RSTOP_INSTALL_DIR", "RSTOP_UNMANAGED_INSTALL",
                "CARGO_DIST_FORCE_INSTALL_DIR", "RSTOP_INSTALLER_GITHUB_BASE_URL", "RSTOP_INSTALLER_GHE_BASE_URL",
                "RSTOP_DISABLE_UPDATE", "RSTOP_GITHUB_TOKEN"):
        env.pop(key, None)
    env.update(XDG_CONFIG_HOME=str(root / "config"), AXOUPDATER_CONFIG_PATH=str(root / "config/rstop"),
               RSTOP_NO_MODIFY_PATH="1", TERM="xterm-256color", COLORTERM="truecolor")
    if os.name != "nt":
        env["HOME"] = str(root)
    if custom or (os.name == "nt" and env.get("GITHUB_ACTIONS") != "true"):
        env["RSTOP_INSTALL_DIR"] = str(root / "bin")
    if base.startswith("http://127.0.0.1:"):
        env["RSTOP_DOWNLOAD_URL"] = base
    suffix = "ps1" if os.name == "nt" else "sh"
    script = root / f"install.{suffix}"
    download(f"{base}/rstop-installer.{suffix}", script)
    command = ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script] if os.name == "nt" else ["sh", script]
    run(command, env)
    receipt_path = root / "config/rstop/rstop-receipt.json"
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    assert receipt["version"] == version
    binary = Path(receipt["install_prefix"]) / ("rstop.exe" if os.name == "nt" else "rstop")
    assert binary.is_file() and run([binary, "--version"], env).strip() == f"rstop {version}"
    assert not any(path.name in {"rstop-update", "rstop-update.exe"} for path in binary.parent.iterdir())
    return binary, env, command, receipt_path


def verify(args, base, root):
    report = {"version": args.version, "target": args.target, "source": "artifacts" if args.artifacts else "published",
              "host": platform.platform(), "checks": {}}
    archive_dir = root / "archives"
    archive_dir.mkdir()
    extension = ".zip" if "windows" in args.target else ".tar.xz"
    archive = archive_dir / f"rstop-{args.target}{extension}"
    download(f"{base}/{archive.name}", archive)
    download(f"{base}/{archive.name}.sha256", archive.with_name(archive.name + ".sha256"))
    info = distribution.archive(archive_dir, args.target, args.version, check_cli=False)
    report["archive_sha256"], report["licenses_sha256"] = sha(archive), info["licenses_sha256"]
    report["checks"]["archive_checksum_and_contents"] = True
    binary, env, command, receipt_path = install(root / "fresh", args.version, base)
    if not args.artifacts:
        latest = root / ("latest.ps1" if os.name == "nt" else "latest.sh")
        download(f"{RELEASES}/latest/download/rstop-installer.{latest.suffix[1:]}", latest)
        assert sha(latest) == sha(command[-1]), "Latest and versioned installers differ"
        report["checks"]["public_latest_installer"] = True
    architecture(binary, args.target)
    assert sha(binary) == info["binary_sha256"], "Installer did not install the native release binary"
    report["binary_sha256"] = sha(binary)
    report["checks"]["native_install_and_receipt"] = True
    distribution.smoke(binary, args.version)
    report["checks"]["installed_cli_record_replay_exports_update_guards"] = True
    report["terminal"] = terminal_smoke.check([str(binary), "--read-only", "--no-gpu", "--interval", "250",
                                               "--config", str(root / "terminal.toml")], env)
    report["checks"]["interactive_terminal_startup_and_keyboard_exit"] = True
    run(command, env)
    assert sha(binary) == info["binary_sha256"]
    report["checks"]["reinstall"] = True
    if not args.artifacts:
        for options in (["update", "--check"], ["update"]):
            assert "up to date" in run([binary, *options], env).lower()
            assert sha(binary) == info["binary_sha256"]
        report["checks"]["public_update_check_and_noop"] = True
        if args.upgrade_from:
            old_base = f"{RELEASES}/download/v{args.upgrade_from}"
            old, old_env, _, old_receipt = install(root / "upgrade", args.upgrade_from, old_base, custom=True)
            for attempt in range(5):
                if "update is available" in run([old, "update", "--check"], old_env).lower():
                    break
                time.sleep(2)
            else:
                raise RuntimeError("Previous release did not detect the published update")
            run([old, "update"], old_env)
            assert run([old, "--version"], old_env).strip() == f"rstop {args.version}"
            assert sha(old) == info["binary_sha256"]
            assert json.loads(old_receipt.read_text(encoding="utf-8"))["version"] == args.version
            report["checks"]["upgrade_from_previous_release"] = True
            report["upgrade_from"] = args.upgrade_from
    assert json.loads(receipt_path.read_text(encoding="utf-8"))["version"] == args.version
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--upgrade-from")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.version = args.version.removeprefix("v")
    assert re.fullmatch(r"\d+\.\d+\.\d+", args.version), "Expected a stable semantic version"
    if args.upgrade_from and args.upgrade_from != "previous":
        args.upgrade_from = args.upgrade_from.removeprefix("v")
        assert re.fullmatch(r"\d+\.\d+\.\d+", args.upgrade_from)
    with tempfile.TemporaryDirectory(prefix="rstop install ") as directory, artifact_server(args.artifacts) as server:
        if args.upgrade_from == "previous":
            releases = Path(directory) / "releases.json"
            download("https://api.github.com/repos/fan-ziqi/rstop/releases?per_page=50", releases)
            current = tuple(map(int, args.version.split(".")))
            previous = []
            for release in json.loads(releases.read_text(encoding="utf-8")):
                tag = release["tag_name"].removeprefix("v")
                if not release["draft"] and not release["prerelease"] and re.fullmatch(r"\d+\.\d+\.\d+", tag):
                    version = tuple(map(int, tag.split(".")))
                    if version < current:
                        previous.append((version, tag))
            args.upgrade_from = max(previous)[1] if previous else None
        base = server or f"{RELEASES}/download/v{args.version}"
        report = verify(args, base, Path(directory))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
