package be.uclouvain.ingi.aia;

import org.chocosolver.solver.Model;
import org.chocosolver.solver.Solution;
import org.chocosolver.solver.Solver;
import org.chocosolver.solver.exception.ContradictionException;
import org.chocosolver.solver.search.limits.FailCounter;
import org.chocosolver.solver.search.loop.lns.INeighborFactory;
import org.chocosolver.solver.search.loop.lns.neighbors.IntNeighbor;
import org.chocosolver.solver.search.strategy.Search;
import org.chocosolver.solver.variables.IntVar;

import java.util.*;
import java.util.function.BiConsumer;

public class TsptwSolver {


    private final TsptwInstance instance;
    private final int           timeout;
    private final int[]         initial;
    private List<BiConsumer<int[],Integer>> observers = new LinkedList<>();

    public void addObserver(BiConsumer<int[],Integer> observer) {
        observers.add(observer);
    }

    private void notifySolution(int [] solution, int objective) {
        for (BiConsumer<int[],Integer> observer: observers) {
            observer.accept(solution, objective);
        }
    }

    public TsptwSolver(final TsptwInstance instance, final int timeout, final int[] initial) {
        this.instance = instance;
        this.timeout  = timeout;
        this.initial  = initial;
    }

    public Solver solve() {

        int totDist = 0;
        for (int i = 0; i < this.initial.length-1; i++) {
            totDist += instance.distances[this.initial[i]][this.initial[i+1]];
        }
        totDist += instance.distances[this.initial[this.initial.length-1]][0];



        final Tsptw.TsptwModel tsptw  = Tsptw.buildModel(instance,totDist);
        final Model            model  = tsptw.model;

        Solver solver = model.getSolver();
        solver.limitTime(this.timeout * 1000L); // its in milliseconds


        Solution initial = new Solution(model, tsptw.x);
        for (int i = 0; i < this.initial.length; i++) {
            initial.setIntVal(tsptw.x[i], this.initial[i]);
        }



        solver.setLNS(new ConsecutiveNeighborhood(tsptw.x,5,31), new FailCounter(solver, 30), initial);
        //solver.setLNS(INeighborFactory.random(tsptw.x), new FailCounter(solver, 50), initial);

        solver.setSearch(
               Search.sequencer(
                    Search.intVarSearch(tsptw.x),
                    Search.intVarSearch(tsptw.dist)
                )
        );

        while (solver.solve()) {
            int [] solution = Arrays.stream(tsptw.x).mapToInt(x -> x.getValue()).toArray();
            notifySolution(solution,tsptw.totdist.getValue());
        }

        return solver;
    }


}

class ConsecutiveNeighborhood extends IntNeighbor {
    protected final int n;
    private Random rd;
    private int window;

    public ConsecutiveNeighborhood(IntVar[] vars, int window, long seed) {
        super(vars);
        this.n = vars.length;
        this.rd = new Random(seed);
        this.window = window;
    }

    public void recordSolution() {
        super.recordSolution();
    }

    public void loadFromSolution(Solution solution) {
        super.loadFromSolution(solution);
    }

    @Override
    public void fixSomeVariables() throws ContradictionException {
        int start = rd.nextInt(n);
        for (int i = 0; i < n; i++) {
            if (i < start || i >= start+window) {
                this.freeze(i);
            }
        }
    }
}
