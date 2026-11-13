package io.github.fionapreroll.rueder;

/**
 * Whether a press of back at the top of the app closes it: only a second one
 * within {@link #WITHIN_MS} of the first, which only says so. A press after
 * that counts as a first one again.
 */
final class BackPress {
    /** How soon the second press has to follow the first, in milliseconds. */
    static final long WITHIN_MS = 2000;

    private long first = Long.MIN_VALUE;

    /** True if the app should close at a press at `now` (milliseconds of a clock that only goes on). */
    boolean closesAt(long now) {
        if (first != Long.MIN_VALUE && now - first >= 0 && now - first <= WITHIN_MS) {
            first = Long.MIN_VALUE;
            return true;
        }
        first = now;
        return false;
    }
}
