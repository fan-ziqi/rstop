#!/usr/bin/env python3
"""Create the README's eight-view animated overview from native rstop screens."""
from pathlib import Path
from PIL import Image, ImageDraw
from render import ROOT, terminal, font, MINT, WHITE, GRAY, RESAMPLE

VIEWS = [
    ('overview', 'Your whole system, at a glance.'),
    ('processes', 'Find the work behind the numbers.'),
    ('gpu', 'GPU load, VRAM, power and thermals.'),
    ('storage', 'Capacity. Throughput. I/O.'),
    ('network', 'Interfaces, connections and owners.'),
    ('system', 'Sensors, pressure and system details.'),
    ('containers', 'Local Docker and Podman workloads.'),
    ('services', 'Services, state and resource usage.')
]


def main():
    dest=ROOT/'docs/assets/rstop-demo.gif';dest.parent.mkdir(parents=True,exist_ok=True)
    cards=[]
    for i,(view,description) in enumerate(VIEWS):
        im=Image.new('RGB',(1000,750),(8,12,10));d=ImageDraw.Draw(im)
        d.text((33,20),'rstop',font=font(25,mono=True),fill=WHITE)
        d.text((136,29),'ONE TERMINAL. YOUR WHOLE SYSTEM.',font=font(11,mono=True),fill=GRAY)
        d.text((34,63),description,font=font(21),fill=WHITE)
        d.text((890,66),f'{i+1:02d} / 08',font=font(13,mono=True),fill=MINT)
        screen=terminal(view).resize((936,610),RESAMPLE.LANCZOS)
        im.paste(screen,(32,109),screen)
        x=34
        for j,(key,_) in enumerate(VIEWS):
            word=key.title() if key!='gpu' else 'GPU'
            d.text((x,724),word,font=font(10),fill=MINT if i==j else (82,102,90))
            x += [107,116,62,99,105,97,123,100][j]
        d.text((855,724),'NATIVE UI / DEMO DATA',font=font(8,mono=True),fill=(82,102,90))
        cards.append(im)
    frames=[];durations=[]
    # One global palette keeps native terminal text stable through every dissolve.
    palette=cards[0].quantize(colors=160)
    for i,card in enumerate(cards):
        frames.append(card.quantize(palette=palette,dither=0));durations.append(1700)
        for step in range(1,5):
            fade=Image.blend(card,cards[(i+1)%len(cards)],step/5)
            frames.append(fade.quantize(palette=palette,dither=0));durations.append(80)
    frames[0].save(dest,save_all=True,append_images=frames[1:],duration=durations,loop=0,optimize=True,disposal=1)
    print(f'{dest} ({dest.stat().st_size:,} bytes)')


if __name__=='__main__':main()
