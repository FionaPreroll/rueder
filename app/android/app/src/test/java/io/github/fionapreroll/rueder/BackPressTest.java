package io.github.fionapreroll.rueder;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class BackPressTest {
    @Test
    public void closesOnlyOnASecondPressSoonAfterTheFirst() {
        BackPress back = new BackPress();
        assertFalse(back.closesAt(10_000));
        assertTrue(back.closesAt(10_000 + BackPress.WITHIN_MS));
    }

    @Test
    public void aLatePressStartsAgain() {
        BackPress back = new BackPress();
        assertFalse(back.closesAt(0));
        assertFalse(back.closesAt(BackPress.WITHIN_MS + 1));
        assertTrue(back.closesAt(BackPress.WITHIN_MS + 500));
    }

    @Test
    public void afterClosingTheNextPressIsAFirstOne() {
        BackPress back = new BackPress();
        back.closesAt(0);
        assertTrue(back.closesAt(100));
        assertFalse(back.closesAt(200));
    }

    @Test
    public void aClockThatWentBackDoesNotClose() {
        BackPress back = new BackPress();
        assertFalse(back.closesAt(5000));
        assertFalse(back.closesAt(4000));
    }
}
