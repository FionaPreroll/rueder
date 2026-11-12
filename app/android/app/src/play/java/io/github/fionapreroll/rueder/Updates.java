package io.github.fionapreroll.rueder;

import android.app.Activity;
import java.util.function.Consumer;

/**
 * The build for the Play Store does not look for new versions of itself: the
 * store brings them. Nothing here reaches out, and the build has no permission
 * to install apps (compare src/github/).
 */
final class Updates {
    static final boolean AVAILABLE = false;

    private Updates() {}

    static void look(Activity activity, Consumer<Boolean> done) {
        done.accept(false);
    }
}
