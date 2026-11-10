// Targeted recovery decompiler for functions that failed the normal pass.
//
// Usage (headless, analysis already saved):
//   analyzeHeadless <projdir> <proj> -process <program> -noanalysis \
//     -scriptPath scripts/ghidra \
//     -postScript DecompileList.java <outDir> <list.tsv> <timeoutSecs> \
//       <maxInstructions> <maxPayloadMB>
//
// The list file may be the failure report (first column = address, optional
// tab-separated name/size/reason).  Successful bodies are written as
// `<address>_<name>.c`; failures are reported on stdout, never silently
// dropped.

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.Writer;

public class DecompileList extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        File list = new File(args[1]);
        int timeout = args.length > 2 ? Integer.parseInt(args[2]) : 180;
        int maxInstructions = args.length > 3 ? Integer.parseInt(args[3]) : 1000000;
        int maxPayloadMB = args.length > 4 ? Integer.parseInt(args[4]) : 128;
        out.mkdirs();

        DecompileOptions options = new DecompileOptions();
        options.setDefaultTimeout(timeout);
        options.setMaxInstructions(maxInstructions);
        options.setMaxPayloadMBytes(maxPayloadMB);

        DecompInterface ifc = new DecompInterface();
        ifc.setOptions(options);
        if (!ifc.openProgram(currentProgram)) {
            throw new RuntimeException("decompiler open failed");
        }

        int ok = 0;
        int failed = 0;
        try (BufferedReader r = new BufferedReader(
                new InputStreamReader(new FileInputStream(list), "UTF-8"))) {
            String line;
            while ((line = r.readLine()) != null) {
                if (monitor.isCancelled()) {
                    break;
                }
                line = line.trim();
                if (line.isEmpty() || line.startsWith("file\t") || line.startsWith("address\t")) {
                    continue;
                }
                String[] fields = line.split("\t");
                String hex = fields[0].trim();
                if (hex.startsWith("0x") || hex.startsWith("0X")) {
                    hex = hex.substring(2);
                }
                Address addr = currentProgram.getAddressFactory()
                        .getDefaultAddressSpace().getAddress(hex);
                Function f = currentProgram.getFunctionManager().getFunctionAt(addr);
                if (f == null) {
                    f = currentProgram.getFunctionManager().getFunctionContaining(addr);
                }
                if (f == null) {
                    println("RECOVERY missing-function " + hex);
                    failed++;
                    continue;
                }
                String name = f.getName().replaceAll("[^A-Za-z0-9_.-]", "_");
                File file = new File(out, hex + "_" + name + ".c");
                try {
                    DecompileResults result = ifc.decompileFunction(f, timeout, monitor);
                    if (result != null && result.decompileCompleted()
                            && result.getDecompiledFunction() != null) {
                        try (Writer w = new OutputStreamWriter(
                                new FileOutputStream(file), "UTF-8")) {
                            w.write("// " + f.getEntryPoint() + " " + f.getName()
                                    + " size=" + f.getBody().getNumAddresses() + "\n");
                            w.write(result.getDecompiledFunction().getC());
                        }
                        ok++;
                        println("RECOVERY ok " + hex + " " + f.getName());
                    } else {
                        failed++;
                        String message = result == null ? "null result"
                                : String.valueOf(result.getErrorMessage());
                        println("RECOVERY failed " + hex + " " + f.getName() + " :: "
                                + message.replace('\n', ' '));
                    }
                } catch (Exception e) {
                    failed++;
                    println("RECOVERY error " + hex + " " + f.getName() + " :: "
                            + e.getClass().getSimpleName() + " " + e.getMessage());
                }
            }
        }
        println("DecompileList ok=" + ok + " failed=" + failed
                + " timeout=" + timeout + "s maxInstructions=" + maxInstructions
                + " maxPayloadMB=" + maxPayloadMB);
        ifc.dispose();
    }
}
