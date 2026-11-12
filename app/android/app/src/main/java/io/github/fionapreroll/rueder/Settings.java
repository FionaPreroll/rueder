package io.github.fionapreroll.rueder;

import android.content.Context;
import android.content.SharedPreferences;

/**
 * What the app keeps: the address of the reader's rueder, and whether the app
 * looks for new versions of itself at every start (off until the reader says so).
 */
final class Settings {
    private static final String FILE = "rueder";
    private static final String SERVER = "server";
    private static final String CHECK_UPDATES = "checkUpdates";

    private Settings() {}

    private static SharedPreferences prefs(Context context) {
        return context.getSharedPreferences(FILE, Context.MODE_PRIVATE);
    }

    /** The origin of the server, or null before the reader named one. */
    static String server(Context context) {
        return Address.origin(prefs(context).getString(SERVER, null));
    }

    /** Keeps the server; false if the address is no https address. */
    static boolean setServer(Context context, String typed) {
        String origin = Address.origin(typed);
        if (origin == null) return false;
        prefs(context).edit().putString(SERVER, origin).apply();
        return true;
    }

    static boolean checksUpdates(Context context) {
        return Updates.AVAILABLE && prefs(context).getBoolean(CHECK_UPDATES, false);
    }

    static void setChecksUpdates(Context context, boolean on) {
        prefs(context).edit().putBoolean(CHECK_UPDATES, on && Updates.AVAILABLE).apply();
    }
}
