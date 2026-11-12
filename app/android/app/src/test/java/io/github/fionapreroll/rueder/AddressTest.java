package io.github.fionapreroll.rueder;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class AddressTest {
    @Test
    public void takesTheOriginOfAnHttpsAddress() {
        assertEquals("https://reader.example", Address.origin(" https://Reader.Example/feeds/1?x#y "));
        assertEquals("https://reader.example:8443", Address.origin("https://reader.example:8443/"));
        assertEquals("https://reader.example", Address.origin("https://reader.example:443"));
    }

    @Test
    public void refusesEverythingElse() {
        String[] wrong = {
            null, "", "  ", "http://reader.example", "reader.example", "ftp://reader.example",
            "javascript:alert(1)", "file:///etc/passwd", "https://ada:secret@reader.example",
            "https://reader example", "https://", "https://reader.example\n.evil",
            "https://" + "a".repeat(Address.MAX) + ".example"
        };
        for (String address : wrong) assertNull(address, Address.origin(address));
    }

    @Test
    public void knowsWhatIsOnTheServer() {
        String server = "https://reader.example";
        assertTrue(Address.onServer("https://reader.example/api/tree", server));
        assertTrue(Address.onServer("https://READER.example/", server));
        assertFalse(Address.onServer("https://reader.example:8443/", server));
        assertFalse(Address.onServer("http://reader.example/", server));
        assertFalse(Address.onServer("https://reader.example.evil/", server));
        assertFalse(Address.onServer("https://evil/?https://reader.example", server));
        assertFalse(Address.onServer("https://reader.example/", null));
    }
}
