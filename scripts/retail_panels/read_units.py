"""Click every unit of the AllUnits map in retail; verify the panel name by
OCR (retrying nearby points), then capture the panel and three tooltips."""
import sys, time, subprocess, os, re, difflib
from evdev import UInput, ecodes as e

layout, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
ui = UInput({e.EV_REL: [e.REL_X, e.REL_Y], e.EV_KEY: [e.BTN_LEFT, e.BTN_RIGHT]}, name="gc-reader")
time.sleep(0.5)

def move(x, y):
    subprocess.run(["hyprctl", "dispatch", f"hl.dsp.cursor.move({{ x = {x}, y = {y} }})"], capture_output=True)
    time.sleep(0.08)
    ui.write(e.EV_REL, e.REL_X, 1); ui.syn(); time.sleep(0.03)
    ui.write(e.EV_REL, e.REL_X, -1); ui.syn(); time.sleep(0.12)

def click(x, y):
    move(x, y)
    ui.write(e.EV_KEY, e.BTN_LEFT, 1); ui.syn(); time.sleep(0.07)
    ui.write(e.EV_KEY, e.BTN_LEFT, 0); ui.syn()

def grab(path, geo="1150,1140 1250x300"):
    subprocess.run(["grim", "-g", geo, path])

def ocr(path, psm="7"):
    big = path + ".big.png"
    subprocess.run(["magick", path, "-resize", "300%", "-colorspace", "Gray", "-negate", big])
    return subprocess.run(["tesseract", big, "-", "--psm", psm], capture_output=True, text=True).stdout

def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower()).replace("l", "i")

rows = [l.split('\t') for l in open(layout).read().splitlines()]
only = set(int(x) for x in os.environ.get("ONLY_IDS", "").split(",") if x)
cam = None
for t, ux, uy, name in rows:
    t, ux, uy = int(t), int(ux), int(uy)
    if only and t not in only:
        continue
    cx = 416 if ux < 1024 else 1440
    cy = max(uy, 240)
    if cam != (cx, cy):
        click(452 + round(cx / 10.64), 1040 + round(cy / 10.64))
        time.sleep(0.7)
        cam = (cx, cy)
    d = f"{out}/{t:03d}"
    os.makedirs(d, exist_ok=True)
    best = (0, "")
    for dx, dy in ((0, 0), (0, -12), (0, 12), (-12, 0), (12, 0), (0, -24), (0, 24), (-24, 0), (24, 0), (0, -40), (0, 40)):
        click(1718 + 3 * (ux + 80 - cx), 720 + 3 * (uy + 80 - cy))  # empty gap
        time.sleep(0.25)
        click(1718 + 3 * (ux + dx - cx), 720 + 3 * (uy + dy - cy))
        time.sleep(0.45)
        grab(f"{d}/name.png", "1590,1155 520x55")
        got = ocr(f"{d}/name.png").strip()
        r = difflib.SequenceMatcher(None, norm(got), norm(name)).ratio()
        g = norm(got)
        # A long name is cut at the left edge of the crop.
        if len(g) >= 10 and g[2:] and norm(name).endswith(g[2:]):
            r = 1.0
        if r > best[0]:
            best = (r, got)
        if r > 0.9:
            break
    grab(f"{d}/panel.png")
    grab(f"{d}/hp.png", "1215,1350 345x90")
    for k, ix in enumerate((1555, 1670, 1785)):
        move(ix, 1398)
        time.sleep(0.5)
        grab(f"{d}/tip{k}.png", "1500,1355 900x85")
    print(f"{t}\t{name}\t{best[0]:.2f}\t{best[1]}", flush=True)
ui.close()
