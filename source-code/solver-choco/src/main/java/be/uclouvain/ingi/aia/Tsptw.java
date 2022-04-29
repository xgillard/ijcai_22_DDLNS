package be.uclouvain.ingi.aia;

import org.chocosolver.solver.Model;
import org.chocosolver.solver.variables.IntVar;
import org.chocosolver.solver.variables.impl.IntervalIntVarImpl;

import java.util.ArrayList;
import java.util.Arrays;

// The formulation is based on this example model from the choco documentation
// https://github.com/chocoteam/choco-solver/blob/master/examples/src/main/java/org/chocosolver/examples/integer/TSP.java

public class Tsptw {

    public static class TsptwModel {
        public final Model    model;
        public final IntVar[] x;
        public final IntVar[] dist;
        public final IntVar   totdist;

        public TsptwModel(final Model m, final IntVar[] x, final IntVar[] d, final IntVar t) {
            this.model   = m;
            this.x       = x;
            this.dist    = d;
            this.totdist = t;
        }
    }

    public static TsptwModel buildModel(TsptwInstance instance, int totDistUB) {

        int          nbNodes = instance.nbNodes;
        int[][]      distances = instance.distances;
        TimeWindow[] timeWindows = instance.timeWindows;

        Model model = new Model("tsptw");

        final int DEPOT   = 0;
        final int N       = nbNodes;
        final int HORIZON = Arrays.stream(timeWindows)
                .mapToInt(x -> x.latest)
                .max().getAsInt();

        final IntVar[] x    = new IntVar[N];

        for (int i = 0; i < N; i++) {
            x[i] = model.intVar("x"+i, 0, N-1); // interval domains
        }

        final IntVar[] earliest    = new IntVar[N];
        final IntVar[] latest      = new IntVar[N];
        final IntVar[] arrival     = new IntVar[N];
        final IntVar[] dist        = new IntVar[N+1];

        for (int i = 0; i < N; i++) {
            earliest[i] = new IntervalIntVarImpl("earliest", 0, HORIZON, model);
            latest[i]   = new IntervalIntVarImpl("latest",   0, HORIZON, model);
            arrival[i]  = new IntervalIntVarImpl("arrival",  0, HORIZON, model);
            dist[i]     = new IntervalIntVarImpl("dist",     0, HORIZON, model);
        }

        // going back to depot also incurs a cost
        dist[N]     = new IntervalIntVarImpl("dist",     0, HORIZON, model);

        int [] E = new int[nbNodes];
        int [] L = new int[nbNodes];
        for (int i = 0; i < nbNodes; i++) {
            E[i] = timeWindows[i].earliest;
            L[i] = timeWindows[i].latest;
        }

        model.allDifferent(x).post();
        x[0].eq(DEPOT).post();
        arrival[0].eq(0).post();

        for (int i = 0; i < nbNodes; i++) {
            model.element(earliest[i],E, x[i]).post();
            model.element(latest[i],  L, x[i]).post();
            arrival[i].le(latest[i]).post();
            arrival[i].ge(earliest[i]).post();
        }

        for (int i = 0; i < nbNodes-1; i++) {
            model.element(dist[i], distances,x[i],0,x[i+1],0);
            (arrival[i].add(dist[i])).le(arrival[i+1]).post();
        }

        // back to depot
        model.element(dist[N], distances, x[N-1], 0, x[0], 0);

        IntVar obj = new IntervalIntVarImpl("tot",0,HORIZON,model);
        model.sum(dist,"=", obj).post();

        obj.lt(totDistUB).post();

        model.setObjective(Model.MINIMIZE, obj);
        return new TsptwModel(model, x, dist, obj);
    }
}
