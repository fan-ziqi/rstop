#!/usr/bin/env python3
"""Render a readable 4K feature film with native UI camera moves and a gentle score."""
import argparse
from collections import deque
from concurrent.futures import ProcessPoolExecutor
from functools import lru_cache
from io import BytesIO
import math
from pathlib import Path
import subprocess
import wave

import cairosvg
import imageio_ffmpeg
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'artifacts/promo/v2'
NATIVE=OUT/'native'
FPS=30
DPI=2
WIDTH,HEIGHT=3840,2160
RESAMPLE=getattr(Image,'Resampling',Image)
TRANSFORM=getattr(Image,'Transform',Image)
WHITE=(239,245,242)
MINT=(170,226,195)
GRAY=(153,174,162)
BG=(8,13,11)
FONT='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
BOLD='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
MONO='/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
CHAPTERS=[
    (0,4,'rstop'),(4,12,'Overview'),(12,25,'Processes'),(25,34,'GPU'),(34,41,'Storage'),
    (41,49,'Network'),(49,56,'System'),(56,63,'Containers'),(63,70,'Services'),
    (70,80,'Themes'),(80,89,'Record, replay and export'),(89,94,'rstop')
]
DURATION=CHAPTERS[-1][1]


def smooth(v):v=max(0,min(1,v));return v*v*(3-2*v)
def interpolate(a,b,q):return tuple(x+(y-x)*q for x,y in zip(a,b))


@lru_cache(maxsize=100)
def font(size,bold=False,mono=False):
    return ImageFont.truetype(MONO if mono else BOLD if bold else FONT,round(size*DPI))


@lru_cache(maxsize=200)
def text_layer(value,size,color=WHITE,bold=False,mono=False):
    face=font(size,bold,mono);box=face.getbbox(value)
    im=Image.new('RGBA',(box[2]-box[0]+8,box[3]-box[1]+8))
    ImageDraw.Draw(im).text((4-box[0],4-box[1]),value,font=face,fill=color)
    return im


def text(im,value,x,y,size,color=WHITE,bold=False,mono=False,center=False,opacity=1):
    layer=text_layer(value,size,color,bold,mono)
    if opacity<1:
        layer=layer.copy();layer.putalpha(layer.getchannel('A').point(lambda a:round(a*max(0,opacity))))
    im.alpha_composite(layer,(round(x*DPI-(layer.width/2 if center else 0)),round(y*DPI)))


@lru_cache(maxsize=4)
def backdrop(style='dark'):
    # The stage stays quiet; all the visual movement is in the product camera.
    h,w=HEIGHT,WIDTH; yy,xx=np.mgrid[0:h,0:w]
    glow=np.exp(-(((xx/w-.48)/.58)**2+((yy/h-.65)/.85)**2)*3)
    a=np.empty((h,w,4),dtype=np.uint8);a[:,:,3]=255
    for i,(base,gain) in enumerate(zip(BG,(3,9,5))):a[:,:,i]=base+glow*gain
    return Image.fromarray(a,'RGBA')


def raster_name(name):return NATIVE/(name+'.png')


def prepare_rasters():
    for path in sorted(NATIVE.glob('*.svg')):
        png=path.with_suffix('.png')
        # Three raster pixels per native cell pixel preserve glyph edges during a 4K push-in.
        correct_size=png.exists() and Image.open(png).size==(3936,2472)
        if not correct_size or png.stat().st_mtime<path.stat().st_mtime:
            cairosvg.svg2png(url=str(path),write_to=str(png),scale=3)
    print('Native 3x rasters ready.',flush=True)


@lru_cache(maxsize=4)
def native(name):return Image.open(raster_name(name)).convert('RGBA')


def camera(im,name,position):
    cx,cy,zoom=position
    source=native(name)
    x,y,w,h=64,190,1792,756
    # zoom is displayed pixels per native SVG pixel at 1080p; no stretching or skewing.
    a=3/(DPI*zoom)
    matrix=(a,0,3*(cx-w/(2*zoom)),0,a,3*(cy-h/(2*zoom)))
    image=source.transform((w*DPI,h*DPI),TRANSFORM.AFFINE,matrix,resample=RESAMPLE.BICUBIC,
                           fillcolor=(10,15,25,255))
    mask=Image.new('L',image.size)
    ImageDraw.Draw(mask).rounded_rectangle((0,0,image.width-1,image.height-1),radius=14*DPI,fill=255)
    image.putalpha(mask);im.alpha_composite(image,(x*DPI,y*DPI))
    ImageDraw.Draw(im).rounded_rectangle((x*DPI,y*DPI,(x+w)*DPI,(y+h)*DPI),radius=14*DPI,
                                        outline=(86,118,99,150),width=DPI)


def trajectory(t,keys):
    if t<=keys[0][0]:return keys[0][1]
    for i in range(1,len(keys)):
        if t<=keys[i][0]:
            start,a=keys[i-1];end,b=keys[i]
            return interpolate(a,b,smooth((t-start)/(end-start)))
    return keys[-1][1]


def heading(im,index,name,detail):
    text(im,f'{index:02d} / 08',66,44,18,MINT,mono=True)
    text(im,name,62,88,54,WHITE,bold=True)
    text(im,detail,65,978,26,GRAY)
    text(im,'rstop',65,1040,15,MINT,mono=True)
    text(im,'Native UI / Demo data',1580,1041,14,(108,138,120),mono=True)
    draw=ImageDraw.Draw(im)
    for i in range(8):
        x=650+i*78
        draw.rounded_rectangle((x*DPI,1047*DPI,(x+54)*DPI,1050*DPI),radius=DPI,
                               fill=MINT if index==i+1 else (48,66,55))


def view_frame(scene,t):
    im=backdrop().copy()
    view=CHAPTERS[scene][2].lower()
    sample=(80,90,100,110,120)[min(4,int(t/1.5))]
    name=f'{view}-{sample}'
    if scene==1:
        keys=[(0,(656,412,.91)),(.65,(656,412,.91)),(1.9,(420,280,2.4)),(3.1,(420,280,2.4)),
              (4.45,(335,630,2.6)),(5.65,(335,630,2.6)),(7.15,(985,630,2.6)),(8,(985,630,2.6))]
        detail='CPU history' if t<3.6 else 'Memory and GPU' if t<6.3 else 'Storage and network'
    elif scene==2:
        if t<2.6:name='processes-native';detail='Resource usage by process'
        elif t<4.8:name='processes-filter';detail='/  cpu:>10'
        elif t<6.8:name='processes-memory';detail='M  Sort by memory'
        elif t<8.9:name='processes-tree';detail='t  Process tree'
        elif t<11.0:name='processes-selected';detail='Select a process. Inspect its resources.'
        else:name='process-columns';detail='C  Choose process columns'
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.55,(415,237,2.55)),(2.6,(415,237,2.55)),
              (3.25,(415,217,2.65)),(6.8,(415,217,2.65)),(7.5,(670,265,2.65)),(8.9,(670,265,2.65)),
              (9.55,(1020,300,2.65)),(10.95,(1020,300,2.65)),(11.55,(650,300,2.65)),(13,(650,300,2.65))]
    elif scene==3:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.75,(460,160,2.45)),(3.25,(460,160,2.45)),
              (4.5,(430,430,2.55)),(5.6,(430,430,2.55)),(6.9,(1040,450,2.55)),(9,(1040,450,2.55))]
        detail='Utilization and VRAM' if t<3.6 else 'Engines, thermals, power and PCIe' if t<6 else 'Device information'
    elif scene==4:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.65,(650,240,2.25)),(3.1,(650,240,2.25)),
              (4.35,(900,635,2.55)),(7,(900,635,2.55))]
        detail='Read and write history' if t<3.5 else 'IOPS, latency and queue depth'
    elif scene==5:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.8,(490,190,2.55)),(3,(490,190,2.55)),
              (4.3,(450,415,2.55)),(5.4,(450,415,2.55)),(6.6,(990,415,2.55)),(8,(990,415,2.55))]
        detail='Interface traffic' if t<3.4 else 'Connections and process ownership' if t<5.8 else 'Rates, round-trip time and endpoints'
    elif scene==6:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.7,(350,340,2.55)),(3.1,(350,340,2.55)),
              (4.5,(995,260,2.55)),(7,(995,260,2.55))]
        detail='Host information and temperatures' if t<3.6 else 'CPU, memory and I/O pressure'
    elif scene==7:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.7,(420,237,2.55)),(3.15,(420,237,2.55)),
              (4.5,(990,237,2.55)),(7,(990,237,2.55))]
        detail='Local workloads and resource usage' if t<3.6 else 'Images, limits and workload labels'
    else:
        keys=[(0,(656,412,.91)),(.55,(656,412,.91)),(1.7,(390,240,2.55)),(3.15,(390,240,2.55)),
              (4.5,(995,240,2.55)),(7,(995,240,2.55))]
        detail='Service state and resource usage' if t<3.6 else 'Tasks, descriptions and labels'
    camera(im,name,trajectory(t,keys));heading(im,scene,CHAPTERS[scene][2],detail)
    return im


def themes_frame(t):
    im=backdrop().copy()
    themes=['midnight','nord','ember','black','white','graphite','ocean','forest','sand']
    # Each theme gets a full, readable preview rather than nine tiny thumbnails.
    index=min(8,int(max(0,t-.7)/.96));theme=themes[index]
    camera(im,'theme-'+theme,trajectory(t,[(0,(656,412,.91)),(.6,(656,412,.91)),(1.55,(350,625,2.3)),(10,(350,625,2.3))]))
    text(im,'09 THEMES',65,45,18,MINT,mono=True)
    text(im,'Make it yours.',62,88,54,WHITE,bold=True)
    text(im,'rstop --theme '+theme,65,978,27,WHITE,mono=True)
    draw=ImageDraw.Draw(im)
    for i,word in enumerate(themes):
        x=65+i*197
        text(im,word.title(),x,1040,16,MINT if i==index else (92,115,101))
        if i==index:draw.line((x*DPI,1069*DPI,(x+120)*DPI,1069*DPI),fill=MINT,width=2*DPI)
    return im


def cli_frame(t):
    im=backdrop().copy()
    text(im,'RECORD / REPLAY / EXPORT',65,45,18,MINT,mono=True)
    text(im,'Keep the evidence.',62,88,54,WHITE,bold=True)
    if t<4.8:
        rows=[('$ rstop record session.jsonl',WHITE),('$ rstop replay session.jsonl',MINT)]
        for i,(value,color) in enumerate(rows):
            count=int(max(0,t-i*1.6)*35)
            text(im,value[:count],85,265+i*95,43,color,mono=True)
        camera(im,'overview-'+('80' if t<3 else '120'),(985,625,2.55))
        # Commands sit in their own narrow strip, away from the native plot.
        overlay=Image.new('RGBA',(1792*DPI,160*DPI),(12,21,16,250))
        text(overlay,'$ rstop record session.jsonl',24,22,34,WHITE,mono=True)
        shown='$ rstop replay session.jsonl'[:int(max(0,t-1.2)*32)]
        text(overlay,shown,24,86,34,MINT,mono=True)
        im.alpha_composite(overlay,(64*DPI,190*DPI))
        text(im,'Record a session. Replay it offline.',65,978,26,GRAY)
    else:
        draw=ImageDraw.Draw(im)
        draw.rounded_rectangle((64*DPI,190*DPI,1856*DPI,946*DPI),radius=14*DPI,fill=(10,15,25),outline=(86,118,99),width=DPI)
        lines=[('$ rstop export csv metrics.csv --input session.jsonl',36,WHITE),
               ('$ head -n 4 metrics.csv',31,MINT),
               ('timestamp,cpu_percent,memory_used,memory_total',27,GRAY)]
        csv=(NATIVE/'metrics.csv').read_text().splitlines()
        # Show a compact, real subset of the actual exported rows.
        import csv as csv_module
        parsed=list(csv_module.DictReader(csv))
        fields=['timestamp','path','value']
        if fields:
            lines[2]=(','.join(fields),27,GRAY)
            selected=[row for row in parsed if row['path'] in ('system.cpu_percent','system.memory.used','system.memory.total')]
            for row in selected[:3]:lines.append((','.join(row[k] for k in fields),27,WHITE))
        for i,(value,size,color) in enumerate(lines):text(im,value,94,255+i*81,size,color,mono=True)
        text(im,'$ rstop export svg screen.svg --input session.jsonl',97,825,27,MINT,mono=True)
        text(im,'JSON / CSV / SVG',65,978,26,GRAY)
    text(im,'rstop',65,1040,15,MINT,mono=True)
    return im


def brand_frame(t,end=False):
    im=backdrop().copy();enter=smooth(t/.8)
    text(im,'rstop',960,315+15*(1-enter),132,WHITE,mono=True,center=True,opacity=enter)
    text(im,'One terminal. Your whole system.',960,540,39,MINT,center=True,opacity=smooth((t-.3)/.8))
    if end:
        text(im,'Linux / macOS / Windows',960,697,25,GRAY,center=True)
        text(im,'fan-ziqi.github.io/rstop',960,805,29,WHITE,mono=True,center=True)
    return im


def frame(index):
    t=index/FPS
    scene=next((i for i,(start,end,_) in enumerate(CHAPTERS) if start<=t<end),len(CHAPTERS)-1)
    start,end,_=CHAPTERS[scene];local=t-start
    if scene==0:im=brand_frame(local)
    elif scene==9:im=themes_frame(local)
    elif scene==10:im=cli_frame(local)
    elif scene==11:im=brand_frame(local,True)
    else:im=view_frame(scene,local)
    # A brief soft cut keeps text readable; the UI itself never dissolves into double text.
    fade=min(smooth(local/.22),smooth((end-t)/.22)) if scene not in (0,11) else min(smooth(t/.6),smooth((DURATION-t)/1))
    if fade<1:im=Image.blend(backdrop(),im,fade)
    return im.convert('RGB')


def render_frame(index):return frame(index).tobytes()


def score(path):
    rate=48000;n=DURATION*rate;t=np.arange(n)/rate
    melody=np.zeros(n);pad=np.zeros(n)
    # Soft C-major / F-major / A-minor / G-sus colors. No bass pulses, percussion or noise.
    progression=[(261.626,329.628,391.995,493.883),(261.626,349.228,440,523.251),
                 (261.626,329.628,440,523.251),(293.665,391.995,440,587.33)]
    for chord_index,start in enumerate(np.arange(0,DURATION,6.4)):
        chord=progression[chord_index%4]
        for note_index,freq in enumerate(chord):
            onset=start+note_index*.48;a=int(onset*rate);span=min(3.8,DURATION-onset)
            if span<=0:continue
            q=np.arange(int(span*rate))/rate
            envelope=(1-np.exp(-q/0.035))*np.exp(-q/1.35)*np.minimum((span-q)/.5,1)
            piano=(np.sin(2*np.pi*freq*q)+.22*np.sin(2*np.pi*freq*2*q)+.045*np.sin(2*np.pi*freq*3*q))
            melody[a:a+len(q)]+=piano*envelope*.043
        a=int(start*rate);span=min(8,DURATION-start);q=np.arange(int(span*rate))/rate
        envelope=np.minimum(q/2.3,1)*np.minimum((span-q)/2.8,1)
        for freq in chord[:3]:pad[a:a+len(q)]+=np.sin(2*np.pi*freq*q)*envelope*.007
    audio=melody+pad
    for delay,level in ((.19,.12),(.37,.08),(.61,.04)):
        shift=int(delay*rate);audio[shift:]+=melody[:-shift]*level
    fade=np.minimum(t/2.2,1)*np.minimum((DURATION-t)/3,1)
    audio*=fade
    left=audio*.96;right=audio*.96
    shift=int(.009*rate);right[shift:]=.96*audio[shift:]+.04*audio[:-shift]
    data=np.stack((left,right),axis=1)
    with wave.open(str(path),'wb') as out:
        out.setnchannels(2);out.setsampwidth(2);out.setframerate(rate);out.writeframes((data*32767).astype('<i2').tobytes())
    print('Gentle original score; peak',round(float(abs(data).max()),3),'RMS',round(float(np.sqrt(np.mean(data*data))),3),flush=True)


def metadata():
    path=OUT/'chapters.txt';lines=[';FFMETADATA1','title=rstop — Feature Film','artist=rstop']
    for start,end,name in CHAPTERS:
        lines+=['[CHAPTER]','TIMEBASE=1/1000',f'START={int(start*1000)}',f'END={int(end*1000)}',f'title={name}']
    path.write_text('\n'.join(lines)+'\n');return path


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--preview',action='store_true');parser.add_argument('--workers',type=int,default=4)
    args=parser.parse_args();OUT.mkdir(parents=True,exist_ok=True);prepare_rasters()
    times=[2,6.6,8.9,10.7,15.8,19.7,22,24,28.2,31.8,37.8,44.8,52,59,66,74,83.5,87,92]
    sheet=Image.new('RGB',(1920,math.ceil(len(times)/3)*390),BG);draw=ImageDraw.Draw(sheet)
    for i,t in enumerate(times):
        image=frame(round(t*FPS));image.resize((620,349),RESAMPLE.LANCZOS).save(OUT/f'preview-{t:04.1f}.jpg',quality=94)
        x=(i%3)*640;y=(i//3)*390;sheet.paste(image.resize((620,349),RESAMPLE.LANCZOS),(x,y))
        draw.text((x+10,y+355),f'{t:04.1f}s',font=ImageFont.truetype(MONO,17),fill=GRAY)
    sheet.save(OUT/'storyboard.jpg',quality=94)
    frame(round(15.8*FPS)).save(OUT/'detail-check.png')
    if args.preview:print(OUT/'storyboard.jpg');return
    score(OUT/'soft-piano.wav');meta=metadata();ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
    silent=OUT/'picture-4k.mp4';master=OUT/'rstop-feature-film-v2-4k.mp4';hd=OUT/'rstop-feature-film-v2-1080p.mp4'
    encoder=subprocess.Popen([ffmpeg,'-hide_banner','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24',
                              '-s',f'{WIDTH}x{HEIGHT}','-r',str(FPS),'-i','-','-an','-c:v','libx264','-preset','fast',
                              '-crf','17','-pix_fmt','yuv420p','-threads','8',str(silent)],stdin=subprocess.PIPE)
    try:
        with ProcessPoolExecutor(max_workers=args.workers) as pool:
            pending=deque();total=FPS*DURATION;submitted=0
            for _ in range(args.workers*2):pending.append(pool.submit(render_frame,submitted));submitted+=1
            for i in range(total):
                encoder.stdin.write(pending.popleft().result())
                if submitted<total:pending.append(pool.submit(render_frame,submitted));submitted+=1
                if i%150==0:print(f'Rendered {i}/{total} frames ({i/FPS:.0f}s)',flush=True)
        encoder.stdin.close()
        if encoder.wait()!=0:raise RuntimeError('4K encoding failed')
    except BaseException:encoder.kill();raise
    subprocess.run([ffmpeg,'-hide_banner','-loglevel','error','-y','-i',str(silent),'-i',str(OUT/'soft-piano.wav'),'-i',str(meta),
                    '-map_metadata','2','-map_chapters','2','-c:v','copy','-c:a','aac','-b:a','256k','-movflags','+faststart',str(master)],check=True)
    subprocess.run([ffmpeg,'-hide_banner','-loglevel','error','-y','-i',str(master),'-vf','scale=1920:1080:flags=lanczos',
                    '-c:v','libx264','-preset','fast','-crf','17','-threads','8','-c:a','copy','-map_chapters','0','-movflags','+faststart',str(hd)],check=True)
    silent.unlink();print(master,flush=True);print(hd,flush=True)


if __name__=='__main__':main()
