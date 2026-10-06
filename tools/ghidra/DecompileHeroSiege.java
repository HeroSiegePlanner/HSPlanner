// @category HSPlanner

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.SourceType;

public class DecompileHeroSiege extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length != 2) {
            throw new IllegalArgumentException("Expected output directory and timeout seconds");
        }
        Path output = Path.of(args[0]);
        int timeout = Integer.parseInt(args[1]);
        if (getProjectRootFolder().getFiles().length > 1) {
            throw new IllegalStateException("Expected one root program in this helper's project");
        }
        String expectedHash = "9b5b5d068e96914db2de98b482d380ec1b905443c446bff25dd360f8e11398ba";
        if (!expectedHash.equals(currentProgram.getExecutableSHA256())) {
            throw new IllegalStateException("Project executable fingerprint does not match verified Hero Siege");
        }
        if (!currentProgram.getImageBase().equals(toAddr("140000000"))) {
            throw new IllegalStateException("Project image base differs from the verified PE function ranges");
        }
        for (String line : Files.readAllLines(output.resolve("symbols.tsv"))) {
            monitor.checkCancelled();
            String[] fields = line.split("\t");
            Address address = toAddr(fields[0]);
            if (currentProgram.getSymbolTable().getSymbol(fields[1], address, null) == null) {
                createLabel(address, fields[1], true, SourceType.USER_DEFINED);
            }
        }
        Path identifiers = output.resolve("identifiers.tsv");
        if (Files.exists(identifiers)) {
            for (String line : Files.readAllLines(identifiers)) {
                monitor.checkCancelled();
                String[] fields = line.split("\t");
                Address address = toAddr(fields[0]);
                if (currentProgram.getSymbolTable().getSymbol(fields[1], address, null) == null) {
                    createLabel(address, fields[1], true, SourceType.USER_DEFINED);
                }
            }
        }

        DecompInterface decompiler = new DecompInterface();
        DecompileOptions options = new DecompileOptions();
        options.grabFromProgram(currentProgram);
        options.setMaxInstructions(2000000);
        options.setMaxPayloadMBytes(256);
        decompiler.setOptions(options);
        try {
            if (!decompiler.openProgram(currentProgram)) {
                throw new IllegalStateException(decompiler.getLastMessage());
            }
            for (String line : Files.readAllLines(output.resolve("targets.tsv"))) {
                monitor.checkCancelled();
                String[] fields = line.split("\t");
                Address start = toAddr(fields[0]);
                Address end = toAddr(fields[1]);
                String name = fields[2];
                println("Disassembling " + name + " at " + start);
                AddressSet range = new AddressSet(start, end.subtract(1));
                DisassembleCommand disassemble = new DisassembleCommand(start, range, true);
                if (!disassemble.applyTo(currentProgram, monitor)) {
                    throw new IllegalStateException("Disassembly failed: " + disassemble.getStatusMsg());
                }
                Function function = getFunctionAt(start);
                if (function == null) {
                    function = createFunction(start, name);
                }
                if (function == null) {
                    throw new IllegalStateException("Cannot create function at " + start);
                }
                function.setName(name, SourceType.USER_DEFINED);
                println("Decompiling " + name + " (timeout " + timeout + "s)");
                decompiler.flushCache();
                DecompileResults result = decompiler.decompileFunction(function, timeout, monitor);
                if (!result.decompileCompleted() || result.getDecompiledFunction() == null) {
                    throw new IllegalStateException(name + ": " + result.getErrorMessage());
                }
                String header = "/* Ghidra C pseudocode; not original game source.\n"
                    + " * Source SHA-256: " + expectedHash + "\n"
                    + " * Function: " + name + " @ " + start + "\n"
                    + " * Targeted analysis only; inferred types and signatures need verification.\n */\n\n";
                String filename = fields.length >= 4 ? fields[3] : fields[0] + "_" + name + ".c";
                Path file = output.resolve(filename);
                Files.writeString(file, header + result.getDecompiledFunction().getC(), StandardCharsets.UTF_8);
                println("Exported " + file);
            }
        } finally {
            decompiler.dispose();
        }
        Files.writeString(output.resolve("complete.txt"), "All requested functions decompiled.\n");
    }
}
