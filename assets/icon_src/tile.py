import math, sys

def squircle(cx, cy, size, n=3.0, pts=90):
    e = 0.30*size; h = size/2
    corners = [(cx+h-e, cy-h+e, 0), (cx+h-e, cy+h-e, 90), (cx-h+e, cy+h-e, 180), (cx-h+e, cy-h+e, 270)]
    out=[]
    for (ox, oy, start) in corners:
        for i in range(pts+1):
            t = math.radians(start - 90 + 90*i/pts)
            c, s_ = math.cos(t), math.sin(t)
            out.append(f"{ox+e*math.copysign(abs(c)**(2/n),c):.2f},{oy+e*math.copysign(abs(s_)**(2/n),s_):.2f}")
    return "M"+" L".join(out)+" Z"

RRECT = "M230,0 H794 A230,230 0 0 1 1024,230 V794 A230,230 0 0 1 794,1024 H230 A230,230 0 0 1 0,794 V230 A230,230 0 0 1 230,0 Z"
STOPS = [(0,(0x3A,0x7B,0xFF)),(0.32,(0x9A,0x5C,0xFF)),(0.64,(0xFF,0x4F,0x7A)),(1,(0xFF,0xAE,0x2B))]
def grad(t):
    for (a,ca),(b,cb) in zip(STOPS,STOPS[1:]):
        if t<=b:
            u=(t-a)/(b-a); return tuple(round(ca[i]+(cb[i]-ca[i])*u) for i in range(3))
    return STOPS[-1][1]
def mix(c, d, u): return tuple(round(c[i]*(1-u)+d[i]*u) for i in range(3))
hexc = lambda c: "#%02X%02X%02X" % c

def glyph(i, x, y, s):
    u = s/24.0
    def T(px, py): return f"{x+px*u:.1f},{y+py*u:.1f}"
    w = f'stroke="#fff" stroke-width="{1.7*u:.1f}" stroke-linejoin="round" stroke-linecap="round" fill="none"'
    if i==0:   # deck
        return f'<g {w}><path d="M{T(5,8.5)} L{T(16,8.5)} L{T(16,15.5)} L{T(5,15.5)} Z" opacity=".55"/><path d="M{T(8,11)} L{T(19,11)} L{T(19,18)} L{T(8,18)} Z" fill="#fff" fill-opacity=".9"/></g>'
    if i==1:   # page with fold
        return f'<g {w}><path d="M{T(7.5,4.5)} L{T(14,4.5)} L{T(17.5,8)} L{T(17.5,19.5)} L{T(7.5,19.5)} Z" fill="#fff" fill-opacity=".9"/></g>'
    if i==2:   # cube
        return f'<g {w}><path d="M{T(12,4.5)} L{T(18.5,8.2)} L{T(18.5,15.8)} L{T(12,19.5)} L{T(5.5,15.8)} L{T(5.5,8.2)} Z"/><path d="M{T(12,12)} L{T(18.5,8.2)} L{T(18.5,15.8)} L{T(12,19.5)} Z" fill="#fff" fill-opacity=".9"/><path d="M{T(5.5,8.2)} L{T(12,12)}"/></g>'
    if i==3:   # spread
        return f'<g {w}><path d="M{T(4.5,7)} L{T(11.4,8)} L{T(11.4,18)} L{T(4.5,17)} Z" fill="#fff" fill-opacity=".9"/><path d="M{T(12.6,8)} L{T(19.5,7)} L{T(19.5,17)} L{T(12.6,18)} Z" fill="#fff" fill-opacity=".9"/></g>'
    return ''.join(f'<circle cx="{x+px*u:.1f}" cy="{y+py*u:.1f}" r="{1.9*u:.1f}" fill="#fff" fill-opacity=".92"/>' for px in (8.5,15.5) for py in (8.5,15.5))

def svg(mode="mac", detail="full", P="", compact=False):
    T = 824 if mode=="mac" else 1024
    cx = cy = 512
    k = 1.0 if mode=="mac" else 1.08
    if mode!="mac" and detail=="simple": k = 1.0
    simple = detail=="simple"
    tile = RRECT if compact else squircle(cx, cy, T)
    rows = [0.50, 0.64, 0.80, 1.0] if not simple else [0.64, 0.81, 1.0]
    cols = 5 if not simple else 3
    S = (138 if not simple else 192)*k          # front tile size
    pitch = S*(1.11 if not simple else 1.16)
    gap = (0.17 if not simple else 0.16)
    span = sum(S*sc*(1+gap) for sc in rows[:-1])
    y_front = cy + (span + S)/2 + 10*k
    g=[f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
 <linearGradient id="{P}bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#24212B"/><stop offset="1" stop-color="#0B0A0E"/></linearGradient>
 <radialGradient id="{P}bloom" cx="0.5" cy="0.62" r="0.5"><stop offset="0" stop-color="#FF4F7A" stop-opacity="0.26"/><stop offset="0.25" stop-color="#C255D9" stop-opacity="0.15"/><stop offset="0.5" stop-color="#9A5CFF" stop-opacity="0.07"/><stop offset="0.75" stop-color="#9A5CFF" stop-opacity="0.02"/><stop offset="1" stop-color="#9A5CFF" stop-opacity="0"/></radialGradient>
 <linearGradient id="{P}rim" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.22"/><stop offset="0.3" stop-color="#fff" stop-opacity="0.04"/><stop offset="1" stop-color="#fff" stop-opacity="0.08"/></linearGradient>
 <linearGradient id="{P}sheen" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity="0.30"/><stop offset="0.5" stop-color="#fff" stop-opacity="0"/><stop offset="1" stop-color="#000" stop-opacity="0.12"/></linearGradient>
 <filter id="{P}tileshadow" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="12" stdDeviation="14" flood-color="#000" flood-opacity="0.5"/></filter>
 <filter id="{P}appshadow" x="-40%" y="-40%" width="180%" height="190%"><feDropShadow dx="0" dy="{10*k:.1f}" stdDeviation="{12*k:.1f}" flood-color="#000" flood-opacity="0.55"/></filter>
 <clipPath id="{P}tclip"><path d="{tile}"/></clipPath>
</defs>''']
    if mode=="mac" and not compact: g.append(f'<path d="{tile}" fill="#000" filter="url(#{P}tileshadow)"/>')
    g.append(f'<path d="{tile}" fill="url(#{P}bg)"/>')
    g.append(f'<g clip-path="url(#{P}tclip)"><rect width="1024" height="1024" fill="url(#{P}bloom)"/>')
    dark = (0x1E,0x1B,0x25)
    # rows back to front; vertical position follows scale (perspective)
    for ri, sc in enumerate(rows):
        s = S*sc
        y = y_front - S - sum(S*q*(1+gap) for q in rows[ri:-1]) + (S - s)*0.0
        fade = (1-sc)*1.15                      # back rows sink into the tile
        for c in range(cols):
            off = c-(cols-1)/2
            x = cx + off*pitch*sc - s/2
            t = c/(cols-1)
            face = mix(grad(t), dark, min(fade,0.85))
            side = mix(face, (0,0,0), 0.5)
            r = s*0.25
            d = abs(off)/((cols-1)/2) * s*0.2
            sx = x + (d if off<0 else -d)
            filt = "" if compact else f' filter="url(#{P}appshadow)"'
            if compact and ri==len(rows)-1:
                g.append(f'<rect x="{x:.1f}" y="{y+8*k:.1f}" width="{s:.1f}" height="{s:.1f}" rx="{r:.1f}" fill="#000" fill-opacity=".35"/>')
            if d>0:
                g.append(f'<rect x="{sx:.1f}" y="{y:.1f}" width="{s:.1f}" height="{s:.1f}" rx="{r:.1f}" fill="{hexc(side)}"{filt if ri==len(rows)-1 else ""}/>')
            g.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{s:.1f}" height="{s:.1f}" rx="{r:.1f}" fill="{hexc(face)}"{filt if (ri==len(rows)-1 and d==0) else ""}/>')
            if ri==len(rows)-1 and not simple and not compact:
                g.append(glyph(c, x, y, s))
            if ri >= len(rows)-2:
                g.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{s:.1f}" height="{s:.1f}" rx="{r:.1f}" fill="url(#{P}sheen)" opacity="{1 if ri==len(rows)-1 else .5}"/>')
    g.append('</g>')
    g.append(f'<path d="{tile}" fill="none" stroke="url(#{P}rim)" stroke-width="{3*(T/824):.1f}" clip-path="url(#{P}tclip)"/>')
    g.append('</svg>')
    return "\n".join(g)

if __name__=="__main__":
    open(sys.argv[3],"w").write(svg(sys.argv[1], sys.argv[2]))
