#!/usr/bin/env python3
"""Create rstop's original 42-second 1080p product film and synthesized score.

pip install pillow numpy cairosvg imageio-ffmpeg
python3 promo/render.py
"""
from concurrent.futures import ProcessPoolExecutor
from functools import lru_cache
from io import BytesIO
from pathlib import Path
import argparse
import math
import subprocess
import wave

import cairosvg
import imageio_ffmpeg
import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT=Path(__file__).resolve().parents[1]
ASSETS=ROOT/'website/dist/assets'
OUT=ROOT/'artifacts/promo'
W,H,FPS,DURATION=1920,1080,30,42
RESAMPLE=getattr(Image,'Resampling',Image)
MINT=(167,243,203)
WHITE=(241,245,243)
GRAY=(128,147,139)
FONT='/usr/share/fonts/opentype/noto/NotoSansCJK-Medium.ttc'
BOLD='/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc'
LATIN='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
MONO='/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
CUTS=(0,5,11,17,23,29,35,42)


def clamp(v):return max(0.,min(1.,v))
def ease(v):v=clamp(v);return 1-(1-v)**3
def smooth(v):v=clamp(v);return v*v*(3-2*v)


@lru_cache(maxsize=150)
def font(size,bold=False,mono=False):
    return ImageFont.truetype(MONO if mono else BOLD if bold else FONT,size,index=2 if not mono else 0)


@lru_cache(maxsize=200)
def label(text,size,color=WHITE,bold=False,mono=False):
    f=font(size,bold,mono);box=f.getbbox(text)
    layer=Image.new('RGBA',(max(1,box[2]-box[0]+8),box[3]-box[1]+8))
    ImageDraw.Draw(layer).text((4-box[0],4-box[1]),text,font=f,fill=color)
    return layer


def text(im,s,x,y,size,color=WHITE,bold=False,opacity=1,center=False,mono=False):
    layer=label(s,size,color,bold,mono)
    if opacity<1:
        layer=layer.copy();layer.putalpha(layer.getchannel('A').point(lambda a:round(a*clamp(opacity))))
    im.alpha_composite(layer,(round(x-layer.width/2 if center else x),round(y)))


@lru_cache(maxsize=8)
def background(scene):
    yy,xx=np.mgrid[0:H,0:W];a=np.zeros((H,W,4),dtype=np.uint8);a[:,:,:3]=(5,8,7);a[:,:,3]=255
    cx,cy,spread=[(980,580,650),(1050,660,670),(1370,560,600),(1100,640,700),(960,610,750),(1000,700,650),(960,590,630)][scene]
    glow=np.exp(-((xx-cx)**2/(spread**2)+(yy-cy)**2/(spread*.6)**2))
    for i,v in enumerate((10,31,19)):a[:,:,i]=np.clip(a[:,:,i]+glow*v,0,255)
    img=Image.fromarray(a,'RGBA')
    if scene in (2,4):
        grid=Image.new('RGBA',(W,H));d=ImageDraw.Draw(grid)
        for x in range(-400,W+400,80):d.line([(x,0),(x+400,H)],fill=(56,90,69,16))
        for y in range(40,H,80):d.line([(0,y),(W,y)],fill=(56,90,69,12))
        img.alpha_composite(grid)
    return img


@lru_cache(maxsize=12)
def terminal(view,theme='midnight'):
    raw=cairosvg.svg2png(url=str(ASSETS/f'{view}-{theme}.svg'),scale=1.45)
    screen=Image.open(BytesIO(raw)).convert('RGBA')
    pad=3;bar=46
    panel=Image.new('RGBA',(screen.width+pad*2,screen.height+bar+pad*2))
    d=ImageDraw.Draw(panel)
    d.rounded_rectangle((0,0,panel.width-1,panel.height-1),radius=16,fill='#101820',outline='#6c8175',width=2)
    d.rounded_rectangle((2,2,panel.width-3,60),radius=14,fill='#1c2328')
    d.rectangle((2,22,panel.width-3,bar),fill='#1c2328')
    for i,c in enumerate(('#f0655a','#e5bb52','#60bf7a')):d.ellipse((23+i*22,17,33+i*22,27),fill=c)
    d.text((panel.width//2-100,12),f'rstop — {view}',font=font(15,mono=True),fill='#9caba3')
    d.text((panel.width-85,13),'DEMO',font=font(13,mono=True),fill='#789287')
    panel.alpha_composite(screen,(pad,bar));return panel


def screen(im,view,x,y,width,tilt=0,opacity=1,theme='midnight'):
    source=terminal(view,theme)
    height=round(width*source.height/source.width)
    layer=source.resize((round(width),height),RESAMPLE.LANCZOS)
    if tilt:
        # Subtle animated camera movement, with the image remaining fully readable.
        layer=layer.rotate(tilt,RESAMPLE.BICUBIC,expand=True)
    if opacity<1:layer.putalpha(layer.getchannel('A').point(lambda a:round(a*clamp(opacity))))
    im.alpha_composite(layer,(round(x),round(y)))


def rule(im,x,y,width,alpha=1):
    ImageDraw.Draw(im).line((x,y,x+width,y),fill=(*MINT,round(80*alpha)),width=1)


def scene_image(scene,local):
    im=background(scene).copy();d=ImageDraw.Draw(im);intro=ease(local/1.2)
    if scene==0:
        # A trace becomes a command; the command becomes the product.
        if local<1.6:
            points=[]
            for x in range(350,1570,4):
                p=(x-350)/1220;v=math.sin(p*40)*math.exp(-((p-.52)/.11)**2)
                points.append((x,540+v*80*smooth(local/.5)))
            upto=max(2,round(len(points)*ease(local/.8)))
            d.line(points[:upto],fill=(*MINT,220),width=3)
            text(im,'A LITTLE CLARITY.',960,665,18,GRAY,center=True,opacity=ease(local-.3),mono=True)
        else:
            q=ease((local-1.6)/1.2)
            text(im,'INTRODUCING',960,250+20*(1-q),18,GRAY,opacity=q,center=True,mono=True)
            text(im,'rstop',960,365+35*(1-q),168,WHITE,opacity=q,center=True,bold=True,mono=True)
            text(im,'_',1280,410,120,MINT,opacity=q,mono=True)
            rule(im,825,620,270,q)
            text(im,'One terminal. Your whole system.',960,666,30,GRAY,opacity=ease(local-2.5),center=True)
    elif scene==1:
        text(im,'Your whole system.',960,90+30*(1-intro),69,WHITE,bold=True,opacity=intro,center=True)
        text(im,'Now in one terminal.',960,192,28,GRAY,opacity=ease((local-.5)/1.1),center=True)
        scale=1440+65*smooth(local/6)
        screen(im,'overview',(W-scale)/2,310+80*(1-intro),scale,tilt=-.8+.8*smooth(local/6),opacity=intro)
        # The lower edge disappears into the stage.
        fade=Image.new('RGBA',(W,160));arr=np.zeros((160,W,4),dtype=np.uint8);arr[:,:,:3]=(5,8,7);arr[:,:,3]=np.linspace(0,255,160)[:,None];fade=Image.fromarray(arr)
        im.alpha_composite(fade,(0,920))
    elif scene==2:
        text(im,'GPU. FRONT AND CENTER.',145,180,18,MINT,opacity=intro,mono=True)
        text(im,'GPU power.',140,269+25*(1-intro),82,WHITE,bold=True,opacity=intro)
        text(im,'In full view.',140,399+25*(1-intro),76,MINT,bold=True,opacity=intro)
        text(im,'Load · VRAM · Power · Thermals',148,585,21,GRAY,opacity=ease(local-.8))
        rule(im,150,657,290,intro)
        text(im,'Down to every process.',148,702,23,GRAY,opacity=ease(local-1.4))
        screen(im,'gpu',790-50*smooth(local/6),160,1420+45*smooth(local/6),tilt=2-1.5*smooth(local/6),opacity=intro)
    elif scene==3:
        text(im,'Follow the signal.',960,104+30*(1-intro),70,WHITE,bold=True,center=True,opacity=intro)
        text(im,'Process trees / Resource attribution / Powerful filters',960,211,23,GRAY,center=True,opacity=ease(local-.5))
        screen(im,'processes',190-25*smooth(local/6),320+55*(1-intro),1540+50*smooth(local/6),tilt=-1+.8*smooth(local/6),opacity=intro)
        layer=Image.new('RGBA',(960,94));dl=ImageDraw.Draw(layer);dl.rounded_rectangle((0,0,959,93),radius=16,fill='#15261d',outline='#7ba68b',width=1)
        shown='cpu:>10 !name:rust'[:max(0,min(18,int((local-1.2)*18)))]
        text(layer,'/',32,30,25,MINT,mono=True);text(layer,shown,85,29,25,WHITE,mono=True)
        text(layer,'FILTER',820,33,19,GRAY)
        if local>1.2:im.alpha_composite(layer,(480,835+round(35*(1-ease(local-1.2)))))
    elif scene==4:
        text(im,'Eight views.',960,105,78,WHITE,bold=True,center=True,opacity=intro)
        text(im,'One complete picture.',960,208,52,GRAY,center=True,opacity=ease(local-.4))
        names=[('01','Overview','overview'),('02','Processes','processes'),('03','GPU','gpu'),('04','Storage','storage'),('05','Network','network'),('06','System','system'),('07','Containers','containers'),('08','Services','services')]
        for i,(n,name,v) in enumerate(names):
            col,row=i%4,i//4;x=140+col*420;y=385+row*265;enter=ease((local-.3-i*.13)/.8)
            tile=terminal(v).resize((365,229),RESAMPLE.LANCZOS)
            tile.putalpha(tile.getchannel('A').point(lambda a:round(a*.68*enter)))
            im.alpha_composite(tile,(x,y+round(30*(1-enter))))
            d.rounded_rectangle((x,y+170,x+365,y+244),radius=8,fill=(8,15,11,245))
            text(im,n,x+14,y+192,13,MINT,opacity=enter,mono=True)
            text(im,name,x+65,y+184,26,WHITE,opacity=enter)
        text(im,'CPU  /  GPU  /  STORAGE  /  NETWORK  /  WORKLOADS',960,997,14,GRAY,center=True,mono=True,opacity=ease(local-2.5))
    elif scene==5:
        text(im,'Record the moment.',145,176+25*(1-intro),67,WHITE,bold=True,opacity=intro)
        text(im,'Replay every detail.',145,288+25*(1-intro),67,MINT,bold=True,opacity=intro)
        text(im,'RECORD. REPLAY. UNDERSTAND.',150,442,19,GRAY,opacity=ease(local-.6),mono=True)
        text(im,'$ rstop record session.jsonl',150,545,22,WHITE,opacity=ease(local-1),mono=True)
        text(im,'$ rstop replay session.jsonl',150,594,22,GRAY,opacity=ease(local-1.5),mono=True)
        screen(im,'network',975-35*smooth(local/6),120,1140+70*smooth(local/6),tilt=2.2-1.3*smooth(local/6),opacity=intro)
        x0,y0,x1=150,855,1750
        d.line((x0,y0,x1,y0),fill='#304d3b',width=2)
        for i in range(105):
            x=x0+i*(x1-x0)/104;h=12+abs(math.sin(i*.42)*math.cos(i*.15))*65
            d.line((x,y0-h,x,y0-10),fill=(*MINT,80),width=3)
        progress=.12+.72*smooth(local/6);xp=x0+(x1-x0)*progress
        d.line((x0,y0,xp,y0),fill=MINT,width=3);d.ellipse((xp-6,y0-6,xp+6,y0+6),fill=MINT)
        text(im,'00:'+str(12+int(local*7)).zfill(2),xp,885,16,MINT,mono=True,center=True)
        text(im,'JSON  /  CSV  /  SVG',150,932,15,GRAY,mono=True)
    else:
        text(im,'rstop',960,230+30*(1-intro),158,WHITE,center=True,mono=True,bold=True,opacity=intro)
        text(im,'_',1260,270,120,MINT,mono=True,opacity=intro)
        text(im,'Your system. In full view.',960,482,54,WHITE,center=True,bold=True,opacity=ease(local-.7))
        text(im,'One terminal. Your whole system.',960,582,26,GRAY,center=True,opacity=ease(local-1.2))
        rule(im,840,687,240,ease(local-1.7))
        text(im,'Linux   /   macOS   /   Windows',960,736,22,MINT,center=True,opacity=ease(local-2),mono=True)
        text(im,'github.com/fan-ziqi/rstop',960,850,24,WHITE,center=True,opacity=ease(local-2.4),mono=True)
        text(im,'BUILT IN RUST. OPEN SOURCE.',960,927,13,GRAY,center=True,opacity=ease(local-2.8),mono=True)
    return im


def frame(t):
    scene=next((i for i in range(7) if CUTS[i]<=t<CUTS[i+1]),6)
    local=t-CUTS[scene];im=scene_image(scene,local)
    # Short, precise dissolves, long enough to feel intentional but keep the pace.
    if scene<6 and CUTS[scene+1]-t<.34:
        q=smooth((t-CUTS[scene+1]+.34)/.34)
        im=Image.blend(im,scene_image(scene+1,0),q)
    opacity=min(smooth(t/.65),smooth((DURATION-t)/1.1))
    if opacity<1:im=Image.blend(Image.new('RGBA',(W,H),(5,8,7,255)),im,opacity)
    if 5<t<35:
        text(im,'RSTOP / PRODUCT FILM',65,38,11,(99,128,113),mono=True,opacity=.7)
        text(im,'Native UI · Illustrative data',W-400,H-38,12,(103,125,114),opacity=.8)
    return im.convert('RGB')


def render_frame(index):return frame(index/FPS).tobytes()


def soundtrack(path):
    """Original synthesized ambient/electronic music. No third-party recordings."""
    rate=48000;n=rate*DURATION;t=np.arange(n,dtype=np.float64)/rate;audio=np.zeros(n,dtype=np.float64)
    # A warm D-minor progression, with slow stereo movement added at the end.
    chords=[(146.832,174.614,220),(130.813,174.614,220),(130.813,164.814,195.998),(146.832,174.614,220)]
    for start in range(0,DURATION,6):
        span=min(8,DURATION-start);q=np.arange(int(span*rate))/rate
        env=np.minimum(q/1.8,1)*np.minimum((span-q)/2.4,1)*.022
        chord=chords[(start//6)%4]
        pad=sum(np.sin(2*np.pi*f*q+.008*np.sin(q*2))* .7+np.sin(2*np.pi*f*1.002*q)*.3 for f in chord)
        audio[start*rate:start*rate+len(q)]+=pad*env
    beat=60/104
    for k in range(int((DURATION-5)/beat)):
        start=5+k*beat
        if start>36:continue
        a=int(start*rate);length=min(int(.5*rate),n-a);q=np.arange(length)/rate
        kick=np.sin(2*np.pi*(44*q+7*(1-np.exp(-q*35))))*np.exp(-q*13)*.14
        audio[a:a+length]+=kick
        # Delicate upper-register plucks alternate with the pulse.
        note=[293.665,440,523.251,349.228,293.665,523.251,440,391.995][k%8]
        pluck=(np.sin(2*np.pi*note*q)+.2*np.sin(2*np.pi*note*2*q))*np.exp(-q*7)*.035
        audio[a:a+length]+=pluck
        if k%2:
            rng=np.random.default_rng(k);noise=rng.normal(size=length)
            audio[a:a+length]+=(noise-np.roll(noise,1))*np.exp(-q*65)*.009
    for moment in CUTS[1:-1]:
        start=max(0,moment-.55);a=int(start*rate);length=int(.9*rate);q=np.arange(length)/rate
        rng=np.random.default_rng(int(moment*13));noise=rng.normal(size=length)
        env=np.sin(np.pi*np.clip(q/.9,0,1))**2*.016
        audio[a:a+length]+=noise*env
    fade=np.minimum(t/2.0,1)*np.minimum((DURATION-t)/2.2,1)
    audio=np.tanh(audio*1.7)*fade
    left=audio*(.94+.06*np.sin(t*.6));right=audio*(.94-.06*np.sin(t*.6))
    stereo=np.stack((left,right),axis=1);stereo*=.83/max(.83,np.max(np.abs(stereo)))
    with wave.open(str(path),'wb') as output:
        output.setnchannels(2);output.setsampwidth(2);output.setframerate(rate);output.writeframes((stereo*32767).astype('<i2').tobytes())


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--preview',action='store_true');parser.add_argument('--workers',type=int,default=4)
    args=parser.parse_args();OUT.mkdir(parents=True,exist_ok=True)
    # Reviewable contact sheet and a website social card, generated from the same visual system.
    times=[2.8,7.8,13.8,19.8,25.8,31.8,38.4]
    sheet=Image.new('RGB',(1280,4*390),(10,13,11));draw=ImageDraw.Draw(sheet)
    for i,t in enumerate(times):
        preview=frame(t).resize((620,349),RESAMPLE.LANCZOS);x=(i%2)*640;y=(i//2)*390
        sheet.paste(preview,(x,y));draw.text((x+12,y+353),f'{t:04.1f}s',font=font(16,mono=True),fill=GRAY)
    sheet.save(OUT/'storyboard.jpg',quality=93)
    frame(38.4).save(OUT/'poster.jpg',quality=95)
    social=frame(7.8).resize((1200,675),RESAMPLE.LANCZOS)
    social.crop((0,0,1200,630)).save(ASSETS/'social.png',optimize=True)
    if args.preview:print(OUT/'storyboard.jpg');return
    soundtrack(OUT/'score.wav')
    ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
    silent=OUT/'picture.mp4';dest=OUT/'rstop-product-film-1080p.mp4'
    command=[ffmpeg,'-hide_banner','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24','-s',f'{W}x{H}','-r',str(FPS),
             '-i','-','-an','-c:v','libx264','-preset','medium','-crf','17','-pix_fmt','yuv420p','-threads','4',str(silent)]
    encoder=subprocess.Popen(command,stdin=subprocess.PIPE)
    try:
        with ProcessPoolExecutor(max_workers=args.workers) as pool:
            for i,pixels in enumerate(pool.map(render_frame,range(FPS*DURATION),chunksize=8)):
                encoder.stdin.write(pixels)
                if i%150==0:print(f'Rendered {i}/{FPS*DURATION} frames',flush=True)
        encoder.stdin.close()
        if encoder.wait()!=0:raise RuntimeError('Video encoding failed')
    except BaseException:
        encoder.kill();raise
    subprocess.run([ffmpeg,'-hide_banner','-loglevel','error','-y','-i',str(silent),'-i',str(OUT/'score.wav'),'-c:v','copy',
                    '-c:a','aac','-b:a','256k','-movflags','+faststart','-shortest',str(dest)],check=True)
    silent.unlink();print(dest,flush=True)


if __name__=='__main__':main()
