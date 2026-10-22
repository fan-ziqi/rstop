#!/usr/bin/env python3
"""Render the 14-second rstop film: fixed compositions, crisp cuts, native UI."""
import argparse
from collections import deque
from concurrent.futures import ProcessPoolExecutor
from functools import lru_cache
from pathlib import Path
import subprocess
import wave

import cairosvg
import imageio_ffmpeg
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'artifacts/promo/short'
NATIVE = ROOT / 'artifacts/promo/v2/native'
FPS, DPI, DURATION = 30, 2, 14
WIDTH, HEIGHT = 3840, 2160
RESAMPLE = getattr(Image, 'Resampling', Image)
WHITE = (246, 245, 241)
INK = (24, 28, 28)
GRAY = (107, 113, 110)
GREEN = (19, 115, 88)
PANEL = (10, 15, 25)
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
BOLD = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
MONO = '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
CHAPTERS = [(0,.8,'rstop'),(.8,3.6,'Processes'),(3.6,5.6,'GPU'),
            (5.6,9.3,'Eight views'),(9.3,11.7,'Themes and session tools'),(11.7,14,'rstop')]


def smooth(v):
    v = max(0, min(1, v))
    return v * v * (3 - 2 * v)


@lru_cache(maxsize=80)
def font(size, bold=False, mono=False):
    return ImageFont.truetype(MONO if mono else BOLD if bold else FONT, round(size * DPI))


@lru_cache(maxsize=160)
def glyphs(value, size, color=INK, bold=False, mono=False):
    face = font(size, bold, mono)
    box = face.getbbox(value)
    im = Image.new('RGBA', (box[2] - box[0] + 8, box[3] - box[1] + 8))
    ImageDraw.Draw(im).text((4-box[0],4-box[1]),value,font=face,fill=color)
    return im


def text(im,value,x,y,size,color=INK,bold=False,mono=False,center=False,opacity=1):
    layer=glyphs(value,size,color,bold,mono)
    if opacity<1:
        layer=layer.copy()
        layer.putalpha(layer.getchannel('A').point(lambda a:round(a*max(0,opacity))))
    im.alpha_composite(layer,(round(x*DPI-(layer.width/2 if center else 0)),round(y*DPI)))


def rect(im,box,color,radius=22):
    ImageDraw.Draw(im).rounded_rectangle(tuple(round(q*DPI) for q in box),radius=round(radius*DPI),fill=color)


def prepare_rasters():
    for path in sorted(NATIVE.glob('*.svg')):
        png=path.with_suffix('.png')
        with Image.open(png) if png.exists() else Image.new('RGB',(1,1)) as existing:
            valid=existing.size==(3936,2472)
        if not valid or png.stat().st_mtime<path.stat().st_mtime:
            cairosvg.svg2png(url=str(path),write_to=str(png),scale=3)


@lru_cache(maxsize=2)
def native(name):
    return Image.open(NATIVE/(name+'.png')).convert('RGBA')


@lru_cache(maxsize=24)
def excerpt(name,box,width,height):
    # Every crop and its display size remain constant throughout a shot.
    source=native(name).crop(tuple(round(q*3) for q in box))
    target=(round(width*DPI),round(height*DPI))
    factor=min(target[0]/source.width,target[1]/source.height)
    fitted=source.resize((round(source.width*factor),round(source.height*factor)),RESAMPLE.LANCZOS)
    canvas=Image.new('RGBA',target,PANEL+(255,))
    canvas.alpha_composite(fitted,(0,0))
    return canvas


def paste_ui(im,name,box,x,y,width,height):
    im.alpha_composite(excerpt(name,tuple(box),width,height),(round(x*DPI),round(y*DPI)))


def stage():
    return Image.new('RGBA',(WIDTH,HEIGHT),WHITE+(255,))


def heading(im,title,kicker):
    text(im,kicker,66,57,20,GREEN,bold=True)
    text(im,title,62,105,65,bold=True)
    text(im,'rstop',1742,61,24,GRAY,mono=True)


def process(t):
    im=stage()
    heading(im,'Find the process.','PROCESSES')
    if t<1.05:
        name,key,action='processes-memory','M','Sort by memory'
    elif t<1.95:
        name,key,action='processes-filter','/','cpu:>10'
    else:
        name,key,action='processes-tree','t','Process tree'
    rect(im,(64,239,1856,935),PANEL)
    paste_ui(im,name,(20,158,1036,385),80,329,1760,393)
    rect(im,(101,824,163,884),(38,54,61),12)
    text(im,key,132,838,29,(173,230,207),mono=True,center=True)
    text(im,action,187,838,29,(221,231,226),mono=key=='/')
    return im


def gpu(t):
    im=stage()
    heading(im,'Know your GPU.','GPU')
    name='gpu-'+('90' if t<.65 else '110' if t<1.3 else '120')
    rect(im,(64,239,1856,1005),PANEL)
    paste_ui(im,name,(16,110,1296,241),80,256,1760,180)
    paste_ui(im,name,(16,248,832,522),144,440,1632,548)
    return im


CARDS=[
    ('Overview','overview-120',(20,160,640,405)),
    ('Storage','storage-120',(20,110,710,370)),
    ('Network','network-120',(20,155,710,410)),
    ('System','system-120',(20,215,500,433)),
    ('Containers','containers-120',(20,163,660,352)),
    ('Services','services-120',(20,163,660,352)),
]


def all_views(t):
    im=stage()
    heading(im,'Everything, together.','8 VIEWS')
    for i,(label,name,box) in enumerate(CARDS):
        x=64+(i%3)*604
        y=250+(i//3)*359
        # Cards settle by 18 pixels; the UI never scales or moves inside its card.
        delay=(i%3)*.045+(i//3)*.06
        offset=round(18*(1-smooth((t-delay)/.28)))
        y+=offset
        rect(im,(x,y,x+584,y+330),PANEL,19)
        text(im,label,x+25,y+22,31,(231,239,235),bold=True)
        sample=(80,90,100,110,120)[min(4,int(t/.72))]
        live=name.rsplit('-',1)[0]+'-'+str(sample)
        paste_ui(im,live,box,x+22,y+91,540,213)
    return im


THEMES=[('Midnight',(14,20,33)),('Nord',(46,52,64)),('Ember',(41,24,24)),
        ('Black',(0,0,0)),('White',(255,255,255)),('Graphite',(41,43,47)),
        ('Ocean',(10,38,57)),('Forest',(18,40,30)),('Sand',(223,211,184))]


def tools(t):
    im=stage()
    heading(im,'Make it yours.','9 THEMES / RECORD / REPLAY / EXPORT')
    rect(im,(64,255,705,935),(234,233,227))
    text(im,'9 themes.',105,303,49,bold=True)
    for i,(name,color) in enumerate(THEMES):
        x=105+(i%3)*190
        y=423+(i//3)*158
        rect(im,(x,y,x+143,y+72),color,14)
        if i==min(8,int(t/.26)):
            ImageDraw.Draw(im).rounded_rectangle(((x-5)*DPI,(y-5)*DPI,(x+148)*DPI,(y+77)*DPI),radius=19*DPI,outline=GREEN,width=2*DPI)
        text(im,name,x,y+88,19,GRAY)
    rect(im,(735,255,1856,935),PANEL)
    commands=[('record','rstop record session.jsonl'),
              ('replay','rstop replay session.jsonl'),
              ('export','rstop export csv metrics.csv')]
    for i,(label,command) in enumerate(commands):
        y=324+i*181
        text(im,label.upper(),781,y,20,(126,166,149),bold=True)
        text(im,'$ '+command,781,y+48,30,(225,238,230),mono=True)
    return im


def brand(t,end=False):
    im=stage()
    if not end:
        text(im,'rstop',960,409,147,INK,mono=True,center=True,opacity=smooth(t/.2))
    else:
        text(im,'rstop',960,245,97,INK,mono=True,center=True)
        text(im,'One terminal.',960,418,72,INK,bold=True,center=True)
        text(im,'Your whole system.',960,513,72,INK,bold=True,center=True)
        text(im,'Linux   /   macOS   /   Windows',960,697,25,GRAY,center=True)
        text(im,'fan-ziqi.github.io/rstop',960,809,28,GREEN,mono=True,center=True)
    return im


def frame(index):
    t=index/FPS
    scene=next(i for i,(a,b,_) in enumerate(CHAPTERS) if a<=t<b)
    local=t-CHAPTERS[scene][0]
    im=[lambda:brand(local),lambda:process(local),lambda:gpu(local),
        lambda:all_views(local),lambda:tools(local),lambda:brand(local,True)][scene]()
    # A short horizontal curtain connects scenes without dissolving UI glyphs.
    if scene and local<.14:
        width=round(WIDTH*(1-smooth(local/.14)))
        ImageDraw.Draw(im).rectangle((WIDTH-width,0,WIDTH,HEIGHT),fill=WHITE+(255,))
    return im.convert('RGB')


def render_frame(index):
    return frame(index).tobytes()


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
    path=OUT/'chapters.txt'
    lines=[';FFMETADATA1','title=rstop — 14 seconds','artist=rstop']
    for start,end,name in CHAPTERS:
        lines+=['[CHAPTER]','TIMEBASE=1/1000',f'START={int(start*1000)}',f'END={int(end*1000)}',f'title={name}']
    path.write_text('\n'.join(lines)+'\n')
    return path


def previews():
    times=[.5,1.4,2.3,3.2,4.5,6.4,8.6,10.3,13]
    sheet=Image.new('RGB',(1920,1170),WHITE)
    draw=ImageDraw.Draw(sheet)
    for i,t in enumerate(times):
        im=frame(round(t*FPS))
        im.resize((1920,1080),RESAMPLE.LANCZOS).save(OUT/f'frame-{t:04.1f}.png')
        x=(i%3)*640;y=(i//3)*390
        sheet.paste(im.resize((620,349),RESAMPLE.LANCZOS),(x,y))
        draw.text((x+12,y+358),f'{t:.1f}s',font=ImageFont.truetype(MONO,17),fill=GRAY)
    sheet.save(OUT/'storyboard.jpg',quality=95)
    print(OUT/'storyboard.jpg',flush=True)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--preview',action='store_true')
    parser.add_argument('--workers',type=int,default=4)
    args=parser.parse_args()
    OUT.mkdir(parents=True,exist_ok=True)
    prepare_rasters()
    previews()
    if args.preview:return
    score(OUT/'soft-piano.wav')
    meta=metadata()
    ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
    silent=OUT/'picture-4k.mp4'
    master=OUT/'rstop-14s-4k.mp4'
    hd=OUT/'rstop-14s-1080p.mp4'
    encoder=subprocess.Popen([ffmpeg,'-hide_banner','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24',
                              '-s',f'{WIDTH}x{HEIGHT}','-r',str(FPS),'-i','-','-an','-c:v','libx264','-preset','fast',
                              '-crf','17','-pix_fmt','yuv420p','-threads','8',str(silent)],stdin=subprocess.PIPE)
    try:
        with ProcessPoolExecutor(max_workers=args.workers) as pool:
            pending=deque();total=FPS*DURATION;submitted=0
            for _ in range(args.workers*2):
                pending.append(pool.submit(render_frame,submitted));submitted+=1
            for i in range(total):
                encoder.stdin.write(pending.popleft().result())
                if submitted<total:
                    pending.append(pool.submit(render_frame,submitted));submitted+=1
                if i%90==0:print(f'Rendered {i}/{total} frames',flush=True)
        encoder.stdin.close()
        if encoder.wait()!=0:raise RuntimeError('4K encoding failed')
    except BaseException:
        encoder.kill();raise
    subprocess.run([ffmpeg,'-hide_banner','-loglevel','error','-y','-i',str(silent),'-i',str(OUT/'soft-piano.wav'),'-i',str(meta),
                    '-map_metadata','2','-map_chapters','2','-c:v','copy','-c:a','aac','-b:a','256k','-movflags','+faststart',str(master)],check=True)
    subprocess.run([ffmpeg,'-hide_banner','-loglevel','error','-y','-i',str(master),'-vf','scale=1920:1080:flags=lanczos',
                    '-c:v','libx264','-preset','fast','-crf','17','-threads','8','-c:a','copy','-map_chapters','0','-movflags','+faststart',str(hd)],check=True)
    silent.unlink()
    print(master,flush=True)
    print(hd,flush=True)


if __name__=='__main__':main()
