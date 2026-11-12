package io.github.fionapreroll.rueder;

import java.net.URI;
import java.net.URISyntaxException;
import java.util.Locale;

/**
 * The address of a rueder as the app keeps it: the origin of an https page, as
 * rueder logs in only over a secure connection and runs at the top of its host.
 * The start page checks the same (www/address.js); this side trusts no page.
 */
final class Address {
    /** The longest address taken, in characters. */
    static final int MAX = 2000;

    private Address() {}

    /** The origin of an https address, such as "https://reader.example", or null. */
    static String origin(String typed) {
        if (typed == null) return null;
        String text = typed.trim();
        if (text.isEmpty() || text.length() > MAX) return null;
        for (int i = 0; i < text.length(); i++) {
            if (Character.isWhitespace(text.charAt(i)) || Character.isISOControl(text.charAt(i))) {
                return null;
            }
        }
        URI uri;
        try {
            uri = new URI(text);
        } catch (URISyntaxException e) {
            return null;
        }
        String scheme = uri.getScheme();
        String host = uri.getHost();
        if (scheme == null || !scheme.equalsIgnoreCase("https") || host == null || host.isEmpty()) {
            return null;
        }
        if (uri.getRawUserInfo() != null) return null;
        String origin = "https://" + host.toLowerCase(Locale.ROOT);
        return uri.getPort() == -1 || uri.getPort() == 443 ? origin : origin + ":" + uri.getPort();
    }

    /** Whether an address is on the server whose origin is given. */
    static boolean onServer(String url, String server) {
        if (url == null || server == null) return false;
        String origin = origin(url);
        return origin != null && origin.equals(server);
    }
}
