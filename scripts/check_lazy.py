#!/usr/bin/env python3
"""Linux PTY acceptance for sticky lazy sources. Requires strace, pyte, Docker CLI and systemd.
Only starts its own read-only monitor; never changes host workloads.
"""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import time
import pyte

parser = argparse.ArgumentParser()
parser.add_argument('binary', nargs='?', default='target/release/rstop')
parser.add_argument('--output', default='artifacts/lazy-acceptance.json')
args = parser.parse_args()
binary = Path(args.binary).resolve()
assert shutil.which('strace'), 'strace is required'
checks = {}
observations = {}
with tempfile.TemporaryDirectory(prefix='rstop-lazy-') as temp:
    trace = Path(temp) / 'connect.log'
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 48, 160, 0, 0))
    process = subprocess.Popen([
        'strace', '-f', '-qq', '-e', 'trace=connect', '-o', str(trace),
        str(binary), '--config', str(Path(temp) / 'config.toml'), '--read-only',
        '--interval', '1000',
    ], stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
        env={**{k: v for k, v in os.environ.items() if k != 'NO_COLOR'},
             'TERM': 'xterm-256color', 'COLORTERM': 'truecolor'})
    screen = pyte.Screen(160, 48)
    stream = pyte.Stream(screen)

    def pump(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            ready, _, _ = select.select([master], [], [], min(.05, max(0, end - time.monotonic())))
            if ready:
                try:
                    stream.feed(os.read(master, 131072).decode(errors='replace'))
                except OSError:
                    break
        return '\n'.join(screen.display)

    def counts():
        text = trace.read_text(errors='replace')
        return {'docker': text.count('docker.sock'),
                'systemd': text.count('/run/dbus/system_bus_socket')}

    try:
        text = pump(3)
        cold = counts()
        observations['unopened'] = cold
        checks['overview_ready'] = 'CPU' in text and 'MEMORY' in text
        checks['unopened_sources_dormant'] = cold == {'docker': 0, 'systemd': 0}
        os.write(master, b'7')
        text = pump(8)
        opened = counts()
        observations['containers_open'] = opened
        checks['first_open_queries_containers'] = opened['docker'] > 0 and 'Loading local Docker' not in text
        checks['services_still_dormant'] = opened['systemd'] == 0
        os.write(master, b'1')
        pump(8)
        away = counts()
        observations['containers_hidden'] = away
        checks['hidden_containers_keep_refreshing'] = away['docker'] > opened['docker']
        os.write(master, b'7')
        text = pump(.3)
        checks['return_uses_live_cache'] = 'WORKLOADS' in text and 'Loading local Docker' not in text
        os.write(master, b'8')
        text = pump(6)
        services = counts()
        observations['services_open'] = services
        checks['first_open_queries_services'] = services['systemd'] > 0 and 'Loading local services' not in text
        os.write(master, b'1')
        pump(8)
        observations['services_hidden'] = counts()
        checks['hidden_services_keep_refreshing'] = observations['services_hidden']['systemd'] > services['systemd']
        os.write(master, b'q')
        process.wait(timeout=5)
        checks['clean_exit'] = process.returncode == 0
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=5)
        os.close(master)
        os.close(slave)
report = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
          'checks': checks, 'connections': observations,
          'method': 'Owned read-only PTY under connect-only strace; counts are connection attempts, not successful API results.'}
Path(args.output).parent.mkdir(parents=True, exist_ok=True)
Path(args.output).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
assert all(checks.values()), 'Lazy source checks failed'
