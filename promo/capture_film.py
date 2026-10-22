#!/usr/bin/env python3
"""Capture native rstop views and keyboard interactions from fictional telemetry."""
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time
from xml.sax.saxutils import escape

import pyte

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/promo/v2/native'
BIN = ROOT / 'target/debug/rstop'
VIEWS = ('overview', 'processes', 'gpu', 'storage', 'network', 'system', 'containers', 'services')
THEMES = ('midnight', 'nord', 'ember', 'black', 'white', 'graphite', 'ocean', 'forest', 'sand')


def dataset():
    spec = importlib.util.spec_from_file_location('demo', ROOT / 'website/tools/generate-demo.py')
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    rows = []
    for t in range(120):
        row = module.snapshot(t)
        row['pressure'] = [dict(resource=name, some_10=a, some_60=a*.7, some_300=a*.5,
                                full_10=b, full_60=b*.6, full_300=b*.4)
                           for name, a, b in [('cpu', 2.4, 0.0), ('memory', .3, .1), ('io', .8, .2)]]
        row['sensors'] = [dict(name=name, temperature=temp, critical=crit)
                          for name, temp, crit in [('CPU Package', 58.0, 95), ('GPU Core', 62.0, 90), ('NVMe SSD', 41.0, 80)]]
        row['inventory']['listeners'] = [dict(protocol='tcp', address='127.0.0.1', port=p, uid=1000)
                                          for p in (5432, 6379, 8080)]
        row['processes'][1]['parent'] = 2400
        row['processes'][2]['parent'] = 2400
        row['processes'][4]['parent'] = 2405
        row['processes'][6]['parent'] = 2405
        rows.append(row)
    return rows


def export(rows, config):
    for count in (80, 90, 100, 110, 120):
        recording = OUT / 'demo.jsonl'
        recording.write_text(''.join(json.dumps(row)+'\n' for row in rows[:count]))
        for view in VIEWS:
            dest = OUT / f'{view}-{count}.svg'
            subprocess.run([str(BIN), 'export', 'svg', str(dest), '--input', str(recording), '--config', str(config),
                            '--view', view, '--width', '160', '--height', '44', '--read-only', '--no-config-exec'],
                           capture_output=True, check=True)
    for theme in THEMES:
        subprocess.run([str(BIN), 'export', 'svg', str(OUT/f'theme-{theme}.svg'), '--input', str(recording),
                        '--config', str(config), '--view', 'overview', '--theme', theme, '--width', '160',
                        '--height', '44', '--read-only', '--no-config-exec'], capture_output=True, check=True)


def terminal_svg(screen):
    color_names = dict(black='000000', red='cc6666', green='b5bd68', brown='f0c674', blue='81a2be',
                       magenta='b294bb', cyan='8abeb7', white='c5c8c6')
    def color(value, default):
        if value == 'default': return default
        return '#'+color_names.get(value, value)
    result = ['<svg xmlns="http://www.w3.org/2000/svg" width="1312" height="824" viewBox="0 0 1312 824">',
              '<rect width="100%" height="100%" fill="#0a0f19"/>',
              '<g font-family="DejaVu Sans Mono,monospace" font-size="13" xml:space="preserve">']
    for y in range(screen.lines):
        for x in range(screen.columns):
            cell = screen.buffer[y][x]; px = 16+x*8; py = 16+y*18
            fg = color(cell.fg, '#d7e2f1'); bg = color(cell.bg, '#0a0f19')
            if cell.reverse: fg, bg = bg, fg
            if bg != '#0a0f19': result.append(f'<rect x="{px}" y="{py}" width="8" height="18" fill="{bg}"/>')
            glyph = cell.data
            if not glyph.strip(): continue
            if 0x2800 <= ord(glyph[0]) <= 0x28ff:
                dots = ord(glyph[0])-0x2800
                for bit, dx, dy in ((0,2,2),(1,2,6),(2,2,10),(3,6,2),(4,6,6),(5,6,10),(6,2,14),(7,6,14)):
                    if dots & (1<<bit): result.append(f'<circle cx="{px+dx}" cy="{py+dy}" r=".85" fill="{fg}"/>')
            else:
                weight = ' font-weight="bold"' if cell.bold else ''
                result.append(f'<text x="{px}" y="{py+14}" fill="{fg}"{weight}>{escape(glyph)}</text>')
    return ''.join(result)+ '</g></svg>'


def interactions(config):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH',44,160,0,0))
    environment = {**os.environ, 'TERM':'xterm-256color', 'COLORTERM':'truecolor'}
    environment.pop('NO_COLOR', None)
    process = subprocess.Popen([str(BIN), 'replay', str(OUT/'demo.jsonl'), '--config', str(config),
                                '--interval','250', '--read-only', '--no-config-exec'],
                               stdin=slave, stdout=slave, stderr=slave, start_new_session=True, env=environment)
    screen = pyte.Screen(160,44); stream = pyte.Stream(screen)
    def pump(seconds=.22):
        until = time.monotonic()+seconds
        while time.monotonic()<until:
            ready,_,_=select.select([master],[],[],min(.025,max(0,until-time.monotonic())))
            if ready:
                try: data=os.read(master,65536)
                except OSError: break
                stream.feed(data.decode('utf-8',errors='replace'))
    def press(keys): os.write(master,keys.encode()); pump()
    captured = {}
    def save(name, expected):
        # A native resize forces a complete repaint rather than an ANSI diff.
        # This prevents unsupported emulator optimizations from leaving stale glyphs.
        for columns in (159, 160):
            screen.reset(); screen.resize(lines=44, columns=columns)
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH',44,columns,0,0))
            os.kill(process.pid, signal.SIGWINCH); pump(.18)
        body='\n'.join(screen.display)
        assert expected in body, (name, body[:500])
        (OUT/f'{name}.svg').write_text(terminal_svg(screen))
        captured[name] = body
    try:
        pump(1.5); press('\x1b[19~')  # F8: pause the fictional recording.
        press('2'); save('processes-native', 'PROCESSES')
        press('/cpu:>10\r'); save('processes-filter', 'Filter: cpu:>10')
        press('\x1b'); press('\x1b[3~'); press('M'); save('processes-memory', 'PROCESSES')
        press('t'); save('processes-tree', 'PROCESSES')
        press('t'); press('P'); press('\x1b[B'); save('processes-selected', 'Selected')
        press('C'); save('process-columns', 'Process columns')
        press('\x1b'); press('\x1b'); press('3'); save('gpu-native', 'GPU LOAD')
        press('1')
        for name, key in [('overview-history-a', '.'), ('overview-history-b', '.'), ('overview-history-c', ',')]:
            press(key); save(name, 'REPLAY')
    finally:
        if process.poll() is None:
            press('q')
            try: process.wait(timeout=3)
            except subprocess.TimeoutExpired: process.terminate();process.wait(timeout=3)
        os.close(master);os.close(slave)
    (OUT/'keyboard-captures.json').write_text(json.dumps(captured,indent=2))


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    config=OUT/'demo-config.toml'
    config.write_text('theme = "midnight"\nhistory = 180\ntime_window_seconds = 120\ncheck_updates = false\n')
    rows=dataset();export(rows,config);interactions(config)
    subprocess.run([str(BIN),'export','csv',str(OUT/'metrics.csv'),'--input',str(OUT/'demo.jsonl'),
                    '--config',str(config),'--samples','3','--read-only','--no-config-exec'],check=True,capture_output=True)
    print('Captured 8 views, 9 themes and native keyboard interactions:',OUT)


if __name__=='__main__':main()
