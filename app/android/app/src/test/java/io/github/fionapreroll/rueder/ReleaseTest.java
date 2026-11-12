package io.github.fionapreroll.rueder;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class ReleaseTest {
    @Test
    public void readsTheVersionOfTheRelease() {
        Release release = Release.parse("{\"build\":42,\"commit\":\"0123abc\",\"size\":5242880}");
        assertNotNull(release);
        assertEquals(42, release.build);
        assertEquals("5.0", release.megabytes());
        assertTrue(release.newerThan(41));
        assertFalse(release.newerThan(42));
        assertFalse(release.newerThan(43));
    }

    @Test
    public void refusesWhatIsNoRelease() {
        String[] wrong = {
            null, "", "not json", "[]", "{}", "{\"build\":0,\"size\":1}",
            "{\"build\":1,\"size\":0}", "{\"build\":\"x\",\"size\":1}",
            "{\"build\":1,\"size\":" + (Release.MAX_APK + 1) + "}",
            "{\"build\":1,\"size\":1,\"commit\":\"<script>\"}",
            "{\"build\":1,\"size\":1,\"pad\":\"" + "x".repeat(Release.MAX_JSON) + "\"}"
        };
        for (String json : wrong) assertNull(json, Release.parse(json));
    }
}
