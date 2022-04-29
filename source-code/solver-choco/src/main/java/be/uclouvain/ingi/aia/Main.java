package be.uclouvain.ingi.aia;

import java.util.Arrays;
import java.util.StringJoiner;
import java.util.stream.IntStream;

import org.apache.commons.cli.CommandLine;
import org.apache.commons.cli.CommandLineParser;
import org.apache.commons.cli.DefaultParser;
import org.apache.commons.cli.Options;
import org.apache.commons.cli.ParseException;
import org.chocosolver.solver.Solver;

public class Main {
    private final String fname;
    private final int[]  initial;
    private final int    timeout;

    private final long   start;
    private long         time;
    private double       objective;
    private int[]        solution;
    private boolean      improved;
    private boolean      crashed;
    private boolean      closed;
    private String       error;

    public Main(final String fname, final int timeout, final int[] initial, final double icost) {
        this.fname     = fname;
        this.timeout   = timeout;
        this.initial   = initial;

        this.start     = System.currentTimeMillis();
        this.time      = this.start;
        this.objective = icost;
        this.solution  = initial.clone();
        this.improved  = false;
        this.crashed   = false;
        this.error     = null;
        this.closed    = false;
    }

    public static void main(final String[] args) throws Exception {
        CommandLine cli = cli(args);

        String fname   = cli.getOptionValue("f");
        int    timeout = Integer.parseInt(cli.getOptionValue("t", "600"));
        int[]  initial = initial(cli.getOptionValue("s"));
        double icost   = Double.parseDouble(cli.getOptionValue("c"));

        Main main = new Main(fname, timeout, initial, icost);
        main.solve();

        StringJoiner join = new StringJoiner(" ");
        Arrays.stream(main.solution)
            .skip(1)
            .forEach(x -> join.add(""+x));
            
        String solution = join.toString();
        String out = String.format("%10s | %10s | %10s | %10.2f | %10.2f | %s",
            instanceName(fname),
            "choco",
            main.status(),
            main.objective,
            (main.time - main.start) / 1000.0,
            main.crashed ? main.error : solution);
        
        System.out.println(out);
    }

    private String status() {
        if (crashed) {
            return "crashed";
        }

        if (closed && improved) {
            return "closed (improved)";
        }
        if (closed && !improved) {
            return "closed (initial)";
        }
        if (improved) {
            return "open (improved)";
        } else {
            return "open (initial)";
        }
    }

    private void solve() {
        try {
            TsptwInstance instance = TsptwParser.fromFile(fname);
            TsptwSolver solver     = new TsptwSolver(instance, timeout, initial);

            solver.addObserver((solution,objective) -> {
                // System.out.println(objective);
                this.time       = System.currentTimeMillis();
                this.objective  = (objective / 1000.0);
                this.improved   = true;
                System.arraycopy(solution, 0, this.solution, 0, this.solution.length);
            });

            Solver s     = solver.solve();
            this.closed  = s.isObjectiveOptimal();
        } catch (Throwable e) {
            this.crashed = true;
            this.error   = e.getMessage();
        }
    }

    private static final String instanceName(final String fname) {
        String[] chunks = fname.split("/");
        if (chunks.length < 2) {
            return chunks[0];
        } else {
            return String.format("%s/%s", chunks[chunks.length-2], chunks[chunks.length-1]);
        }
    }

    private static Options options() {
        Options options = new Options();
        options.addOption("f", true, "instance file");
        options.addOption("s", true, "initial solution");
        options.addOption("c", true, "initial solution cost");
        options.addOption("t", true, "timeout");
        return options;
    }

    private static CommandLine cli(final String[] args) throws ParseException {
        CommandLineParser parser = new DefaultParser();
        CommandLine cmd = parser.parse(options(), args);

        return cmd;
    }

    private static int[] initial(final String solution) {
        return IntStream.concat(
                IntStream.of(0),
                Arrays.stream(solution.split("\s+"))
                    .mapToInt(x -> Integer.parseInt(x))
            ).toArray();
    }
}
