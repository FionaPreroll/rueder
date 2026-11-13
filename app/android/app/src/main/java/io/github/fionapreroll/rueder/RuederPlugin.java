package io.github.fionapreroll.rueder;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

/**
 * What the start page (www/start.js) asks of the app: what it keeps, to keep a new
 * server or the choice about updates, to open the server and to look for a new
 * version, and to ask the server where its operator's legal texts are. Only the
 * start page gets an answer: the page of the server, which also runs in the
 * WebView, is refused, so nothing there can change where the app goes.
 */
@CapacitorPlugin(name = "Rueder")
public class RuederPlugin extends Plugin {
    /** How long the server may take to say where its legal texts are. */
    private static final int LEGAL_TIMEOUT_MS = 5000;
    /** The most of its answer that is read. */
    static final int MAX_WAYS = 16 * 1024;

    private boolean fromStartPage(PluginCall call) {
        MainActivity activity = (MainActivity) getActivity();
        if (activity.client() != null && activity.client().showsStartPage()) return true;
        call.reject("only the app's start page may ask this");
        return false;
    }

    @PluginMethod
    public void state(PluginCall call) {
        if (!fromStartPage(call)) return;
        MainActivity activity = (MainActivity) getActivity();
        JSObject state = new JSObject();
        state.put("server", Settings.server(getContext()));
        state.put("checkUpdates", Settings.checksUpdates(getContext()));
        state.put("updatesAvailable", Updates.AVAILABLE);
        state.put("version", BuildConfig.VERSION_NAME);
        state.put("build", BuildConfig.VERSION_CODE);
        state.put("setup", activity.takeSetup());
        call.resolve(state);
    }

    @PluginMethod
    public void save(PluginCall call) {
        if (!fromStartPage(call)) return;
        String server = call.getString("server");
        if (server != null && !Settings.setServer(getContext(), server)) {
            call.reject("not an https address");
            return;
        }
        Boolean checkUpdates = call.getBoolean("checkUpdates");
        if (checkUpdates != null) Settings.setChecksUpdates(getContext(), checkUpdates);
        call.resolve();
    }

    @PluginMethod
    public void open(PluginCall call) {
        if (!fromStartPage(call)) return;
        String server = Settings.server(getContext());
        if (server == null) {
            call.reject("no server yet");
            return;
        }
        MainActivity activity = (MainActivity) getActivity();
        activity.runOnUiThread(() -> activity.client().openServer(server));
        call.resolve();
    }

    /**
     * What the saved server says to anybody about logging in, with the
     * addresses of its operator's legal texts, as it came: the start page reads
     * it and keeps only whole web addresses. Nothing but the saved server is
     * asked, and nothing is sent to it.
     */
    @PluginMethod
    public void legal(PluginCall call) {
        if (!fromStartPage(call)) return;
        String server = Settings.server(getContext());
        if (server == null) {
            call.reject("no server yet");
            return;
        }
        new Thread(() -> {
            String body = ways(server);
            if (body == null) {
                call.reject("no answer");
                return;
            }
            JSObject result = new JSObject();
            result.put("body", body);
            call.resolve(result);
        }, "rueder-legal").start();
    }

    private static String ways(String server) {
        HttpURLConnection connection = null;
        try {
            connection = (HttpURLConnection) new URL(server + "/api/login/ways").openConnection();
            connection.setConnectTimeout(LEGAL_TIMEOUT_MS);
            connection.setReadTimeout(LEGAL_TIMEOUT_MS);
            connection.setInstanceFollowRedirects(false);
            connection.setUseCaches(false);
            if (connection.getResponseCode() != 200) return null;
            try (InputStream in = connection.getInputStream()) {
                ByteArrayOutputStream read = new ByteArrayOutputStream();
                byte[] buffer = new byte[1024];
                int n;
                while ((n = in.read(buffer)) != -1) {
                    read.write(buffer, 0, n);
                    if (read.size() > MAX_WAYS) return null;
                }
                return read.toString(StandardCharsets.UTF_8.name());
            }
        } catch (IOException | ClassCastException e) {
            return null;
        } finally {
            if (connection != null) connection.disconnect();
        }
    }

    @PluginMethod
    public void lookForUpdate(PluginCall call) {
        if (!fromStartPage(call)) return;
        Updates.look(getActivity(), (newer) -> {
            if (newer == null) {
                call.reject("no answer");
                return;
            }
            JSObject result = new JSObject();
            result.put("newer", newer);
            call.resolve(result);
        });
    }
}
