package io.github.fionapreroll.rueder;

import android.content.Intent;
import android.os.Bundle;
import com.getcapacitor.BridgeActivity;

/**
 * The app's only screen: the WebView with the start page (www/), which opens the
 * reader's rueder. The launcher's shortcut "Server and updates", and the menu of
 * rueder's page, come back to the start page to change them. If the reader agreed,
 * the app looks for a new version of itself at every start.
 */
public class MainActivity extends BridgeActivity {
    /** The extra of the shortcut that asks for the start page. */
    static final String SETUP = "io.github.fionapreroll.rueder.SETUP";

    private ServerClient client;
    private boolean setup;

    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(RuederPlugin.class);
        super.onCreate(savedInstanceState);
        client = new ServerClient(bridge);
        bridge.setWebViewClient(client);
        setup = asksForSetup(getIntent());
        if (savedInstanceState == null && Settings.checksUpdates(this)) {
            Updates.look(this, (newer) -> {});
        }
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        if (asksForSetup(intent)) {
            setup = true;
            bridge.getWebView().loadUrl(bridge.getLocalUrl() + "/index.html?setup");
        }
    }

    private static boolean asksForSetup(Intent intent) {
        return intent != null && intent.getBooleanExtra(SETUP, false);
    }

    ServerClient client() {
        return client;
    }

    /** Whether the start page was asked for rather than the server; asked once. */
    boolean takeSetup() {
        boolean asked = setup;
        setup = false;
        return asked;
    }
}
