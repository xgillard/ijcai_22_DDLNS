package be.uclouvain.ingi.aia;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;

public final class TsptwParser {
    public static TsptwInstance fromFile(final String fname) throws IOException {
        try (BufferedReader reader = new BufferedReader(new FileReader(fname))) {
            int lc                   = 0;
            int nb_nodes             = 0;
            int[][] distances        = new int[0][0];
            TimeWindow[] timewindows = new TimeWindow[0];

            int twc = 0;
            String line;
            while ((line = reader.readLine()) != null) {
                // skip comment lines
                if (line.startsWith("#") || line.isEmpty()) {
                    continue;
                }

                // First line is the number of nodes
                if (lc == 0) {
                    nb_nodes   = Integer.parseInt(line.split("\\s+")[0]);
                    distances  = new int[nb_nodes][nb_nodes];
                    timewindows= new TimeWindow[nb_nodes];
                }
                // The next 'nb_nodes' lines represent the distances matrix
            else if (lc >= 1 && lc <=nb_nodes) {
                    int i = (lc - 1);
                    int j = 0;
                    for (String distance : line.split("\\s+")) {
                        float fdist     = Float.parseFloat(distance);
                        int   idist     = (int) Math.rint(TsptwInstance.PRECISION * fdist);
                        distances[i][j] = idist;
                        j += 1;
                    }
                }
                // Finally, the last 'nb_nodes' lines impose the time windows constraints
            else {
                    String[] tokens = line.split("\\s+");
                    double fearliest    = Double.parseDouble(tokens[0]);
                    double flatest      = Double.parseDouble(tokens[1]);

                    int iearliest      = (int) Math.rint(fearliest * TsptwInstance.PRECISION);
                    int ilatest        = (int) Math.rint(flatest   * TsptwInstance.PRECISION);

                    timewindows[twc++] = new TimeWindow(iearliest, ilatest);
                }

                lc += 1;
            }

            return new TsptwInstance(nb_nodes, distances, timewindows);
        }
    }
}
