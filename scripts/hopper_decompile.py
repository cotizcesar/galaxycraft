"""Hopper-side batch decompiler for REA's alternates pass.

Run inside Hopper:
  HOPPER_LIST=/path/list.tsv HOPPER_OUT=/path/out \
  Hopper -l WinPE --intel-32 --analysis -Y hopper_decompile.py \
    --executable /path/image.executable

The list has `address<TAB>name<TAB>size` rows.  The script waits for Hopper's
background analysis, decompiles each procedure at its entry, writes
`<address>_<name>.c`, and exits the Hopper process.  One Hopper process handles
the whole list sequentially.
"""
import os
import time
import traceback


def sanitize(value):
    return "".join(c if c.isalnum() or c in "_.-" else "_" for c in value)


def main():
    out = os.environ["HOPPER_OUT"]
    list_path = os.environ["HOPPER_LIST"]
    os.makedirs(out, exist_ok=True)
    doc = Document.getCurrentDocument()
    waited = 0
    while doc.backgroundProcessActive() and waited < 1800:
        time.sleep(1)
        waited += 1
    rows = []
    with open(list_path, "r") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("address\t"):
                continue
            fields = line.split("\t")
            rows.append((fields[0], fields[1] if len(fields) > 1 else fields[0]))
    ok = failed = 0
    with open(os.path.join(out, "hopper-status.tsv"), "w") as status:
        status.write("address\tname\tstatus\tdetail\n")
        for hexaddr, name in rows:
            try:
                addr = int(hexaddr, 16)
                segment = doc.getSegmentAtAddress(addr)
                procedure = segment.getProcedureAtAddress(addr) if segment else None
                if procedure is None:
                    failed += 1
                    status.write("%s\t%s\tmissing\tno procedure\n" % (hexaddr, name))
                    continue
                text = procedure.decompile()
                if not text:
                    failed += 1
                    status.write("%s\t%s\tempty\tdecompile returned empty\n" % (hexaddr, name))
                    continue
                path = os.path.join(out, "%s_%s.c" % (hexaddr.lower(), sanitize(name)))
                with open(path, "w") as f:
                    f.write("// %s %s hopper\n" % (hexaddr, name))
                    f.write(text)
                    if not text.endswith("\n"):
                        f.write("\n")
                ok += 1
                status.write("%s\t%s\tok\t%s\n" % (hexaddr, name, path))
            except Exception as error:
                failed += 1
                status.write("%s\t%s\terror\t%s\n" % (
                    hexaddr, name, traceback.format_exc(limit=1).replace("\n", " ")))
        status.write("# summary\tok=%d\tfailed=%d\n" % (ok, failed))
    os._exit(0)


try:
    main()
except Exception:
    try:
        with open(os.path.join(os.environ.get("HOPPER_OUT", "/tmp"), "hopper-fatal.txt"), "w") as f:
            f.write(traceback.format_exc())
    except Exception:
        pass
    os._exit(1)
