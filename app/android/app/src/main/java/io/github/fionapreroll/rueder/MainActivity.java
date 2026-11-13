package io.github.fionapreroll.rueder;

import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import android.webkit.WebView;
import android.widget.Toast;
import androidx.activity.OnBackPressedCallback;
import com.getcapacitor.BridgeActivity;

/**
 * The app's only screen: the WebView with the start page (www/), which opens the
 * reader's rueder. The launcher's shortcut "Server and updates", and the menu of
 * rueder's page, come back to the start page to change them. If the reader agreed,
 * the app looks for a new version of itself at every start.
 *
 * Back goes back in the WebView: to the list or the article before in rueder's
 * page, which keeps them in its history, or from the start page to the server.
 * Where there is nothing to go back to, the first press says that a second one
 * closes the app, and only a second one soon after does.
 */
public class MainActivity extends BridgeActivity {
    /** The extra of the shortcut that asks for the start page. */
    static final String SETUP = "io.github.fionapreroll.rueder.SETUP";

    private ServerClient client;
    private boolean setup;
    private final BackPress back = new BackPress();
    private Toast backAgain;

    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(RuederPlugin.class);
        super.onCreate(savedInstanceState);
        client = new ServerClient(bridge);
        bridge.setWebViewClient(client);
        setup = asksForSetup(getIntent());
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override
            public void handleOnBackPressed() {
                goBack();
            }
        });
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

    private void goBack() {
        WebView web = bridge.getWebView();
        if (web.canGoBack()) {
            web.goBack();
            return;
        }
        if (back.closesAt(SystemClock.elapsedRealtime())) {
            if (backAgain != null) backAgain.cancel();
            finish();
            return;
        }
        backAgain = Toast.makeText(this, R.string.back_again, Toast.LENGTH_SHORT);
        backAgain.show();
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
