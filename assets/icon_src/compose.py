import sys, numpy as np, math, subprocess
sys.path.insert(0,'.')
from tile import squircle
from PIL import Image, ImageFilter
bust=Image.open('bust_painted.png')
SIZE=690; DX=-12; DY=3
def tile_png(mode, out):
    T=824 if mode=='mac' else 1024
    tile=squircle(512,512,T)
    svg=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024"><defs>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#24212B"/><stop offset="1" stop-color="#0B0A0E"/></linearGradient>
<radialGradient id="bloom" cx=".5" cy=".46" r=".5"><stop offset="0" stop-color="#FF4F7A" stop-opacity=".26"/><stop offset=".3" stop-color="#C255D9" stop-opacity=".13"/><stop offset="1" stop-color="#9A5CFF" stop-opacity="0"/></radialGradient>
<filter id="ts" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="12" stdDeviation="14" flood-color="#000" flood-opacity=".5"/></filter>
<clipPath id="tc"><path d="{tile}"/></clipPath></defs>
{f'<path d="{tile}" fill="#000" filter="url(#ts)"/>' if mode=='mac' else ''}<path d="{tile}" fill="url(#bg)"/><g clip-path="url(#tc)"><rect width="1024" height="1024" fill="url(#bloom)"/></g></svg>'''
    rim=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024"><defs><linearGradient id="rim" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".22"/><stop offset=".3" stop-color="#fff" stop-opacity=".04"/><stop offset="1" stop-color="#fff" stop-opacity=".08"/></linearGradient><clipPath id="tc"><path d="{tile}"/></clipPath></defs><path d="{tile}" fill="none" stroke="url(#rim)" stroke-width="{3*T/824:.1f}" clip-path="url(#tc)"/></svg>'''
    open('t.svg','w').write(svg); open('r.svg','w').write(rim)
    subprocess.run(['node','render.mjs','t.svg','t.png','1024'],check=True); subprocess.run(['node','render.mjs','r.svg','r.png','1024'],check=True)
    base=Image.open('t.png').convert('RGBA')
    k=T/824
    h=int(SIZE*k); w=int(bust.width*h/bust.height); b=bust.resize((w,h),Image.LANCZOS)
    ox=(1024-w)//2+int(DX*k); oy=(1024-h)//2+int(DY*k)
    s_=Image.new('RGBA',(1024,1024),(0,0,0,0)); blk=Image.new('RGBA',b.size,(0,0,0,255)); blk.putalpha(b.getchannel('A').point(lambda v:int(v*0.6)))
    s_.paste(blk,(ox,oy+int(18*k)),blk); s_=s_.filter(ImageFilter.GaussianBlur(20*k))
    layer=Image.new('RGBA',(1024,1024),(0,0,0,0)); layer.alpha_composite(s_); layer.alpha_composite(b,(ox,oy))
    tileA=Image.open('t.png').getchannel('A')
    la=np.minimum(np.asarray(layer.getchannel('A')),np.asarray(tileA)); layer.putalpha(Image.fromarray(la))
    base.alpha_composite(layer); base.alpha_composite(Image.open('r.png').convert('RGBA'))
    base.save(out)
tile_png('mac','R_mac.png'); tile_png('full','R_full.png')
