package dev.localmt.smoke;

public final class LocalmtNative {
    public static final int LANGUAGE_ENGLISH = 0;
    public static final int LANGUAGE_RUSSIAN = 1;
    public static final int LANGUAGE_THAI = 2;
    public static final int LANGUAGE_VIETNAMESE = 3;
    public static final int LANGUAGE_JAPANESE = 4;

    static {
        System.loadLibrary("localmt_jni_smoke");
    }

    private LocalmtNative() {
    }

    public static native String startupSummary();

    public static native int supportedLanguageCount();

    public static native String languageCode(int languageId);

    public static native void validateLanguagePair(int sourceLanguageId, int targetLanguageId);

    public static native String ggufModelPackSummary(String modelPackPath);

    public static native String trustedSummary(String modelPackPath);

    public static native void configureLlamaRuntime(String absoluteLibraryPath);

    public static native void configureOrtRuntime(String absoluteLibraryPath);

    public static GgufTranslator openGgufTranslator(String modelPackPath) {
        return new GgufTranslator(openGgufNative(modelPackPath));
    }

    public static Translator openTrustedTranslator(String modelPackPath) {
        return new Translator(openTrusted(modelPackPath));
    }

    public static String translateGguf(
            String modelPackPath,
            int sourceLanguageId,
            int targetLanguageId,
            String text) {
        try (GgufTranslator translator = openGgufTranslator(modelPackPath)) {
            return translator.translate(sourceLanguageId, targetLanguageId, text);
        }
    }

    public static String translateTrusted(
            String modelPackPath,
            int sourceLanguageId,
            int targetLanguageId,
            String text) {
        try (Translator translator = openTrustedTranslator(modelPackPath)) {
            return translator.translate(sourceLanguageId, targetLanguageId, text);
        }
    }

    private static native long openGgufNative(String modelPackPath);

    private static native long openTrusted(String modelPackPath);

    private static native String translateGgufNative(
            long nativeTranslatorHandle,
            int sourceLanguageId,
            int targetLanguageId,
            String text);

    private static native String translate(
            long nativeTranslatorHandle,
            int sourceLanguageId,
            int targetLanguageId,
            String text);

    private static native void closeGgufNative(long nativeTranslatorHandle);

    private static native void close(long nativeTranslatorHandle);

    public static final class GgufTranslator implements AutoCloseable {
        private long nativeHandle;

        private GgufTranslator(long nativeHandle) {
            if (nativeHandle == 0) {
                throw new IllegalStateException("localmt GGUF translator open returned a null handle");
            }
            this.nativeHandle = nativeHandle;
        }

        public String translate(int sourceLanguageId, int targetLanguageId, String text) {
            ensureOpen();
            LocalmtNative.validateLanguagePair(sourceLanguageId, targetLanguageId);
            return LocalmtNative.translateGgufNative(
                    nativeHandle, sourceLanguageId, targetLanguageId, text);
        }

        @Override
        public void close() {
            long handle = nativeHandle;
            nativeHandle = 0;
            if (handle != 0) {
                LocalmtNative.closeGgufNative(handle);
            }
        }

        private void ensureOpen() {
            if (nativeHandle == 0) {
                throw new IllegalStateException("localmt GGUF translator is closed");
            }
        }
    }

    public static final class Translator implements AutoCloseable {
        private long nativeHandle;

        private Translator(long nativeHandle) {
            if (nativeHandle == 0) {
                throw new IllegalStateException("localmt translator open returned a null handle");
            }
            this.nativeHandle = nativeHandle;
        }

        public String translate(int sourceLanguageId, int targetLanguageId, String text) {
            ensureOpen();
            LocalmtNative.validateLanguagePair(sourceLanguageId, targetLanguageId);
            return LocalmtNative.translate(nativeHandle, sourceLanguageId, targetLanguageId, text);
        }

        @Override
        public void close() {
            long handle = nativeHandle;
            nativeHandle = 0;
            if (handle != 0) {
                LocalmtNative.close(handle);
            }
        }

        private void ensureOpen() {
            if (nativeHandle == 0) {
                throw new IllegalStateException("localmt translator is closed");
            }
        }
    }
}
