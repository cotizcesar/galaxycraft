"""Build a Use-Map-Settings .scm with one unit of every type owned by
player 1, laid out 5 per row (160 px apart) for panel-by-panel reading."""
import struct, sys, collections

SRC, OUT, ROOT = sys.argv[1], sys.argv[2], sys.argv[3]

# --- MPQ crypto -----------------------------------------------------------
crypt = [0] * 0x500
seed = 0x00100001
for i in range(0x100):
    idx = i
    for _ in range(5):
        seed = (seed * 125 + 3) % 0x2AAAAB
        hi = (seed & 0xFFFF) << 16
        seed = (seed * 125 + 3) % 0x2AAAAB
        crypt[idx] = hi | (seed & 0xFFFF)
        idx += 0x100

def hs(s, t):
    s1, s2 = 0x7FED7FED, 0xEEEEEEEE
    for ch in s.upper():
        c = ord(ch)
        s1 = (crypt[t * 0x100 + c] ^ (s1 + s2)) & 0xFFFFFFFF
        s2 = (c + s1 + s2 + (s2 << 5) + 3) & 0xFFFFFFFF
    return s1

def encrypt(data, key):
    out = bytearray()
    s2 = 0xEEEEEEEE
    for (v,) in struct.iter_unpack('<I', data):
        s2 = (s2 + crypt[0x400 + (key & 0xFF)]) & 0xFFFFFFFF
        out += struct.pack('<I', (v ^ (key + s2)) & 0xFFFFFFFF)
        key = ((~key << 0x15) + 0x11111111 | (key >> 0x0B)) & 0xFFFFFFFF
        s2 = (v + s2 + (s2 << 5) + 3) & 0xFFFFFFFF
    return bytes(out)

def mpq(files):
    hsize = 16
    body = bytearray()
    blocks = []
    for name, data in files:
        blocks.append((32 + len(body), len(data), len(data), 0x80000000))
        body += data
    htab = [(0xFFFFFFFF, 0xFFFFFFFF, 0xFFFF, 0xFFFF, 0xFFFFFFFF)] * hsize
    for bi, (name, _) in enumerate(files):
        i = hs(name, 0) % hsize
        while htab[i][4] != 0xFFFFFFFF:
            i = (i + 1) % hsize
        htab[i] = (hs(name, 1), hs(name, 2), 0, 0, bi)
    ht = b''.join(struct.pack('<IIHHI', *e) for e in htab)
    bt = b''.join(struct.pack('<IIII', *b) for b in blocks)
    ht = encrypt(ht, hs('(hash table)', 3))
    bt = encrypt(bt, hs('(block table)', 3))
    hpos = 32 + len(body)
    bpos = hpos + len(ht)
    total = bpos + len(bt)
    hdr = struct.pack('<4sIIHHIIII', b'MPQ\x1a', 32, total, 0, 3, hpos, bpos, hsize, len(blocks))
    return hdr + bytes(body) + ht + bt

# --- CHK ----------------------------------------------------------------
b = open(SRC, 'rb').read()
secs = collections.OrderedDict()
o = 0
order = []
while o + 8 <= len(b):
    n = b[o:o + 4]; l = struct.unpack_from('<i', b, o + 4)[0]
    secs.setdefault(n, b[o + 8:o + 8 + l]); order.append(n); o += 8 + l

# Players: slot 0 human, every other slot inactive.
secs[b'OWNR'] = bytes([6, 5] + [0] * 6 + [0] * 4)  # slot 1: an idle computer
secs[b'IOWN'] = secs[b'OWNR']
secs[b'SIDE'] = bytes([1, 0] + [5] * 6 + [7, 7, 7, 4])  # Terran, Zerg
secs[b'TRIG'] = b''  # no victory/defeat: the game runs until quit
# Every tile the map's most common (walkable platform) tile.
mt = secs[b'MTXM']
common = 64  # fully walkable platform floor (cv5/vf4 checked)
secs[b'MTXM'] = struct.pack('<H', common) * (len(mt) // 2)
secs[b'TILE'] = secs[b'MTXM']

flags = {}
names = {}
for line in open(f'{ROOT}/fixtures/oracle_units.tsv').read().splitlines()[1:]:
    f = line.split('\t'); flags[int(f[0])] = int(f[11])
for line in open(f'{ROOT}/docs/UNIT-COMPARISON.md'):
    p = [x.strip() for x in line.split('|')]
    if len(p) > 3 and p[1].isdigit() and int(p[1]) not in names:
        names[int(p[1])] = p[2]
skip = {14, 33, 73, 85, 91, 92, 101, 105, 119, 121, 145, 153, 158, 161, 202, 204, 214}
types = [t for t in range(228)
         if t not in skip and not flags[t] & 0x10 and not flags[t] & 0x800
         and 'Unused' not in names.get(t, '')]
import os
if os.environ.get('IDS'):
    order = [int(x) for x in os.environ['IDS'].split(',')]
    types = [t for t in order if t in types]
elif os.environ.get('ONLY'):
    lo, hi = map(int, os.environ['ONLY'].split('-'))
    types = [t for t in types if lo <= t <= hi]
# Keep the start location record(s) of the source map for slot 0.
start = [r for r in (secs[b'UNIT'][i:i + 36] for i in range(0, len(secs[b'UNIT']), 36))
         if struct.unpack_from('<H', r, 8)[0] == 214 and r[16] in (0, 1)]
units = bytearray(b''.join(start))
layout = []
for k, t in enumerate(types):
    block, kk = divmod(k, 120)
    x = 96 + 160 * (kk % 5) + 1024 * block
    y = 112 + 160 * (kk // 5)
    serial = 1000 + k
    units += struct.pack('<IHHHHHHBBBBIHHII', serial, x, y, t, 0, 0x1f, 0x1f, 0, 100, 100, 100, 0, 0, 0, 0, 0)
    layout.append((t, x, y, names.get(t, '?')))
    if os.environ.get('CAGE'):
        # A ring of Pylons (64 px) keeps a wandering critter in place.
        for dx in (-64, 0, 64):
            for dy in (-64, 0, 64):
                if dx or dy:
                    units += struct.pack('<IHHHHHHBBBBIHHII', 5000 + 10 * k + (dx // 64 + 1) * 3 + dy // 64 + 1,
                                         x + dx, y + dy, 156, 0, 0x1f, 0x1f, 0, 100, 100, 100, 0, 0, 0, 0, 0)
secs[b'UNIT'] = bytes(units)
out = b''.join(n + struct.pack('<i', len(secs[n])) + secs[n] for n in secs)
open(OUT, 'wb').write(mpq([('staredit\\scenario.chk', out)]))
with open(OUT + '.layout.tsv', 'w') as fh:
    for t, x, y, n in layout:
        fh.write(f'{t}\t{x}\t{y}\t{n}\n')
print(len(types), 'units, rows', (len(types) + 4) // 5, 'tile', common)
