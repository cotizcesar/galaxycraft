"""OCR the retail captures (hp/shields, armor and weapon tooltips) and compare
with the executable's tables.  Units whose panel name did not match are
reported as not read."""
import sys, os, re, subprocess
shots, root, log = sys.argv[1], sys.argv[2], sys.argv[3]

def table(name):
    lines = open(f'{root}/fixtures/{name}').read().splitlines()
    head = lines[0].split('\t')
    return {int(l.split('\t')[0]): dict(zip(head, map(int, l.split('\t')))) for l in lines[1:]}
units, weapons = table('oracle_units.tsv'), table('oracle_weapons.tsv')

def ocr(path, psm='6'):
    big = path + '.o.png'
    subprocess.run(['magick', path, '-resize', '400%', '-colorspace', 'Gray', '-negate', '-threshold', '55%', big], check=True)
    return subprocess.run(['tesseract', big, '-', '--psm', psm], capture_output=True, text=True).stdout

n = good_n = unread = 0
for line in open(log).read().splitlines():
    t, name, score, seen = (line.split('\t') + ['', ''])[:4]
    t = int(t)
    g = re.sub(r'[^a-z0-9]', '', seen.split(':')[0].lower()).replace('l', 'i')
    nm = re.sub(r'[^a-z0-9]', '', name.lower()).replace('l', 'i')
    if len(g) >= 10 and nm.endswith(g[2:]):
        score = '1'
    if float(score) < 0.9:
        print(f"{t}\t{name}\tNOT READ\tpanel showed [{seen.split(':')[0].strip()}]")
        unread += 1
        continue
    d = f'{shots}/{t:03d}'
    u = units[t]
    big = f'{d}/hp.png.o.png'
    subprocess.run(['magick', f'{d}/hp.png', '-resize', '400%', '-colorspace', 'Gray', '-negate', big], check=True)
    hp = subprocess.run(['tesseract', big, '-', '--psm', '6', '-c', 'tessedit_char_whitelist=0123456789/'],
                        capture_output=True, text=True).stdout
    pairs = [(int(a), int(b)) for a, b in re.findall(r'(\d+)\s*/\s*(\d+)', hp)]
    tips = ' '.join(ocr(f'{d}/tip{k}.png') for k in range(3))
    tips = tips.replace('O', '0')
    armor = re.findall(r'Armor[:;.]?\s*(\d+)', tips)
    dmg = sorted(set(int(x) for x in re.findall(r'Damage[:;.]?\s*(\d+)', tips)))
    src = units[u['turret']] if u['turret'] < 228 else u
    # The tooltip shows damage x hits per attack (Firebat 8x2 = 16).
    # The tooltip shows the weapon damage, doubled for two-blow weapons
    # (Firebat 16, Goliath air 20): the base damage must divide it by 1 or 2.
    want_dmg = sorted(set(weapons[w]['damage'] for w in (src['ground_weapon'], src['air_weapon']) if w != 130))
    checks = []
    # Protoss panels print hp then shields; take the pair equal to max hp
    # when present (the OCR may drop a digit of another line).
    want_hp = u['hp'] // 256
    got_hp = want_hp if any(b == want_hp and a == want_hp for a, b in pairs) else (pairs[0][1] if pairs else None)
    checks.append(('hp', got_hp, u['hp'] // 256, got_hp == u['hp'] // 256))
    if u['has_shield']:
        got = u['shield'] if any(a == b == u['shield'] for a, b in pairs) else (pairs[1][1] if len(pairs) > 1 else None)
        checks.append(('shields', got, u['shield'], got == u['shield']))
    if armor:
        checks.append(('armor', int(armor[0]), u['armor'], int(armor[0]) == u['armor']))
    if dmg:
        # Every weapon the unit owns must show its damage (a spell such as
        # Spider Mines may add another line).
        checks.append(('damage', dmg, want_dmg,
                       all(any(x in (b, 2 * b) for x in dmg) for b in want_dmg)))
    good = all(c[3] for c in checks)
    n += 1; good_n += good
    cs = '; '.join(f"{k} {g}/{w}{'' if o else ' DIFF'}" for k, g, w, o in checks)
    print(f"{t}\t{name}\t{'ok' if good else 'CHECK'}\t{cs}")
print(f"# read {n}, match {good_n}, not read {unread}", file=sys.stderr)
