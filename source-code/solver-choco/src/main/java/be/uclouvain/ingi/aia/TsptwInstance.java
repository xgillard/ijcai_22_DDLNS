package be.uclouvain.ingi.aia;

import java.util.Arrays;
import java.util.Comparator;

// The formulation is based on this example model from the choco documentation
// https://github.com/chocoteam/choco-solver/blob/master/examples/src/main/java/org/chocosolver/examples/integer/TSP.java

public class TsptwInstance {
    int          nbNodes;
    int[][]      distances;
    TimeWindow[] timeWindows;

    public static final double PRECISION = 1000.0;

    public TsptwInstance(final int nbNodes, final int[][] distances, final TimeWindow[] tw) {
        this.nbNodes    = nbNodes;
        this.distances  = distances;
        this.timeWindows= tw;
    }

    public TsptwInstance sort() {
        Integer [] perm = new Integer[nbNodes];
        for (int i = 0; i < nbNodes; i++) {
            perm[i] = i;
        }
        Arrays.sort(perm, new Comparator<Integer>() {
            @Override
            public int compare(Integer o1, Integer o2) {
                return timeWindows[o1].latest-timeWindows[o2].latest;
            }
        });

        int [][] distMatrix_ = new int[nbNodes][nbNodes];
        TimeWindow [] tw_ = new TimeWindow[nbNodes];

        for (int i = 0; i < nbNodes; i++) {
            tw_[i] = new TimeWindow(timeWindows[perm[i]].earliest,timeWindows[perm[i]].latest);
        }
        for (int i = 0; i < nbNodes; i++) {
            for (int j = 0; j < nbNodes; j++) {
                distMatrix_[i][j] = distances[perm[i]][perm[j]];
            }
        }
        return new TsptwInstance(nbNodes,distMatrix_,tw_);
    }

}
