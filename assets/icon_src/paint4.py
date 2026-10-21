import numpy as np, math, cv2
from PIL import Image, ImageDraw, ImageFilter
rng=np.random.default_rng(11)
src=cv2.imread('bust_source.png')
cut=Image.open('bust_cutout.png').convert('RGBA'); bb=cut.getbbox()
x0,y0,x1,y1=bb; pad=8; x0-=pad;y0-=pad;x1+=pad;y1+=pad
crop=src[y0:y1,x0:x1]
sr=cv2.dnn_superres.DnnSuperResImpl_create(); sr.readModel('EDSR_x4.pb'); sr.setModel('edsr',4)
up=sr.upsample(crop)                                # 4x clean detail
H,W=up.shape[:2]
# alpha: upscale rembg matte smoothly, then firm it into a clean anti-aliased edge
al=np.asarray(cut.getchannel('A'))[y0:y1,x0:x1].astype(np.float32)/255
al=cv2.resize(al,(W,H),interpolation=cv2.INTER_CUBIC)
al=cv2.GaussianBlur(al,(0,0),2.2); al=np.clip((al-0.5)*3.2+0.5,0,1)
al=cv2.GaussianBlur(al,(0,0),0.9)
rgb=cv2.cvtColor(up,cv2.COLOR_BGR2RGB).astype(np.float32)/255
L=0.2126*rgb[...,0]+0.7152*rgb[...,1]+0.0722*rgb[...,2]
m_=al>0.5; lo,hi=np.percentile(L[m_],2),np.percentile(L[m_],99.5); s=np.clip((L-lo)/(hi-lo),0,1)
yy,xx=np.mgrid[0:H,0:W]; t=np.clip(0.5+0.5*((xx/W-0.5)*0.6+(0.5-yy/H)*1.0)/0.8,0,1)
stops=np.array([[0,0x3A,0x7B,0xFF],[0.32,0x9A,0x5C,0xFF],[0.64,0xFF,0x4F,0x7A],[1,0xFF,0xAE,0x2B]],float)
base=np.stack([np.interp(t,stops[:,0],stops[:,i]) for i in (1,2,3)],-1)/255
paint=base*(0.34+0.86*s**1.05)[...,None]+0.24*(s**5)[...,None]
white=np.clip(rgb*1.04,0,1)*np.array([0.97,0.965,1.0])

# the first version's mask, same layout and parameters, built at 4x resolution
B=558.0/W          # base-resolution pixel in hi-res units (first version worked at the cutout's own size)
S=1/B
m=Image.new('L',(W,H),0); d=ImageDraw.Draw(m)
def blob(cx,cy,r,n=14):
    for _ in range(n):
        rr=r*rng.uniform(0.35,0.75); x=cx+rng.normal(0,r*0.45); y=cy+rng.normal(0,r*0.35)
        d.ellipse([(x-rr)*W,(y-rr*0.8)*H,(x+rr)*W,(y+rr*0.8)*H],fill=255)
blob(0.72,0.14,0.22,22); blob(0.88,0.36,0.14,12); blob(0.62,0.42,0.12,10)
d.line([(0.18*W,0.50*H),(0.42*W,0.44*H),(0.70*W,0.55*H)],fill=255,width=int(0.07*W),joint='curve')
blob(0.80,0.90,0.12,10)
m=m.filter(ImageFilter.GaussianBlur(10*S))
a=np.asarray(m).astype(float)/255
noise=rng.random(a.shape)
edge=(a>0.08)&(a<0.92)
spray=np.where(edge,(noise<a**1.4).astype(float),(a>=0.92).astype(float))
spray=np.maximum(spray,(a>0.55).astype(float))
far=(a>0.01)&(a<0.25)
spray=np.maximum(spray,(rng.random(a.shape)<0.018*far/ (S*S) ))
sp=Image.fromarray((spray*255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(0.9*S*0.6))
dd=ImageDraw.Draw(sp)
for _ in range(16):
    for tries in range(200):
        x=int(rng.uniform(0.15,0.95)*W); col_=np.asarray(sp)[:,x]; ys=np.where(col_>200)[0]
        if len(ys)==0: continue
        y0=ys.max(); break
    L=rng.uniform(0.03,0.16)*H; w=rng.uniform(2,4.5)*2*S/2*1.0
    dd.line([(x,y0),(x,y0+L)],fill=255,width=max(2,int(w))); dd.ellipse([x-w*0.8,y0+L-w*0.8,x+w*0.8,y0+L+w*0.8],fill=255)
Mf=np.asarray(sp.filter(ImageFilter.GaussianBlur(0.8))).astype(np.float32)/255
out=white*(1-Mf[...,None])+paint*Mf[...,None]
img=np.dstack([np.clip(out,0,1),al])
Image.fromarray((img*255).astype(np.uint8),'RGBA').save('bust_painted_hi.png')
