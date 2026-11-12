package io.github.fionapreroll.rueder;

import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

/**
 * What the start page (www/start.js) asks of the app: what it keeps, to keep a new
 * server or the choice about updates, to open the server and to look for a new
 * version. Only the start page gets an answer: the page of the server, which also
 * runs in the WebView, is refused, so nothing there can change where the app goes.
 */
@CapacitorPlugin(name = "Rueder")
public class RuederPlugin extends Plugin {
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
