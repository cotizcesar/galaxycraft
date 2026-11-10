// Decompile every function of an already-analysed program, in a resumable
// slice.  Writes one pseudocode file per function plus a per-batch manifest.
//
// Usage (headless, analysis already saved):
//   analyzeHeadless <projdir> <proj> -process <program> -noanalysis \
//     -scriptPath scripts/ghidra \
//     -postScript DecompileAll.java <outDir> <start> <count> [timeoutSeconds]
//
// Files are named `<index>_<address>_<name>.c`; an existing successful file is
// skipped, so rerunning a batch after a timeout resumes where it stopped and
// a later pass with a longer timeout retries only the failed functions.

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

import java.io.File;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.Writer;
import java.util.ArrayList;
import java.util.List;

public class DecompileAll extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args.length > 0 ? args[0] : "/tmp/opencode/decomp_full");
        int start = args.length > 1 ? Integer.parseInt(args[1]) : 0;
        int count = args.length > 2 ? Integer.parseInt(args[2]) : 500;
        int timeoutSeconds = args.length > 3 ? Integer.parseInt(args[3]) : 5;
        out.mkdirs();

        List<Function> funcs = new ArrayList<>();
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            funcs.add(f);
        }

        DecompInterface ifc = new DecompInterface();
        ifc.setOptions(new DecompileOptions());
        if (!ifc.openProgram(currentProgram)) {
            throw new RuntimeException("decompiler open failed");
        }

        int end = Math.min(funcs.size(), start + count);
        int written = 0;
        int skipped = 0;
        int failed = 0;
        File manifest = new File(out, String.format("manifest-%05d.tsv", start));
        try (Writer mw = new OutputStreamWriter(new FileOutputStream(manifest), "UTF-8")) {
            mw.write("index\taddress\tname\tsize\tstatus\n");
            for (int i = start; i < end; i++) {
                if (monitor.isCancelled()) {
                    break;
                }
                Function f = funcs.get(i);
                String addr = f.getEntryPoint().toString();
                String name = f.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
                long size = f.getBody().getNumAddresses();
                File file = new File(out, String.format("%05d_%s_%s.c", i, addr, name));
                if (file.exists() && !isFailed(file)) {
                    skipped++;
                    mw.write(i + "\t" + addr + "\t" + f.getName() + "\t" + size + "\tskipped\n");
                    continue;
                }
                try {
                    DecompileResults r = ifc.decompileFunction(f, timeoutSeconds, monitor);
                    if (r != null && r.decompileCompleted() && r.getDecompiledFunction() != null) {
                        try (Writer w = new OutputStreamWriter(new FileOutputStream(file), "UTF-8")) {
                            w.write("// " + addr + " " + f.getName() + " size=" + size + "\n");
                            w.write(r.getDecompiledFunction().getC());
                        }
                        written++;
                        mw.write(i + "\t" + addr + "\t" + f.getName() + "\t" + size + "\tok\n");
                    } else {
                        failed++;
                        try (Writer w = new OutputStreamWriter(new FileOutputStream(file), "UTF-8")) {
                            w.write("// " + addr + " " + f.getName() + " DECOMPILE_FAILED\n");
                            if (r != null) {
                                w.write(String.valueOf(r.getErrorMessage()));
                            }
                        }
                        mw.write(i + "\t" + addr + "\t" + f.getName() + "\t" + size + "\tfailed\n");
                    }
                } catch (Exception e) {
                    failed++;
                    mw.write(i + "\t" + addr + "\t" + f.getName() + "\t" + size + "\terror:" +
                            e.getClass().getSimpleName() + "\n");
                }
            }
        }
        println("DecompileAll " + start + ".." + end + " written=" + written +
                " skipped=" + skipped + " failed=" + failed + " total=" + funcs.size() +
                " timeout=" + timeoutSeconds + "s");
        ifc.dispose();
    }

    private boolean isFailed(File file) {
        try (java.io.BufferedReader r = new java.io.BufferedReader(
                new java.io.InputStreamReader(new java.io.FileInputStream(file), "UTF-8"))) {
            String first = r.readLine();
            return first != null && first.contains("DECOMPILE_FAILED");
        } catch (Exception e) {
            return false;
        }
    }
}
