#!/usr/bin/env python3
"""Render fictional, privacy-safe demo data with rstop's actual SVG exporter."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "website/dist/assets"
VIEWS = ("overview", "processes", "gpu", "storage", "network", "system", "containers", "services")
GIB = 1024 ** 3


def snapshot(t):
    wave = lambda phase, base, amp: base + amp * math.sin(t / 9 + phase) + amp / 3 * math.sin(t / 2.4 + phase)
    jobs = [("python", "python train.py --device cuda", 184, 7.2), ("rustc", "rustc --crate-name engine", 92, 1.4),
            ("node", "node server.mjs", 12, .6), ("postgres", "postgres -D /var/lib/postgresql", 6, 1.1),
            ("nginx", "nginx: worker process", 2, .08), ("docker", "dockerd", 3, .24),
            ("redis-server", "redis-server 127.0.0.1:6379", 1, .12), ("rstop", "rstop", .8, .04)]
    processes = [dict(pid=2400+i, parent=1, start_time=1792600000, name=n, command=c, user="demo", state="Running",
                      cpu=max(.1, wave(i, cpu, cpu*.12)), memory=int(mem*GIB), virtual_memory=int(mem*GIB*2),
                      cpu_time_ms=34000+i*1000, read_per_sec=120000*(i+1), write_per_sec=64000*(i+1),
                      gpu_memory=int(7.2*GIB) if i == 0 else None, gpu_percent=78 if i == 0 else None,
                      gpu_ids=[0] if i == 0 else [], runtime=300+t, nice=0, thread=False, kernel_thread=False)
                 for i, (n,c,cpu,mem) in enumerate(jobs)]
    return dict(timestamp=1792600000+t, elapsed=float(t), collection_ms=8, hostname="rstop-demo", os="Linux · Demo session",
                kernel="6.8.0", uptime=14400+t, cpu_name="32-core workstation", cpu_percent=wave(0,38,13), cpu_mhz=4200,
                cores=[max(0,min(100,wave(i*.4,40+i%5*5,20))) for i in range(32)], core_temperatures=[58.0]*32, load=[4.28,3.84,3.21],
                memory=dict(total=64*GIB,used=int(wave(.2,24,1.8)*GIB),available=38*GIB,cached=16*GIB,swap_total=8*GIB,swap_used=0),
                processes=processes, networks=[dict(name="eth0",mac="02:00:00:00:00:01",address="192.0.2.10",mtu=1500,
                speed_mbps=1000,rx_per_sec=wave(1,24e6,9e6),tx_per_sec=wave(2,4e6,2e6),rx_total=24*GIB,tx_total=4*GIB,state="up")],
                connections=dict(sampled_at=1792600000+t,owners_sampled_at=1792600000+t,status="Illustrative demo connections",
                owner_scan_complete=True,connections=[dict(id=f"demo-{i}",protocol="tcp",local=f"192.0.2.10:{50000+i}",
                remote=f"198.51.100.{i+1}:443",state="ESTABLISHED",pids=[2402],process_names=["node"],
                rx_per_sec=1e6*(i+1),tx_per_sec=120000*(i+1),rtt_ms=12+i,received=30000000,sent=2000000) for i in range(6)]),
                disks=[dict(name="/dev/nvme0n1p1",mount="/",filesystem="ext4",kind="SSD",total=2*1024*GIB,
                used=780*GIB,available=1268*GIB,extra={"inodes_total":1000000,"inodes_free":820000})],
                disk_io=[dict(name="nvme0n1",read_per_sec=wave(.5,280e6,110e6),write_per_sec=wave(1.2,110e6,60e6),
                busy_percent=wave(0,22,10),aggregate=True,read_iops=2400,write_iops=1200,await_ms=.3,queue_depth=.6)],
                gpus=[dict(id=0,name="NVIDIA GeForce RTX 4090",vendor="NVIDIA",backend="NVML",uuid="GPU-DEMO",driver="Demo",
                utilization=round(wave(0,78,12)),memory_utilization=48,memory_used=int(11.4*GIB),memory_total=24*GIB,
                temperature=62,fan=45,power_watts=285,power_limit_watts=450,clock_mhz=2520,memory_clock_mhz=10501,
                encoder=0,decoder=0,pcie_rx_kib=145000,pcie_tx_kib=52000,performance_state="P0",
                processes=[dict(pid=2400,memory=int(7.2*GIB),utilization=78,kind="C")],process_status="Demo compute process")],
                gpu_status="Demo · NVIDIA NVML", sensors=[dict(name="CPU Package",temperature=58,critical=95)],
                inventory=dict(updated_at=1792600000+t,runtimes=["Docker"],service_managers=["systemd"],
                containers=[dict(id=f"demo{i:08d}",runtime="docker",name=n,image=im,state="running",status="Up 4 hours",
                cpu=2.4+i,memory=int((.2+i*.3)*GIB),memory_limit=4*GIB,memory_percent=5+i*7,network_rx=24000000,
                network_tx=4000000,pids=8+i,ports=f"127.0.0.1:{8000+i}->80/tcp") for i,(n,im) in enumerate([
                ("web","nginx:stable"),("api","node:lts"),("database","postgres:17"),("cache","redis:7")])],
                services=[dict(name=n+".service",scope="system",load="loaded",active="active",sub="running",description=d,
                pid=2402+i,cpu_percent=.5+i,memory=int((.03+i*.02)*GIB),tasks=4+i) for i,(n,d) in enumerate([
                ("nginx","Web server"),("docker","Container runtime"),("postgresql","Database server"),("ssh","SSH server"),
                ("systemd-journald","Journal service"),("NetworkManager","Network management")])]))


def compact_svg(path):
    """Merge background cells and same-style glyph runs; preserve the native UI."""
    ns = "{http://www.w3.org/2000/svg}"
    ET.register_namespace("", "http://www.w3.org/2000/svg")
    tree = ET.parse(path)
    for group in tree.getroot().iter():
        prev = None
        for el in list(group):
            if prev is not None and el.tag == prev.tag:
                a, b = dict(prev.attrib), dict(el.attrib)
                if el.tag == ns+"rect" and all(a.get(k)==b.get(k) for k in ("y","height","fill")) and float(a.get("x",-100))+float(a.get("width",0))==float(b.get("x",-200)):
                    prev.set("width",str(int(float(a["width"])+float(b["width"])))); group.remove(el); continue
                if el.tag == ns+"text" and all(a.get(k)==b.get(k) for k in set(a)|set(b) if k != "x") and float(a.get("x",-100))+len(prev.text or "")*8==float(b.get("x",-200)):
                    prev.text=(prev.text or "")+(el.text or "");group.remove(el);continue
            prev=el
    tree.write(path,encoding="unicode")


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--binary",type=Path,default=ROOT/"target/debug/rstop")
    args=parser.parse_args();ASSETS.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="rstop-demo-") as tmp:
        record=Path(tmp)/"demo.jsonl"
        record.write_text("".join(json.dumps(snapshot(t))+"\n" for t in range(120)))
        config=Path(tmp)/"config.toml";config.write_text('history = 180\ntime_window_seconds = 120\n')
        for theme in ("midnight","nord","ember","black","white","graphite","ocean","forest","sand"):
            for view in VIEWS:
                out=ASSETS/f"{view}-{theme}.svg"
                subprocess.run([str(args.binary),"export","svg",str(out),"--input",str(record),"--view",view,"--theme",theme,
                                "--config",str(config),"--width","160","--height","44","--read-only","--no-config-exec"],check=True,capture_output=True)
                compact_svg(out)
                print(out.relative_to(ROOT),flush=True)


if __name__ == "__main__": main()
