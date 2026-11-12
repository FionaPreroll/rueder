package io.github.fionapreroll.rueder;

import android.app.Activity;
import android.app.AlertDialog;
import android.app.DownloadManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageInfo;
import android.database.Cursor;
import android.net.ConnectivityManager;
import android.net.Uri;
import android.os.Environment;
import androidx.core.content.ContextCompat;
import androidx.core.content.FileProvider;
import androidx.core.content.pm.PackageInfoCompat;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.function.Consumer;

/**
 * New versions of the app, for the build that is installed from the release on
 * GitHub (the Play Store's build has none, see src/play/). If the reader agreed,
 * it reads version.json from the release at every start; a newer build is
 * offered in a dialog, downloaded by Android's download manager and handed to
 * Android's installer, which asks the reader and installs it only if it is
 * signed with the same key as the app that is installed.
 *
 * The download manager writes to the app's folder on the shared storage, where
 * other apps with the permission for storage can write too, up to Android 9.
 * So the APK is copied into the app's own cache first and handed on only if it
 * is this app in the build that version.json names: an APK of another package
 * would not be checked against the app's key, but installed as an app of its
 * own.
 */
final class Updates {
    static final boolean AVAILABLE = true;

    /** Where the release keeps the newest build: version.json and rueder.apk. */
    private static final String RELEASE = BuildConfig.UPDATES_URL;
    private static final String APK = "rueder.apk";
    private static final int TIMEOUT_MS = 10_000;

    private Updates() {}

    /**
     * Looks for a newer build. `done` hears true if there is one (and the dialog
     * offers it), false if not, null if the release did not answer.
     */
    static void look(Activity activity, Consumer<Boolean> done) {
        new Thread(() -> {
            Release release = fetch();
            activity.runOnUiThread(() -> {
                if (release == null) {
                    done.accept(null);
                    return;
                }
                boolean newer = release.newerThan(BuildConfig.VERSION_CODE);
                if (newer && !activity.isFinishing()) offer(activity, release);
                done.accept(newer);
            });
        }, "rueder-updates").start();
    }

    private static Release fetch() {
        HttpURLConnection connection = null;
        try {
            connection = (HttpURLConnection) new URL(RELEASE + "version.json").openConnection();
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setUseCaches(false);
            if (connection.getResponseCode() != 200) return null;
            try (InputStream in = connection.getInputStream()) {
                ByteArrayOutputStream read = new ByteArrayOutputStream();
                byte[] buffer = new byte[1024];
                int n;
                while ((n = in.read(buffer)) != -1) {
                    read.write(buffer, 0, n);
                    if (read.size() > Release.MAX_JSON) return null;
                }
                return Release.parse(read.toString(StandardCharsets.UTF_8.name()));
            }
        } catch (IOException | ClassCastException e) {
            return null;
        } finally {
            if (connection != null) connection.disconnect();
        }
    }

    private static void offer(Activity activity, Release release) {
        ConnectivityManager network = activity.getSystemService(ConnectivityManager.class);
        boolean metered = network != null && network.isActiveNetworkMetered();
        String text = activity.getString(R.string.update_text, release.build, release.megabytes());
        if (metered) text += "\n\n" + activity.getString(R.string.update_metered);
        new AlertDialog.Builder(activity)
            .setTitle(R.string.update_title)
            .setMessage(text)
            .setPositiveButton(R.string.update_download, (dialog, which) -> download(activity, release))
            .setNegativeButton(R.string.update_later, null)
            .show();
    }

    private static void download(Activity activity, Release release) {
        Context context = activity.getApplicationContext();
        File folder = context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS);
        if (folder == null) return;
        File file = new File(folder, APK);
        if (file.exists() && !file.delete()) return;
        DownloadManager downloads = context.getSystemService(DownloadManager.class);
        if (downloads == null) return;
        DownloadManager.Request request = new DownloadManager.Request(Uri.parse(RELEASE + APK))
            .setTitle(context.getString(R.string.update_title))
            .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE)
            .setDestinationUri(Uri.fromFile(file));
        long id = downloads.enqueue(request);
        BroadcastReceiver done = new BroadcastReceiver() {
            @Override
            public void onReceive(Context received, Intent intent) {
                if (intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1) != id) return;
                context.unregisterReceiver(this);
                if (!succeeded(downloads, id) || file.length() != release.size) return;
                new Thread(() -> {
                    File kept = takeIn(context, file, release);
                    if (kept != null) install(context, kept);
                }, "rueder-updates").start();
            }
        };
        ContextCompat.registerReceiver(
            context,
            done,
            new IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE),
            ContextCompat.RECEIVER_EXPORTED
        );
    }

    private static boolean succeeded(DownloadManager downloads, long id) {
        try (Cursor cursor = downloads.query(new DownloadManager.Query().setFilterById(id))) {
            if (cursor == null || !cursor.moveToFirst()) return false;
            int status = cursor.getInt(cursor.getColumnIndexOrThrow(DownloadManager.COLUMN_STATUS));
            return status == DownloadManager.STATUS_SUCCESSFUL;
        }
    }

    /**
     * The downloaded APK in the app's own cache, where no other app can change
     * it, or null if it is not this release of this app. The download goes.
     */
    private static File takeIn(Context context, File downloaded, Release release) {
        File folder = new File(context.getCacheDir(), "updates");
        File kept = new File(folder, APK);
        boolean copied = (folder.isDirectory() || folder.mkdirs()) && copy(downloaded, kept, release.size);
        downloaded.delete();
        if (!copied) {
            kept.delete();
            return null;
        }
        PackageInfo archive = context.getPackageManager().getPackageArchiveInfo(kept.getPath(), 0);
        if (
            archive == null ||
            !release.describes(archive.packageName, PackageInfoCompat.getLongVersionCode(archive), context.getPackageName())
        ) {
            kept.delete();
            return null;
        }
        return kept;
    }

    /** Copies exactly `size` bytes, or nothing. */
    private static boolean copy(File from, File to, long size) {
        try (InputStream in = new FileInputStream(from); OutputStream out = new FileOutputStream(to)) {
            byte[] buffer = new byte[64 * 1024];
            long total = 0;
            int n;
            while ((n = in.read(buffer)) != -1) {
                total += n;
                if (total > size) return false;
                out.write(buffer, 0, n);
            }
            return total == size;
        } catch (IOException e) {
            return false;
        }
    }

    /** Hands the APK to Android's installer, which asks the reader. */
    private static void install(Context context, File file) {
        Uri apk = FileProvider.getUriForFile(context, context.getPackageName() + ".fileprovider", file);
        Intent intent = new Intent(Intent.ACTION_VIEW)
            .setDataAndType(apk, "application/vnd.android.package-archive")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
        context.startActivity(intent);
    }
}
