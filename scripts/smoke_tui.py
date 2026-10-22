#!/usr/bin/env python3
"""Linux/macOS PTY acceptance. Requires pyte; only reads this host, sends no signals."""
import argparse
import re
import sys
import tempfile
import fcntl
import json
import os
import pty
import select
import signal
import struct
import subprocess
import termios
import time
from pathlib import Path
import pyte

parser=argparse.ArgumentParser()
parser.add_argument('binary',nargs='?',default='target/release/rstop')
parser.add_argument('--output',default='/tmp/rstop-qa/tui-results.json')
args=parser.parse_args()
owned=tempfile.TemporaryDirectory(prefix='rstop-qa-')
log_path=Path(owned.name)/'output.log'
error_path=Path(owned.name)/'error.log'
log_file=log_path.open('w')
error_file=error_path.open('w')
writer=subprocess.Popen([sys.executable,'-u','-c',"import sys; print('rstop-output-stdout'); print('rstop-output-stderr',file=sys.stderr); [print(line.strip(),flush=True) for line in sys.stdin]"],stdin=subprocess.PIPE,stdout=log_file,stderr=error_file,text=True)
master,slave=pty.openpty()
fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',48,160,0,0))
original=termios.tcgetattr(slave)
environment={k:v for k,v in os.environ.items() if k!='NO_COLOR'}
process=subprocess.Popen([str(Path(args.binary).resolve()),'--config',str(Path(owned.name)/'config.toml')],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env={**environment,'TERM':'xterm-256color'})
screen=pyte.Screen(160,48)
stream=pyte.Stream(screen)
checks={}
skipped={}

def pump(seconds=.35):
 deadline=time.monotonic()+seconds
 while time.monotonic()<deadline:
  ready,_,_=select.select([master],[],[],min(.05,max(0,deadline-time.monotonic())))
  if ready:
   try:data=os.read(master,65536)
   except OSError:break
   stream.feed(data.decode('utf8',errors='replace'))
 return '\n'.join(screen.display)

def press(keys,seconds=.35):
 os.write(master,keys.encode());return pump(seconds)

def tab_buttons():
 # Inspect label rows; include all surrounding padding and the border in the hit region.
 row=2 if screen.lines>=28 else 1
 return row,[(m.group(1),m.start()-2,m.end()+2) for m in re.finditer(r'\[([0-9EA])\] [A-Za-z]+',screen.display[row])]

def active_at(row,x):
 return screen.buffer[row][x].bold

def process_selection_visible():
 # The selection marker occupies the first table cell. Fractional meter glyphs
 # elsewhere can also contain ▎ and do not imply a selected process.
 for row,line in enumerate(screen.display):
  if 'PID' not in line or 'USER' not in line:continue
  border=line.rfind('│',0,line.index('PID'))
  if border<0:continue
  for data in screen.display[row+1:]:
   if data[border]!='│':break
   if data[border+1]=='▎':return True
 return False

try:
 text=pump(7)
 checks['overview']='CPU' in text and 'MEMORY' in text and 'NETWORK' in text
 checks['starts_unselected']=not process_selection_visible()
 checks['button_navigation']=screen.display[1].startswith('╭') and '[1] Overview' in screen.display[2]
 checks['compact_bottom']='[Esc] Menu' in screen.display[-1] and '[F6] Tools' in screen.display[-1] and '[F10] Quit' in screen.display[-1] and 'Ready ·' not in text
 checks['header_time_window']='Window 3m' in screen.display[0] and 'z In' not in screen.display[0]
 row,buttons=tab_buttons()
 available_keys={key for key,_,_ in buttons}
 checks['contextual_navigation']=not any(label in screen.display[row] for label in ['Explorer','History','Events'])
 for key,start,end in buttons:
  checks[f'tab_{key}_uniform_background']=len({screen.buffer[row][x].bg for x in range(start,end)})==1
  # The label, both sides, and all four corners must choose the same view.
  for x,y in [(start,row),(end-1,row),((start+end-1)//2,row),(start,1),(end-1,1),(start,3),(end-1,3)]:
   text=press(f'\x1b[<0;{x+1};{y+1}M\x1b[<0;{x+1};{y+1}m',.1)
   checks[f'tab_{key}_cell_{x}_{y}']=active_at(row,start+1)
  if end<screen.columns:
   press(f'\x1b[<0;{end+1};{row+1}M\x1b[<0;{end+1};{row+1}m',.1)
   checks[f'tab_{key}_gap']=active_at(row,start+1)
 text=press('1')
 for key,marker in [('2','PROCESSES'),('3','GPU LOAD'),('4','FILESYSTEMS'),('5','CONNECTIONS'),('6','PRESSURE'),('7','WORKLOADS'),('8','SERVICE UNITS')]:
  # Optional page checks are conditional on auto-discovery in this machine.
  if key in ['3','7','8'] and {'3':'GPU','7':'Containers','8':'Services'}[key] not in text:continue
  text=press(key);checks['view_'+key]=marker in text
  if key == '3':
   checks['gpu_versions']='CUDA Driver API' in text and 'NVML' in text
   checks['gpu_default_metrics']=all(label in text for label in ['ENGINES','POWER','TEMPERATURE','PCIE'])
   text=press('T');checks['gpu_history']='ENGINES' in text and 'TEMPERATURE' in text
   text=press('I');checks['gpu_device_info']=all(label in text for label in ['DEVICE INFO','PROCESSES','GPU LOAD'])
   text=press('/pci\r');checks['gpu_pci_field']='PCI Bus-ID' in text
   press('\x1b')
   text=press('/persistence\r');checks['gpu_device_filter']='persistence' in text and ('Off' in text or 'On' in text) and 'PROCESSES' in text
   press('\x1b');text=press('S');checks['gpu_summary_key']='GPU LOAD' in text
  if key in ['7','8']:
   empty=any(message in text for message in ['No containers','No readable container engines','No services','No readable service manager'])
   if empty:
    checks['empty_workload_'+key]=True
    for name in ['live_detail_','management_menu_','management_keys_','management_confirm_']:
     skipped[name+key]='No usable local workload exists; no existing daemon or workload was started for testing.'
   else:
    # The live query has a 4s timeout; include the wrapped beginning, middle and end.
    text=press('\r',4.5)
    extra=text
    for _ in range(10):extra+=press('\x1b[6~',.1)
    checks['live_detail_'+key]='Live inspection' in extra and ('HostPID' in extra if key=='7' else 'MainPID=' in extra)
    press('\x1b')
    text=press('m');checks['management_menu_'+key]='Workload tools' in text and 'restart' in text
    # Only open and cancel a mutating plan. Never execute against existing resources.
    checks['management_keys_'+key]=bool(re.search(r'\[r\]\s+restart',text))
    text=press('r');checks['management_confirm_'+key]='[y] Confirm' in text and 'Confirm workload action' in text
    press('\x1b')
   text=press('s');checks['workload_sort_'+key]='Workload sorting' in text
   press('\x1b')
   text=press('M');checks['workload_ram_key_'+key]='MEMORY' in text and 'Workload sorting' not in text
   text=press('P');checks['workload_cpu_key_'+key]='CPU' in text and 'Workload sorting' not in text
 for key,marker in [('9','APPLICATION INDEX'),('A','APPLICATION INDEX')]:
  if key not in available_keys:continue
  text=press(key);checks['view_'+key]=marker in text
 press('2')
 text=press('C');checks['column_picker']='Process columns' in text and '[x]' in text
 text=press('\x1b[F');checks['column_scroll']='autogroup_id' in text
 press('\x1b')
 text=press('\x1bOS');checks['sort_menu']='Sort by' in text and 'RAM' in text
 text=press('\x1b[B\r');checks['sort_choice']='RAM' in text and 'Sort by' not in text
 text=press('\x1b[17~');checks['process_tools']='Process tools' in text and 'Environment' in text
 press('\x1b')
 table_row=next(i for i,line in enumerate(screen.display) if 'PID' in line and 'USER' in line)
 text=press('\x1bOR');checks['filter_shortcut']=not screen.cursor.hidden and '[Enter] Apply' in screen.display[-1]
 checks['filter_geometry']=next(i for i,line in enumerate(screen.display) if 'PID' in line and 'USER' in line)==table_row
 checks['filter_inline']=screen.cursor.y< table_row and screen.cursor.y>=4
 press('\x1b')
 text=press('s');checks['sort_mouse_menu']='Sort by' in text
 # Locate the actual chooser entry; menu height changes as sort fields grow.
 menu_row=next(i for i,line in enumerate(screen.display) if re.search(r'│\s+VRAM\s+│',line))
 menu_x=screen.display[menu_row].index('VRAM')
 text=press(f'\x1b[<0;{menu_x+1};{menu_row+1}M\x1b[<0;{menu_x+1};{menu_row+1}m');checks['sort_mouse_choice']='Sort by' not in text
 text=press('s');checks['sort_mouse_selected']=bool(re.search(r'▸[^\n]*VRAM',text))
 press('\x1b')
 text=press('\x1b[<2;16;13M\x1b[<2;16;13m');checks['row_context_menu']='Process tools' in text
 press('\x1b')
 text=press('v');checks['tag']='●' in text
 text=press('U');checks['untag']='●' not in text
 text=press('n');checks['nice_prompt']='nice' in text and '[Enter] Continue' in text
 press('\x1b')
 text=press('a');checks['affinity_prompt']='affinity' in text
 press('\x1b')
 text=press('x');checks['trace_prompt']='trace' in text
 press('\x1b')
 text=press('K');checks['signal_targets']='Confirm process signal' in text and 'process(es)' in text
 press('\x1b')
 text=press('/cpu:>0 !name:zzzz\r');checks['query']='Filter: cpu:>0' in text
 press('\x1b')
 text=press('/rstop\r');checks['filter']='Filter: rstop' in text
 text=press('f\x15cargo\x1b');checks['filter_cancel_restores']='Filter: rstop' in text
 text=press('f\x15q\x1b');checks['typed_quit_stays_in_filter']=process.poll() is None and 'Filter: rstop' in text
 text=press('\x1b[3~');checks['delete_clears_filter']='Filter: Search…' in text
 text=press('frsoXp\x1b[H\x1b[C\x1b[Ct\x1b[C\x1b[3~\r');checks['filter_cursor_edit']='Filter: rstop' in text
 text=press('f\x1b[B');checks['filter_down_commits']='[Enter] Apply' not in text and 'Filter: rstop' in text
 text=press('\r');checks['process_detail']='PROCESS ' in text and 'Command' in text
 press('\x1b')
 text=press('o',.8);checks['open_files']='OPEN FILES' in text
 press('\x1b')
 # Environment contents are intentionally never written to the QA report.
 text=press('e',.8);checks['environment']='ENVIRONMENT' in text
 press('\x1b')
 # Follow only the dedicated child created by this script.
 press('\x1b');press('2');text=press(f'/pid:{writer.pid}\r\x1b[B')
 checks['selection_live_resources']=f'PID {writer.pid}' in text and 'Selected' in text and 'GPU ' in text and 'RAM ' in text
 row=next(i for i,line in enumerate(screen.display) if 'PID' in line and 'USER' in line)+4
 text=press(f'\x1b[<0;12;{row+1}M\x1b[<0;12;{row+1}m')
 checks['blank_deselects']=f'PID {writer.pid} · Selected' not in text and not process_selection_visible()
 text=pump(1.5);checks['refresh_stays_unselected']=not process_selection_visible()
 text=press('\x1b[20~');checks['unselected_signal_disabled']='Confirm process signal' not in text
 text=press('\x1b[B');checks['keyboard_reselects']=f'PID {writer.pid}' in text and 'Selected' in text
 text=press('\x1b[18~',1)
 checks['process_output']='OUTPUT · PID' in text and 'rstop-output-stdout' in text and 'rstop-output-stderr' in text
 checks['output_follow_default']='LIVE · [f] Pause' in text
 writer.stdin.write('rstop-output-next\n');writer.stdin.flush()
 text=pump(3.5);checks['output_follow']='rstop-output-next' in text
 text=press('\x1b[A');checks['output_scroll_pauses']='PAUSED · [f] Follow' in text
 text=press('f');checks['output_resume']='LIVE · [f] Pause' in text
 text=press('/rstop-output-stdout\r');checks['output_search']='rstop-output-stdout' in text and 'rstop-output-stderr' not in text
 press('/\x1b')
 text=press('p');checks['output_file_prompt']='log file' in text and 'regular log file' in text
 text=press(str(log_path)+'\r',.8);checks['output_manual_file']='rstop-output-stdout' in text and 'Source:' in text
 writer.stdin.write('x'*140+'far-end-marker\n');writer.stdin.flush();pump(3.5)
 text=press('\x1b[C'*12);checks['output_long_line_pan']='far-end-marker' in text
 press('\x1b[D'*12)
 press('\x1b');press('\x1b');text=pump(3.5)
 checks['output_close_stays_closed']='OUTPUT ·' not in text
 text=press('?');checks['main_menu']='One terminal. Your whole system.' in text and 'Settings' in text and 'Help' in text and 'Ziqi Fan' in text and 'Apache-2.0' in text and 'github.com/fan-ziqi/rstop' in text and 'buymeacoffee.com/robotsfan' in text
 text=press('\x1bOP');checks['help']='KEYBOARD' in text
 text=press('\x1b');checks['help_returns_to_menu']='One terminal. Your whole system.' in text and 'KEYBOARD' not in text
 text=press('\x1bOQ');checks['settings']='PREFERENCES' in text
 text=press('\x1b');checks['settings_return_to_menu']='One terminal. Your whole system.' in text and 'PREFERENCES' not in text
 press('\x1b')
 text=press(' ');checks['pause']='PAUSED' in text
 press(' ')
 text=press('c');checks['theme']='PROCESSES' in text
 # Sampling speed and elapsed chart window are independent keyboard controls.
 text=press('W');checks['window_menu']='Time window' in text and '3m elapsed time' in text
 text=press('3');checks['window_preset']='Window 1m' in screen.display[0]
 text=press('z');checks['window_zoom_in']='Window 30s' in screen.display[0]
 text=press('Z');checks['window_zoom_out']='Window 1m' in screen.display[0]
 text=press('+');checks['sampling_keeps_window']='Window 1m' in screen.display[0] and '0.75s' in screen.display[0]
 press('-')
 text=press('W0');checks['window_all']='Window All' in screen.display[0]
 text=press('W4');checks['window_restore']='Window 3m' in screen.display[0]
 # Clear the existing filter, then dispatch commands directly by their visible key.
 press('\x1b[3~')
 foreground=screen.buffer[0][2].fg
 text=press('\x1b');checks['escape_menu']='One terminal. Your whole system.' in text
 text=press(':');checks['commands']='Commands' in text and 'Time window' in text
 checks['modal_dims_background']=screen.buffer[0][2].fg!=foreground
 text=press('\x1b[6~');checks['commands_page_down']='Commands' in text and '▸' in text
 text=press('\x1b[F');checks['commands_end']=bool(re.search(r'▸\s+[^\n]*Open ',text)) and 'Open Events' not in text
 text=press('W');checks['command_key_dispatch']='Time window' in text and 'elapsed time' in text
 press('\x1b')
 text=press('5R');checks['context_diagnosis']='DIAGNOSE THIS RESOURCE' in text and 'Network' in text
 text=press('\x1b[F')+press('\x1b[5~');checks['context_trends_and_alerts']='RECENT TRENDS' in text and 'RELATED ALERTS' in text
 press('\x1b')
 text=press('\x1b[15~');checks['network_view_menu']='View' in text and 'Focus interfaces' in text
 text=press('\x1b[B\r');checks['network_interfaces']='INTERFACES' in text and 'CONNECTIONS' in text
 # Focus changes the action target while both tables remain on screen.
 text=press('\x1bOS');checks['interface_sort_menu']='Interface sorting' in text and 'Reverse current order' in text
 press('\x1b')
 text=press('D');checks['network_connections']=all(label in text for label in ['CONNECTIONS','INTERFACES','DESTINATION'])
 text=press('\x1b[17~');checks['network_tools_menu']='Network tools' in text and 'Test website or service' in text and 'Capture packets' in text
 text=press('C');checks['capture_interface_prompt']='capture interface' in text
 text=press('\r');checks['capture_filter_prompt']='capture filter' in text
 text=press('\r');checks['capture_output_prompt']='capture output' in text
 text=press('\r');checks['capture_preview']='CONFIRM PACKET CAPTURE' in text and '128 bytes' in text and 'tcpdump' in text
 press('\x1b')
 text=press('4U');checks['folder_scan_prompt']='folder path' in text and '100,000' in text
 press('\x1b');press('2')
 for width,height in [(80,24),(60,18),(160,48)]:
  press('2')
  fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',height,width,0,0))
  screen.resize(lines=height,columns=width)
  os.kill(process.pid,signal.SIGWINCH)
  text=pump(.5);checks[f'resize_{width}x{height}']=process.poll() is None and 'rstop' in text
  checks[f'actions_{width}x{height}']=all(label in screen.display[-1] for label in ['[F3] Filter','[F4] Sort','[F6] Tools','[F10] Quit'])
  text=press('5');checks[f'network_both_{width}x{height}']=all(label in text for label in ['INTERFACES','CONNECTIONS'])
  if '3' in available_keys:
   text=press('3');checks[f'gpu_both_{width}x{height}']=all(label in text for label in ['PROCESSES','DEVICE INFO'])
  press('2')
  row,tabs=tab_buttons()
  for key,start,end in tabs[-1:]:
   for x in [start,end-1]:
    press(f'\x1b[<0;{x+1};{row+1}M\x1b[<0;{x+1};{row+1}m',.15)
    checks[f'resize_tab_{width}_{x}']=screen.buffer[row][start+1].bold
 press('q');process.wait(timeout=3)
 checks['exit_code']=process.returncode==0
 restored=termios.tcgetattr(slave)
 checks['terminal_restored']=restored[3]==original[3]
finally:
 if process.poll() is None:process.terminate();process.wait(timeout=5)
 if writer.poll() is None:writer.terminate();writer.wait(timeout=5)
 writer.stdin.close();log_file.close();error_file.close();owned.cleanup()
 os.close(master);os.close(slave)
Path(args.output).parent.mkdir(parents=True,exist_ok=True)
Path(args.output).write_text(json.dumps(checks,indent=2)+'\n')
Path(args.output).with_suffix('.skipped.json').write_text(json.dumps(skipped,indent=2)+'\n')
print(json.dumps(checks,indent=2))
assert all(checks.values()),'PTY checks failed'
