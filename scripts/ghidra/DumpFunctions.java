// Dump the discovered function inventory of an analysed program.
//
// Usage (headless, analysis already saved):
//   analyzeHeadless <projdir> <proj> -process <program> -noanalysis \
//     -scriptPath scripts/ghidra -postScript DumpFunctions.java <output.tsv>

import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;

import java.io.PrintWriter;

public class DumpFunctions extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        String out = args.length > 0 ? args[0] : "/tmp/opencode/functions_full.tsv";
        FunctionManager fm = currentProgram.getFunctionManager();
        try (PrintWriter w = new PrintWriter(out)) {
            w.println("address\tname\tsize\tthunk");
            for (Function f : fm.getFunctions(true)) {
                w.println(f.getEntryPoint() + "\t" + f.getName() + "\t" +
                        f.getBody().getNumAddresses() + "\t" + (f.isThunk() ? "1" : "0"));
            }
        }
        println("DumpFunctions wrote " + fm.getFunctionCount() + " functions to " + out);
    }
}
