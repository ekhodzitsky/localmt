package dev.localmt.smoke;

public final class LocalmtNative {
    static {
        System.loadLibrary("localmt_jni_smoke");
    }

    private LocalmtNative() {
    }

    public static native String startupSummary();

    public static native String trustedSummary(String modelPackPath);

    public static native void configureOrtRuntime(String absoluteLibraryPath);

    public static native String translateTrusted(
            String modelPackPath,
            int sourceLanguageId,
            int targetLanguageId,
            String text);
}
