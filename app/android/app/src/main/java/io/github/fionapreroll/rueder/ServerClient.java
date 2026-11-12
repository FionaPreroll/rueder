package io.github.fionapreroll.rueder;

import android.graphics.Bitmap;
import android.net.Uri;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import com.getcapacitor.Bridge;
import com.getcapacitor.BridgeWebViewClient;

/**
 * Where the WebView may go. The start page is the app's own; the reader's rueder
 * loads in the WebView as in a browser; anything else, such as a link in an
 * article, opens in the browser. A rueder that does not answer sends the reader
 * back to the start page, which says so.
 */
class ServerClient extends BridgeWebViewClient {
    private final Bridge bridge;
    /** The page shown now, for the plugin to tell its own page from the server's. */
    private volatile String shown = "";
    /** The start page in the back history goes once the server's page is there. */
    private boolean forgetStart;

    ServerClient(Bridge bridge) {
        super(bridge);
        this.bridge = bridge;
    }

    /** Whether the start page is what is shown. */
    boolean showsStartPage() {
        return shown.startsWith(bridge.getLocalUrl() + "/");
    }

    /** Opens the server, and forgets the start page once it is there. */
    void openServer(String server) {
        forgetStart = true;
        bridge.getWebView().loadUrl(server + "/");
    }

    @Override
    public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
        if (Address.onServer(request.getUrl().toString(), Settings.server(view.getContext()))) {
            return false;
        }
        // The start page stays; anything else goes to the browser.
        return super.shouldOverrideUrlLoading(view, request);
    }

    @Override
    public void onPageStarted(WebView view, String url, Bitmap favicon) {
        shown = url == null ? "" : url;
        super.onPageStarted(view, url, favicon);
    }

    @Override
    public void onPageFinished(WebView view, String url) {
        super.onPageFinished(view, url);
        if (forgetStart && Address.onServer(url, Settings.server(view.getContext()))) {
            forgetStart = false;
            view.clearHistory();
        }
    }

    @Override
    public void onReceivedError(WebView view, WebResourceRequest request, WebResourceError error) {
        super.onReceivedError(view, request, error);
        String url = request.getUrl().toString();
        if (request.isForMainFrame() && Address.onServer(url, Settings.server(view.getContext()))) {
            view.loadUrl(bridge.getLocalUrl() + "/index.html?unreachable=" + Uri.encode(url));
        }
    }
}
