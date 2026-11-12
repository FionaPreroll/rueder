package io.github.fionapreroll.rueder;

import org.json.JSONException;
import org.json.JSONObject;

/**
 * The newest build of the app as its release describes it in version.json:
 * {"build": 123, "commit": "...", "size": 4567890}, which CI writes next to the
 * APK (.github/workflows/android.yml).
 */
final class Release {
    /** The largest version.json read, in bytes, and the largest APK taken. */
    static final int MAX_JSON = 4 * 1024;
    static final long MAX_APK = 200L * 1024 * 1024;

    final int build;
    final long size;
    final String commit;

    private Release(int build, long size, String commit) {
        this.build = build;
        this.size = size;
        this.commit = commit;
    }

    /** The release in a version.json, or null if it is none. */
    static Release parse(String json) {
        if (json == null || json.length() > MAX_JSON) return null;
        try {
            JSONObject object = new JSONObject(json);
            int build = object.getInt("build");
            long size = object.getLong("size");
            String commit = object.optString("commit", "");
            if (build <= 0 || size <= 0 || size > MAX_APK) return null;
            if (!commit.matches("[0-9a-f]{0,40}")) return null;
            return new Release(build, size, commit);
        } catch (JSONException e) {
            return null;
        }
    }

    /** Whether it is newer than the build that runs. */
    boolean newerThan(int installed) {
        return build > installed;
    }

    /**
     * Whether a downloaded APK is this release of this app: the package that is
     * installed, and the build version.json names. Android checks the signature
     * only when an APK updates the app that is installed; an APK of another
     * package would be installed as an app of its own.
     */
    boolean describes(String archivePackage, long archiveBuild, String ownPackage) {
        return archivePackage != null && archivePackage.equals(ownPackage) && archiveBuild == build;
    }

    /** Its size in megabytes, with one decimal, for the dialog. */
    String megabytes() {
        return String.format(java.util.Locale.ROOT, "%.1f", size / (1024.0 * 1024.0));
    }
}
