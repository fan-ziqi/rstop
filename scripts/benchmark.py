#!/usr/bin/env python3
"""Linux PTY benchmark, sequential randomized rounds. Never controls host workloads.
Usage: python3 scripts/benchmark.py --manifest tools.json --output artifacts/benchmark.json
Manifest: {"name": {"command":["binary","flags"],"version":["binary","--version"]}}
"""
import argparse,fcntl,hashlib,json,os,pathlib,pty,random,select,shutil,signal,statistics,struct,subprocess,termios,time,platform
p=argparse.ArgumentParser();p.add_argument('--manifest',required=True);p.add_argument('--output',required=True);p.add_argument('--rounds',type=int,default=3);p.add_argument('--warmup',type=float,default=4);p.add_argument('--duration',type=float,default=10);p.add_argument('--seed',type=int,default=614);a=p.parse_args()
def stop_requested(signum, frame):
 raise KeyboardInterrupt
signal.signal(signal.SIGTERM, stop_requested)

tools=json.loads(pathlib.Path(a.manifest).read_text());ticks=os.sysconf('SC_CLK_TCK');page=os.sysconf('SC_PAGE_SIZE');results=[]

def tree(root):
 todo=[root];seen=set();records=[]
 while todo:
  pid=todo.pop()
  if pid in seen:continue
  seen.add(pid)
  try:
   fields=pathlib.Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
   cpu=sum(int(fields[i]) for i in [11,12])/ticks;reaped=sum(int(fields[i]) for i in [13,14])/ticks;rss=int(fields[21])*page
   records.append((pid,cpu,reaped,rss))
   for task in pathlib.Path(f'/proc/{pid}/task').iterdir():
    try:todo.extend(map(int,(task/'children').read_text().split()))
    except (OSError,ValueError):pass
  except (OSError,ValueError,IndexError):pass
 return records

def run(name,spec,round_index):
 master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',48,160,0,0))
 config=pathlib.Path(a.output).resolve().parent/'benchmark-config'/f'{name}-{round_index}';(config/'btop').mkdir(parents=True,exist_ok=True)
 (config/'btop/btop.conf').write_text('update_ms = 1000\n')
 env={**{k:v for k,v in os.environ.items() if k!='NO_COLOR'},'TERM':'xterm-256color','COLORTERM':'truecolor','LC_ALL':'C.UTF-8','XDG_CONFIG_HOME':str(config),'HTOPRC':str(config/'htoprc')}
 began=time.monotonic();proc=subprocess.Popen(spec['command'],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env=env);first=None;output=bytearray();samples=[];next_sample=began;last_pss=0;pss=None;frame_bytes=0;baseline=None;end=None;input_sent=False
 try:
  deadline=began+a.warmup+a.duration
  while time.monotonic()<deadline:
   ready,_,_=select.select([master],[],[],.025)
   if ready:
    try:chunk=os.read(master,131072)
    except OSError:chunk=b''
    if chunk:
     if first is None:first=(time.monotonic()-began)*1000
     frame_bytes+=len(chunk)
     if len(output)<300000:output.extend(chunk[:300000-len(output)])
   now=time.monotonic()
   if 'initial_keys' in spec and not input_sent and now-began>=spec.get('input_delay_seconds',3):
    os.write(master,spec['initial_keys'].encode());input_sent=True
   if proc.poll() is not None:break
   if now>=next_sample:
    records=tree(proc.pid);by_pid={pid:(cpu,rss) for pid,cpu,_,rss in records};root=by_pid.get(proc.pid,(0,0));total=sum(cpu+reaped for _,cpu,reaped,_ in records)
    if now-began>=a.warmup:
     if baseline is None:baseline=(now,root[0],total,frame_bytes)
     if now-last_pss>=1:
      value=0;readable=True
      for pid,_,_,_ in records:
       try:
        lines=pathlib.Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines();value+=next(int(s.split()[1])*1024 for s in lines if s.startswith('Pss:'))
       except (OSError,StopIteration):readable=False
      pss=value if readable else None;last_pss=now
     samples.append({'seconds':now-began,'root_rss':root[1],'tree_rss':sum(rss for _,_,_,rss in records),'tree_pss':pss,'root_cpu_seconds':root[0],'tree_cpu_seconds':total,'helpers':len(records)-1})
     end=(now,root[0],total,frame_bytes)
    next_sample=now+.2
  valid=proc.poll() is None and baseline is not None and end is not None and len(samples)>3
  row={'tool':name,'round':round_index,'command':spec['command'],'terminal_bytes':frame_bytes,'first_output_ms':first,'completed':valid,'exit_before_end':proc.returncode,'samples':samples}
  if 'initial_keys' in spec:
   row['initial_input']={'keys':spec['initial_keys'],'delay_seconds':spec.get('input_delay_seconds',3)}
   row['initial_input_sent']=input_sent
   row['expected_state_observed']=all(text in output.decode(errors='replace') for text in spec.get('expected_terminal_text',[]))
   row['completed']=valid and input_sent and row['expected_state_observed']
  if valid:
   elapsed=end[0]-baseline[0];row.update(root_cpu_percent=100*(end[1]-baseline[1])/elapsed,tree_cpu_percent=100*(end[2]-baseline[2])/elapsed,root_rss_mib=statistics.median(s['root_rss'] for s in samples)/2**20,tree_rss_peak_mib=max(s['tree_rss'] for s in samples)/2**20,tree_pss_mib=statistics.median(s['tree_pss'] for s in samples if s['tree_pss'] is not None)/2**20 if any(s['tree_pss'] is not None for s in samples) else None,terminal_kib_per_sec=(end[3]-baseline[3])/elapsed/1024)
  else:row['terminal_excerpt']=output.decode(errors='replace')[-5000:]
  return row
 finally:
  if proc.poll() is None:
   try:os.write(master,b'q')
   except OSError:pass
   try:proc.wait(timeout=2)
   except subprocess.TimeoutExpired:
    os.killpg(proc.pid,signal.SIGTERM)
    try:proc.wait(timeout=2)
    except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
  os.close(master);os.close(slave)

versions={}
for name,spec in tools.items():
 try:versions[name]=subprocess.run(spec.get('version',[spec['command'][0],'--version']),capture_output=True,text=True,timeout=15).stdout.strip()[:1600]
 except Exception as e:versions[name]=str(e)
metadata={'date_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'platform':platform.platform(),'cpu_model':next((s.split(':',1)[1].strip() for s in pathlib.Path('/proc/cpuinfo').read_text().splitlines() if s.startswith('model name')),''),'logical_cpus':os.cpu_count(),'memory_mib':int(next(s.split()[1] for s in pathlib.Path('/proc/meminfo').read_text().splitlines() if s.startswith('MemTotal:')))/1024,'load_start':os.getloadavg(),'tty':[160,48],'warmup_seconds':a.warmup,'measurement_seconds':a.duration,'rounds':a.rounds,'seed':a.seed,'sample_interval_seconds':.2,'versions':versions,'method':'Root CPU excludes children. Tree CPU includes child CPU reaped by each process plus currently living descendants; RSS summed across descendants, PSS sampled once/s where readable. First output includes terminal control bytes. Defaults differ; not feature-equivalent.'}
metadata['color_environment']={'TERM':'xterm-256color','COLORTERM':'truecolor','NO_COLOR':'unset'}
metadata['binary_builds']={name:{'path':str(pathlib.Path(shutil.which(spec['command'][0]) or spec['command'][0]).resolve()),'sha256':hashlib.sha256(pathlib.Path(shutil.which(spec['command'][0]) or spec['command'][0]).read_bytes()).hexdigest()} for name,spec in tools.items()}
def save():
 pathlib.Path(a.output).parent.mkdir(parents=True,exist_ok=True);pathlib.Path(a.output).write_text(json.dumps({'metadata':metadata,'runs':results},indent=2)+'\n')
rng=random.Random(a.seed)
for round_index in range(1,a.rounds+1):
 order=list(tools);rng.shuffle(order)
 for name in order:
  print(f'round {round_index}/{a.rounds}: {name}',flush=True)
  try:row=run(name,tools[name],round_index)
  except Exception as e:row={'tool':name,'round':round_index,'completed':False,'error':str(e)}
  results.append(row);save()
  print(json.dumps({k:v for k,v in row.items() if k not in ['samples','terminal_excerpt','command']}),flush=True)
metadata['load_end']=os.getloadavg();save()
