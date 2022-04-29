package be.uclouvain.ingi.aia;

public final class TimeWindow {
    final int earliest;
    final int latest;

    public TimeWindow(final int earliest, final int latest) {
        this.earliest = earliest;
        this.latest   = latest;
    }

    public int getEarliest() {
        return earliest;
    }
    public int getLatest() {
        return latest;
    }
}
