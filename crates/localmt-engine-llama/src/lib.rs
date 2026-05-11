//! llama.cpp / GGUF adapter boundary for localmt.

#[cfg(feature = "llama-runtime")]
use core::ffi::{c_char, c_void};
use core::{fmt, num::NonZeroUsize};
#[cfg(feature = "llama-runtime")]
use std::ffi::{CString, OsString};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "llama-runtime")]
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(any(test, feature = "llama-runtime"))]
use localmt_core::NonEmptyText;
use localmt_core::{Language, TranslateRequest, Translation};
use localmt_engine::{TranslationError, TranslatorEngine};
use localmt_models::{GgufModelAssetPlan, ModelId};
use serde::Deserialize;

const DEFAULT_CONTEXT_TOKENS: usize = 2048;
const DEFAULT_CPU_THREADS: usize = 1;
const DEFAULT_MAX_OUTPUT_TOKENS: usize = 256;
const DEFAULT_TEMPERATURE: f32 = 0.0;
const DISABLED_RUNTIME: &str = "llama.cpp runtime is not enabled";
const HUNYUAN_ASSISTANT_TAG: &str = "<｜hy_Assistant｜>";
const HUNYUAN_BEGIN_OF_SENTENCE: &str = "<｜hy_begin▁of▁sentence｜>";
const HUNYUAN_USER_TAG: &str = "<｜hy_User｜>";
const LLAMA_CPP_DYLIB_PATH_ENV: &str = "LLAMA_CPP_DYLIB_PATH";
#[cfg(feature = "llama-runtime")]
const LLAMA_TOKEN_NULL: i32 = -1;
#[cfg(feature = "llama-runtime")]
const LLAMA_DEFAULT_SEED: u32 = 0xFFFF_FFFF;
#[cfg(feature = "llama-runtime")]
const TOKEN_PIECE_BUFFER_BYTES: usize = 128;
#[cfg(feature = "llama-runtime")]
const MAX_TOKEN_PIECE_BUFFER_BYTES: usize = 16 * 1024;
#[cfg(feature = "llama-runtime")]
static LLAMA_CPP_DYLIB_PATH_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();
#[cfg(feature = "llama-runtime")]
static LLAMA_BACKEND_REFCOUNT: Mutex<usize> = Mutex::new(0);

/// Verified llama.cpp model-load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaModelPlan {
    model_id: ModelId,
    model_path: PathBuf,
    chat_template_path: Option<PathBuf>,
    runtime_config_path: Option<PathBuf>,
}

impl LlamaModelPlan {
    /// { assets were built from a verified GGUF model pack }
    /// fn from_assets(assets: GgufModelAssetPlan) -> Result<Self, LlamaEngineError>
    /// { ret preserves root-qualified GGUF and optional llama metadata paths }
    pub fn from_assets(assets: GgufModelAssetPlan) -> Result<Self, LlamaEngineError> {
        Ok(Self {
            model_id: assets.model_id().clone(),
            model_path: assets.model_path().to_path_buf(),
            chat_template_path: assets.chat_template_path().map(Path::to_path_buf),
            runtime_config_path: assets.runtime_config_path().map(Path::to_path_buf),
        })
    }

    /// { true }
    /// fn model_id(&self) -> &ModelId
    /// { ret is the verified model-pack id }
    pub const fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    /// { true }
    /// fn model_path(&self) -> &Path
    /// { ret is the root-qualified GGUF model path }
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// { true }
    /// fn chat_template_path(&self) -> `Option<&Path>`
    /// { ret is Some only when a verified chat_template file was declared }
    pub fn chat_template_path(&self) -> Option<&Path> {
        self.chat_template_path.as_deref()
    }

    /// { true }
    /// fn runtime_config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when a verified llama_runtime_config file was declared }
    pub fn runtime_config_path(&self) -> Option<&Path> {
        self.runtime_config_path.as_deref()
    }

    /// { self may declare a runtime config path }
    /// fn parse_runtime_config(&self) -> Result<LlamaRuntimeConfig, LlamaEngineError>
    /// { ret is Ok with parsed config or defaults when no config path exists }
    pub fn parse_runtime_config(&self) -> Result<LlamaRuntimeConfig, LlamaEngineError> {
        match self.runtime_config_path() {
            Some(path) => LlamaRuntimeConfig::from_json_file(path),
            None => Ok(LlamaRuntimeConfig::default()),
        }
    }
}

/// llama.cpp runtime configuration owned by localmt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LlamaRuntimeConfig {
    context_tokens: NonZeroUsize,
    cpu_threads: NonZeroUsize,
    max_output_tokens: NonZeroUsize,
    temperature: f32,
}

impl LlamaRuntimeConfig {
    /// { contents is a JSON runtime-config document }
    /// fn from_json_str(contents: &str) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when optional numeric fields are valid }
    pub fn from_json_str(contents: &str) -> Result<Self, LlamaEngineError> {
        let raw = serde_json::from_str::<RawLlamaRuntimeConfig>(contents).map_err(|source| {
            LlamaEngineError::ParseRuntimeConfig {
                reason: source.to_string(),
            }
        })?;

        Self::from_raw(raw)
    }

    /// { path points to a readable JSON runtime-config file }
    /// fn from_json_file(path: &Path) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when file contents parse as LlamaRuntimeConfig }
    pub fn from_json_file(path: &Path) -> Result<Self, LlamaEngineError> {
        let contents =
            fs::read_to_string(path).map_err(|source| LlamaEngineError::ReadRuntimeConfig {
                path: path.to_path_buf(),
                reason: source.to_string(),
            })?;

        Self::from_json_str(&contents)
    }

    /// { true }
    /// fn context_tokens(&self) -> NonZeroUsize
    /// { ret is the configured llama context window in tokens }
    pub const fn context_tokens(&self) -> NonZeroUsize {
        self.context_tokens
    }

    /// { true }
    /// fn cpu_threads(&self) -> NonZeroUsize
    /// { ret is the configured CPU thread count }
    pub const fn cpu_threads(&self) -> NonZeroUsize {
        self.cpu_threads
    }

    /// { true }
    /// fn max_output_tokens(&self) -> NonZeroUsize
    /// { ret is the maximum number of generated llama tokens per request }
    pub const fn max_output_tokens(&self) -> NonZeroUsize {
        self.max_output_tokens
    }

    /// { true }
    /// fn temperature(&self) -> f32
    /// { ret is the configured sampling temperature }
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    fn from_raw(raw: RawLlamaRuntimeConfig) -> Result<Self, LlamaEngineError> {
        Ok(Self {
            context_tokens: nonzero_or_default(
                "context_tokens",
                raw.context_tokens,
                DEFAULT_CONTEXT_TOKENS,
            )?,
            cpu_threads: nonzero_or_default("cpu_threads", raw.cpu_threads, DEFAULT_CPU_THREADS)?,
            max_output_tokens: nonzero_or_default(
                "max_output_tokens",
                raw.max_output_tokens,
                DEFAULT_MAX_OUTPUT_TOKENS,
            )?,
            temperature: valid_temperature(raw.temperature.unwrap_or(DEFAULT_TEMPERATURE))?,
        })
    }
}

impl Default for LlamaRuntimeConfig {
    fn default() -> Self {
        Self {
            context_tokens: nonzero_default(DEFAULT_CONTEXT_TOKENS),
            cpu_threads: nonzero_default(DEFAULT_CPU_THREADS),
            max_output_tokens: nonzero_default(DEFAULT_MAX_OUTPUT_TOKENS),
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

/// Translation prompt text passed to the llama.cpp completion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaTranslationPrompt(String);

impl LlamaTranslationPrompt {
    /// { request has a valid language pair and non-empty text }
    /// fn from_request(request: &TranslateRequest) -> Self
    /// { ret is the stable HY-MT segment prompt for request.target() and request.text() }
    pub fn from_request(request: &TranslateRequest) -> Self {
        let target = language_label(request.pair().target());
        let instruction = format!(
            "Translate the following segment into {target}, without additional explanation.\n\n{}",
            request.text().as_str()
        );
        Self(format!(
            "{HUNYUAN_BEGIN_OF_SENTENCE}{HUNYUAN_USER_TAG}{instruction}{HUNYUAN_ASSISTANT_TAG}"
        ))
    }

    /// { true }
    /// fn as_str(&self) -> &str
    /// { ret is the prompt text }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// llama.cpp translator handle.
pub struct LlamaTranslator {
    #[cfg(feature = "llama-runtime")]
    model_context: Option<Mutex<LlamaModelContext>>,
    #[cfg(feature = "llama-runtime")]
    _backend: Option<LlamaBackendLease>,
    #[cfg(feature = "llama-runtime")]
    _native: Option<LlamaNativeLibrary>,
    #[cfg(feature = "llama-runtime")]
    runtime_config: LlamaRuntimeConfig,
}

impl fmt::Debug for LlamaTranslator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LlamaTranslator")
    }
}

impl LlamaTranslator {
    /// { plan was built from verified GGUF assets }
    /// fn load(plan: LlamaModelPlan) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when the llama runtime feature can load the model }
    pub fn load(plan: LlamaModelPlan) -> Result<Self, LlamaEngineError> {
        #[cfg(not(feature = "llama-runtime"))]
        {
            let _plan = plan;
            Err(LlamaEngineError::RuntimeDisabled)
        }

        #[cfg(feature = "llama-runtime")]
        {
            let runtime_config = plan.parse_runtime_config()?;
            let dylib_path = resolve_llama_cpp_dylib_path()?;
            let native = LlamaNativeLibrary::load(&dylib_path)?;
            let backend = LlamaBackendLease::acquire(&native);
            let model_context = LlamaModelContext::load(&native, &plan, runtime_config)?;
            Ok(Self {
                model_context: Some(Mutex::new(model_context)),
                _backend: Some(backend),
                _native: Some(native),
                runtime_config,
            })
        }
    }

    /// { true }
    /// fn disabled_for_tests() -> Self
    /// { ret is a translator handle that reports disabled runtime on translate }
    #[cfg(test)]
    pub(crate) fn disabled_for_tests() -> Self {
        Self {
            #[cfg(feature = "llama-runtime")]
            model_context: None,
            #[cfg(feature = "llama-runtime")]
            _backend: None,
            #[cfg(feature = "llama-runtime")]
            _native: None,
            #[cfg(feature = "llama-runtime")]
            runtime_config: LlamaRuntimeConfig::default(),
        }
    }

    /// { self may own a loaded llama.cpp model/context and request is valid }
    /// fn translate_with_llama(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is Ok only when llama.cpp generates non-empty UTF-8 text }
    #[cfg(feature = "llama-runtime")]
    fn translate_with_llama(
        &self,
        request: &TranslateRequest,
    ) -> Result<Translation, TranslationError> {
        let Some(model_context) = self.model_context.as_ref() else {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp model context is not loaded".to_owned(),
            ));
        };

        let prompt = LlamaTranslationPrompt::from_request(request);
        let mut model_context = lock_model_context(model_context);
        model_context.translate_prompt(&prompt, self.runtime_config)
    }
}

impl TranslatorEngine for LlamaTranslator {
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        #[cfg(not(feature = "llama-runtime"))]
        {
            let _request = request;
            Err(TranslationError::EngineUnavailable(
                DISABLED_RUNTIME.to_owned(),
            ))
        }

        #[cfg(feature = "llama-runtime")]
        {
            self.translate_with_llama(request)
        }
    }
}

/// Error returned by the llama.cpp / GGUF adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlamaEngineError {
    /// Runtime feature or native backend is not available in this build.
    RuntimeDisabled,
    /// Dynamic llama.cpp loading requires an explicit dylib path.
    MissingLlamaDylibPath,
    /// Dynamic llama.cpp dylib path is not usable.
    InvalidLlamaDylibPath {
        /// Path configured through LLAMA_CPP_DYLIB_PATH.
        path: PathBuf,
        /// Stable validation reason.
        reason: String,
    },
    /// llama.cpp dynamic library failed to load.
    LoadNativeLibrary {
        /// Path configured through LLAMA_CPP_DYLIB_PATH or FFI.
        path: PathBuf,
        /// Stable load error reason.
        reason: String,
    },
    /// llama.cpp dynamic library is missing a required C API symbol.
    MissingNativeSymbol {
        /// Path configured through LLAMA_CPP_DYLIB_PATH or FFI.
        path: PathBuf,
        /// Missing required symbol.
        symbol: &'static str,
        /// Stable lookup error reason.
        reason: String,
    },
    /// GGUF model path cannot be passed to llama.cpp.
    InvalidModelPath {
        /// Verified model path.
        path: PathBuf,
        /// Stable path conversion reason.
        reason: &'static str,
    },
    /// llama.cpp failed to load the verified GGUF model.
    LoadModel {
        /// Verified model path.
        path: PathBuf,
        /// Stable load reason.
        reason: &'static str,
    },
    /// llama.cpp failed to create a context from the loaded model.
    CreateContext {
        /// Verified model path.
        path: PathBuf,
        /// Stable context creation reason.
        reason: &'static str,
    },
    /// Runtime-config JSON could not be read.
    ReadRuntimeConfig {
        /// Config path.
        path: PathBuf,
        /// Stable read error reason.
        reason: String,
    },
    /// Runtime-config JSON could not be parsed.
    ParseRuntimeConfig {
        /// Stable parse error reason.
        reason: String,
    },
    /// Runtime-config numeric field is invalid.
    InvalidRuntimeConfig {
        /// Config field name.
        field: &'static str,
        /// Stable validation reason.
        reason: &'static str,
    },
}

impl fmt::Display for LlamaEngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeDisabled => formatter.write_str(DISABLED_RUNTIME),
            Self::MissingLlamaDylibPath => {
                write!(
                    formatter,
                    "{LLAMA_CPP_DYLIB_PATH_ENV} must point to a llama.cpp dynamic library"
                )
            }
            Self::InvalidLlamaDylibPath { path, reason } => write!(
                formatter,
                "{LLAMA_CPP_DYLIB_PATH_ENV} {} is invalid: {reason}",
                path.display()
            ),
            Self::LoadNativeLibrary { path, reason } => {
                write!(
                    formatter,
                    "failed to load llama.cpp library {}: {reason}",
                    path.display()
                )
            }
            Self::MissingNativeSymbol {
                path,
                symbol,
                reason,
            } => write!(
                formatter,
                "llama.cpp library {} is missing symbol {symbol}: {reason}",
                path.display()
            ),
            Self::InvalidModelPath { path, reason } => {
                write!(
                    formatter,
                    "llama.cpp model path {} is invalid: {reason}",
                    path.display()
                )
            }
            Self::LoadModel { path, reason } => {
                write!(
                    formatter,
                    "llama.cpp failed to load model {}: {reason}",
                    path.display()
                )
            }
            Self::CreateContext { path, reason } => {
                write!(
                    formatter,
                    "llama.cpp failed to create context for {}: {reason}",
                    path.display()
                )
            }
            Self::ReadRuntimeConfig { path, reason } => {
                write!(
                    formatter,
                    "failed to read llama runtime config {}: {reason}",
                    path.display()
                )
            }
            Self::ParseRuntimeConfig { reason } => {
                write!(formatter, "failed to parse llama runtime config: {reason}")
            }
            Self::InvalidRuntimeConfig { field, reason } => {
                write!(formatter, "invalid llama runtime config {field}: {reason}")
            }
        }
    }
}

impl std::error::Error for LlamaEngineError {}

/// { path is a candidate llama.cpp dynamic library path }
/// `fn configure_llama_cpp_dylib_path(path: impl Into<PathBuf>) -> Result<(), LlamaEngineError>`
/// { ret is Ok only when future llama.cpp loads may use path as the explicit dylib }
#[cfg(feature = "llama-runtime")]
pub fn configure_llama_cpp_dylib_path(path: impl Into<PathBuf>) -> Result<(), LlamaEngineError> {
    let path = validate_llama_cpp_dylib_path(path.into())?;
    let _native = LlamaNativeLibrary::load(&path)?;

    match LLAMA_CPP_DYLIB_PATH_OVERRIDE.set(path) {
        Ok(()) => Ok(()),
        Err(path) => match LLAMA_CPP_DYLIB_PATH_OVERRIDE.get() {
            Some(existing) if existing == &path => Ok(()),
            Some(existing) => Err(LlamaEngineError::InvalidLlamaDylibPath {
                path,
                reason: format!("runtime path already configured as {}", existing.display()),
            }),
            None => Err(LlamaEngineError::InvalidLlamaDylibPath {
                path,
                reason: "runtime path could not be configured".to_owned(),
            }),
        },
    }
}

#[cfg(feature = "llama-runtime")]
struct LlamaNativeLibrary {
    backend_init: LlamaBackendInit,
    backend_free: LlamaBackendFree,
    model_default_params: LlamaModelDefaultParams,
    model_load_from_file: LlamaModelLoadFromFile,
    model_free: LlamaModelFree,
    context_default_params: LlamaContextDefaultParams,
    init_from_model: LlamaInitFromModel,
    context_free: LlamaFree,
    model_get_vocab: LlamaModelGetVocab,
    get_memory: LlamaGetMemory,
    memory_clear: LlamaMemoryClear,
    model_has_encoder: LlamaModelHasEncoder,
    model_decoder_start_token: LlamaModelDecoderStartToken,
    vocab_bos: LlamaVocabBos,
    tokenize: LlamaTokenize,
    batch_get_one: LlamaBatchGetOne,
    encode: LlamaEncode,
    decode: LlamaDecode,
    sampler_chain_default_params: LlamaSamplerChainDefaultParams,
    sampler_chain_init: LlamaSamplerChainInit,
    sampler_chain_add: LlamaSamplerChainAdd,
    sampler_init_greedy: LlamaSamplerInitGreedy,
    sampler_init_temp: LlamaSamplerInitTemp,
    sampler_init_dist: LlamaSamplerInitDist,
    sampler_accept: LlamaSamplerAccept,
    sampler_sample: LlamaSamplerSample,
    sampler_free: LlamaSamplerFree,
    vocab_is_eog: LlamaVocabIsEog,
    token_to_piece: LlamaTokenToPiece,
    _library: libloading::Library,
}

#[cfg(feature = "llama-runtime")]
impl LlamaNativeLibrary {
    /// { path points to an absolute existing file }
    /// fn load(path: &Path) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when path loads and exports the minimal llama.cpp model-load API }
    fn load(path: &Path) -> Result<Self, LlamaEngineError> {
        let library = unsafe {
            // SAFETY: the caller already validated that path is an absolute
            // file. Loading may run platform loader code, so this boundary only
            // performs preflight and never calls loaded functions.
            libloading::Library::new(path)
        }
        .map_err(|source| LlamaEngineError::LoadNativeLibrary {
            path: path.to_path_buf(),
            reason: source.to_string(),
        })?;

        let backend_init = load_symbol(&library, path, "llama_backend_init")?;
        let backend_free = load_symbol(&library, path, "llama_backend_free")?;
        let model_default_params = load_symbol(&library, path, "llama_model_default_params")?;
        let model_load_from_file = load_symbol(&library, path, "llama_model_load_from_file")?;
        let model_free = load_symbol(&library, path, "llama_model_free")?;
        let context_default_params = load_symbol(&library, path, "llama_context_default_params")?;
        let init_from_model = load_symbol(&library, path, "llama_init_from_model")?;
        let context_free = load_symbol(&library, path, "llama_free")?;
        let model_get_vocab = load_symbol(&library, path, "llama_model_get_vocab")?;
        let get_memory = load_symbol(&library, path, "llama_get_memory")?;
        let memory_clear = load_symbol(&library, path, "llama_memory_clear")?;
        let model_has_encoder = load_symbol(&library, path, "llama_model_has_encoder")?;
        let model_decoder_start_token =
            load_symbol(&library, path, "llama_model_decoder_start_token")?;
        let vocab_bos = load_symbol(&library, path, "llama_vocab_bos")?;
        let tokenize = load_symbol(&library, path, "llama_tokenize")?;
        let batch_get_one = load_symbol(&library, path, "llama_batch_get_one")?;
        let encode = load_symbol(&library, path, "llama_encode")?;
        let decode = load_symbol(&library, path, "llama_decode")?;
        let sampler_chain_default_params =
            load_symbol(&library, path, "llama_sampler_chain_default_params")?;
        let sampler_chain_init = load_symbol(&library, path, "llama_sampler_chain_init")?;
        let sampler_chain_add = load_symbol(&library, path, "llama_sampler_chain_add")?;
        let sampler_init_greedy = load_symbol(&library, path, "llama_sampler_init_greedy")?;
        let sampler_init_temp = load_symbol(&library, path, "llama_sampler_init_temp")?;
        let sampler_init_dist = load_symbol(&library, path, "llama_sampler_init_dist")?;
        let sampler_accept = load_symbol(&library, path, "llama_sampler_accept")?;
        let sampler_sample = load_symbol(&library, path, "llama_sampler_sample")?;
        let sampler_free = load_symbol(&library, path, "llama_sampler_free")?;
        let vocab_is_eog = load_symbol(&library, path, "llama_vocab_is_eog")?;
        let token_to_piece = load_symbol(&library, path, "llama_token_to_piece")?;

        Ok(Self {
            backend_init,
            backend_free,
            model_default_params,
            model_load_from_file,
            model_free,
            context_default_params,
            init_from_model,
            context_free,
            model_get_vocab,
            get_memory,
            memory_clear,
            model_has_encoder,
            model_decoder_start_token,
            vocab_bos,
            tokenize,
            batch_get_one,
            encode,
            decode,
            sampler_chain_default_params,
            sampler_chain_init,
            sampler_chain_add,
            sampler_init_greedy,
            sampler_init_temp,
            sampler_init_dist,
            sampler_accept,
            sampler_sample,
            sampler_free,
            vocab_is_eog,
            token_to_piece,
            _library: library,
        })
    }
}

#[cfg(feature = "llama-runtime")]
type LlamaBackendInit = unsafe extern "C" fn();
#[cfg(feature = "llama-runtime")]
type LlamaBackendFree = unsafe extern "C" fn();
#[cfg(feature = "llama-runtime")]
type LlamaModelDefaultParams = unsafe extern "C" fn() -> LlamaModelParams;
#[cfg(feature = "llama-runtime")]
type LlamaModelLoadFromFile =
    unsafe extern "C" fn(*const c_char, LlamaModelParams) -> *mut LlamaModel;
#[cfg(feature = "llama-runtime")]
type LlamaModelFree = unsafe extern "C" fn(*mut LlamaModel);
#[cfg(feature = "llama-runtime")]
type LlamaContextDefaultParams = unsafe extern "C" fn() -> LlamaContextParams;
#[cfg(feature = "llama-runtime")]
type LlamaInitFromModel =
    unsafe extern "C" fn(*mut LlamaModel, LlamaContextParams) -> *mut LlamaContext;
#[cfg(feature = "llama-runtime")]
type LlamaFree = unsafe extern "C" fn(*mut LlamaContext);
#[cfg(feature = "llama-runtime")]
type LlamaModelGetVocab = unsafe extern "C" fn(*const LlamaModel) -> *const LlamaVocab;
#[cfg(feature = "llama-runtime")]
type LlamaGetMemory = unsafe extern "C" fn(*const LlamaContext) -> *mut LlamaMemory;
#[cfg(feature = "llama-runtime")]
type LlamaMemoryClear = unsafe extern "C" fn(*mut LlamaMemory, bool);
#[cfg(feature = "llama-runtime")]
type LlamaModelHasEncoder = unsafe extern "C" fn(*const LlamaModel) -> bool;
#[cfg(feature = "llama-runtime")]
type LlamaModelDecoderStartToken = unsafe extern "C" fn(*const LlamaModel) -> LlamaToken;
#[cfg(feature = "llama-runtime")]
type LlamaVocabBos = unsafe extern "C" fn(*const LlamaVocab) -> LlamaToken;
#[cfg(feature = "llama-runtime")]
type LlamaTokenize = unsafe extern "C" fn(
    *const LlamaVocab,
    *const c_char,
    i32,
    *mut LlamaToken,
    i32,
    bool,
    bool,
) -> i32;
#[cfg(feature = "llama-runtime")]
type LlamaBatchGetOne = unsafe extern "C" fn(*mut LlamaToken, i32) -> LlamaBatch;
#[cfg(feature = "llama-runtime")]
type LlamaEncode = unsafe extern "C" fn(*mut LlamaContext, LlamaBatch) -> i32;
#[cfg(feature = "llama-runtime")]
type LlamaDecode = unsafe extern "C" fn(*mut LlamaContext, LlamaBatch) -> i32;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerChainDefaultParams = unsafe extern "C" fn() -> LlamaSamplerChainParams;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerChainInit = unsafe extern "C" fn(LlamaSamplerChainParams) -> *mut LlamaSampler;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerChainAdd = unsafe extern "C" fn(*mut LlamaSampler, *mut LlamaSampler);
#[cfg(feature = "llama-runtime")]
type LlamaSamplerInitGreedy = unsafe extern "C" fn() -> *mut LlamaSampler;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerInitTemp = unsafe extern "C" fn(f32) -> *mut LlamaSampler;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerInitDist = unsafe extern "C" fn(u32) -> *mut LlamaSampler;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerAccept = unsafe extern "C" fn(*mut LlamaSampler, LlamaToken);
#[cfg(feature = "llama-runtime")]
type LlamaSamplerSample =
    unsafe extern "C" fn(*mut LlamaSampler, *mut LlamaContext, i32) -> LlamaToken;
#[cfg(feature = "llama-runtime")]
type LlamaSamplerFree = unsafe extern "C" fn(*mut LlamaSampler);
#[cfg(feature = "llama-runtime")]
type LlamaVocabIsEog = unsafe extern "C" fn(*const LlamaVocab, LlamaToken) -> bool;
#[cfg(feature = "llama-runtime")]
type LlamaTokenToPiece =
    unsafe extern "C" fn(*const LlamaVocab, LlamaToken, *mut c_char, i32, i32, bool) -> i32;

#[cfg(feature = "llama-runtime")]
type LlamaToken = i32;
#[cfg(feature = "llama-runtime")]
type LlamaPos = i32;
#[cfg(feature = "llama-runtime")]
type LlamaSeqId = i32;

#[cfg(feature = "llama-runtime")]
#[repr(C)]
struct LlamaModel {
    _private: [u8; 0],
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
struct LlamaContext {
    _private: [u8; 0],
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
struct LlamaVocab {
    _private: [u8; 0],
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
struct LlamaMemory {
    _private: [u8; 0],
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
struct LlamaSampler {
    _private: [u8; 0],
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LlamaBatch {
    n_tokens: i32,
    token: *mut LlamaToken,
    embd: *mut f32,
    pos: *mut LlamaPos,
    n_seq_id: *mut i32,
    seq_id: *mut *mut LlamaSeqId,
    logits: *mut i8,
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LlamaSamplerChainParams {
    no_perf: bool,
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LlamaModelParams {
    devices: *mut *mut c_void,
    tensor_buft_overrides: *const c_void,
    n_gpu_layers: i32,
    split_mode: i32,
    main_gpu: i32,
    tensor_split: *const f32,
    progress_callback: Option<unsafe extern "C" fn(f32, *mut c_void) -> bool>,
    progress_callback_user_data: *mut c_void,
    kv_overrides: *const c_void,
    vocab_only: bool,
    use_mmap: bool,
    use_direct_io: bool,
    use_mlock: bool,
    check_tensors: bool,
    use_extra_bufts: bool,
    no_host: bool,
    no_alloc: bool,
}

#[cfg(all(test, feature = "llama-runtime"))]
impl LlamaModelParams {
    const fn zeroed_for_tests() -> Self {
        Self {
            devices: core::ptr::null_mut(),
            tensor_buft_overrides: core::ptr::null(),
            n_gpu_layers: 0,
            split_mode: 0,
            main_gpu: 0,
            tensor_split: core::ptr::null(),
            progress_callback: None,
            progress_callback_user_data: core::ptr::null_mut(),
            kv_overrides: core::ptr::null(),
            vocab_only: false,
            use_mmap: false,
            use_direct_io: false,
            use_mlock: false,
            check_tensors: false,
            use_extra_bufts: false,
            no_host: false,
            no_alloc: false,
        }
    }
}

#[cfg(feature = "llama-runtime")]
#[repr(C)]
#[derive(Clone, Copy)]
struct LlamaContextParams {
    n_ctx: u32,
    n_batch: u32,
    n_ubatch: u32,
    n_seq_max: u32,
    n_threads: i32,
    n_threads_batch: i32,
    rope_scaling_type: i32,
    pooling_type: i32,
    attention_type: i32,
    flash_attn_type: i32,
    rope_freq_base: f32,
    rope_freq_scale: f32,
    yarn_ext_factor: f32,
    yarn_attn_factor: f32,
    yarn_beta_fast: f32,
    yarn_beta_slow: f32,
    yarn_orig_ctx: u32,
    defrag_thold: f32,
    cb_eval: *mut c_void,
    cb_eval_user_data: *mut c_void,
    type_k: i32,
    type_v: i32,
    abort_callback: *mut c_void,
    abort_callback_data: *mut c_void,
    embeddings: bool,
    offload_kqv: bool,
    no_perf: bool,
    op_offload: bool,
    swa_full: bool,
    kv_unified: bool,
    samplers: *mut c_void,
    n_samplers: usize,
}

#[cfg(all(test, feature = "llama-runtime"))]
impl LlamaContextParams {
    const fn zeroed_for_tests() -> Self {
        Self {
            n_ctx: 0,
            n_batch: 0,
            n_ubatch: 0,
            n_seq_max: 0,
            n_threads: 0,
            n_threads_batch: 0,
            rope_scaling_type: 0,
            pooling_type: 0,
            attention_type: 0,
            flash_attn_type: 0,
            rope_freq_base: 0.0,
            rope_freq_scale: 0.0,
            yarn_ext_factor: 0.0,
            yarn_attn_factor: 0.0,
            yarn_beta_fast: 0.0,
            yarn_beta_slow: 0.0,
            yarn_orig_ctx: 0,
            defrag_thold: 0.0,
            cb_eval: core::ptr::null_mut(),
            cb_eval_user_data: core::ptr::null_mut(),
            type_k: 0,
            type_v: 0,
            abort_callback: core::ptr::null_mut(),
            abort_callback_data: core::ptr::null_mut(),
            embeddings: false,
            offload_kqv: false,
            no_perf: false,
            op_offload: false,
            swa_full: false,
            kv_unified: false,
            samplers: core::ptr::null_mut(),
            n_samplers: 0,
        }
    }
}

#[cfg(feature = "llama-runtime")]
struct LlamaBackendLease {
    backend_free: LlamaBackendFree,
}

#[cfg(feature = "llama-runtime")]
impl LlamaBackendLease {
    /// { native has resolved llama backend lifecycle symbols }
    /// fn acquire(native: &LlamaNativeLibrary) -> Self
    /// { ret owns one backend reference until dropped }
    fn acquire(native: &LlamaNativeLibrary) -> Self {
        let mut refcount = lock_backend_refcount();
        if *refcount == 0 {
            unsafe {
                // SAFETY: llama_backend_init is a process-level initializer.
                // The refcount serializes calls and keeps init/free balanced.
                (native.backend_init)();
            }
        }
        *refcount = refcount.saturating_add(1);

        Self {
            backend_free: native.backend_free,
        }
    }
}

#[cfg(feature = "llama-runtime")]
impl Drop for LlamaBackendLease {
    fn drop(&mut self) {
        let mut refcount = lock_backend_refcount();
        if *refcount == 0 {
            return;
        }

        *refcount -= 1;
        if *refcount == 0 {
            unsafe {
                // SAFETY: the final lease releases the process-level backend
                // after all model/context handles in this translator dropped.
                (self.backend_free)();
            }
        }
    }
}

#[cfg(feature = "llama-runtime")]
struct LlamaCpuDeviceList {
    devices: Box<[*mut c_void; 1]>,
}

#[cfg(feature = "llama-runtime")]
impl LlamaCpuDeviceList {
    /// { true }
    /// fn new() -> Self
    /// { ret is a stable empty llama.cpp device list for CPU-only model loading }
    fn new() -> Self {
        Self {
            devices: Box::new([core::ptr::null_mut()]),
        }
    }

    /// { true }
    /// fn as_mut_ptr(&mut self) -> *mut *mut c_void
    /// { ret points to a NULL-terminated empty device list }
    fn as_mut_ptr(&mut self) -> *mut *mut c_void {
        self.devices.as_mut_ptr()
    }
}

#[cfg(feature = "llama-runtime")]
struct LlamaModelContext {
    context: *mut LlamaContext,
    model: *mut LlamaModel,
    vocab: *const LlamaVocab,
    _cpu_devices: LlamaCpuDeviceList,
    context_free: LlamaFree,
    model_free: LlamaModelFree,
    get_memory: LlamaGetMemory,
    memory_clear: LlamaMemoryClear,
    model_has_encoder: LlamaModelHasEncoder,
    model_decoder_start_token: LlamaModelDecoderStartToken,
    vocab_bos: LlamaVocabBos,
    tokenize: LlamaTokenize,
    batch_get_one: LlamaBatchGetOne,
    encode: LlamaEncode,
    decode: LlamaDecode,
    sampler_chain_default_params: LlamaSamplerChainDefaultParams,
    sampler_chain_init: LlamaSamplerChainInit,
    sampler_chain_add: LlamaSamplerChainAdd,
    sampler_init_greedy: LlamaSamplerInitGreedy,
    sampler_init_temp: LlamaSamplerInitTemp,
    sampler_init_dist: LlamaSamplerInitDist,
    sampler_accept: LlamaSamplerAccept,
    sampler_sample: LlamaSamplerSample,
    sampler_free: LlamaSamplerFree,
    vocab_is_eog: LlamaVocabIsEog,
    token_to_piece: LlamaTokenToPiece,
}

#[cfg(feature = "llama-runtime")]
impl LlamaModelContext {
    /// { native has loaded llama model/context symbols and plan points to a verified GGUF model }
    /// fn load(native: &LlamaNativeLibrary, plan: &LlamaModelPlan, config: LlamaRuntimeConfig) -> Result<Self, LlamaEngineError>
    /// { ret owns one loaded model and context until dropped }
    fn load(
        native: &LlamaNativeLibrary,
        plan: &LlamaModelPlan,
        config: LlamaRuntimeConfig,
    ) -> Result<Self, LlamaEngineError> {
        let model_path = plan.model_path();
        let model_path_c = c_model_path(model_path)?;
        let mut model_params = unsafe {
            // SAFETY: this calls the resolved llama.cpp factory and receives a
            // by-value params struct matching the pinned C ABI boundary.
            (native.model_default_params)()
        };
        let mut cpu_devices = LlamaCpuDeviceList::new();
        apply_cpu_only_model_params(&mut model_params, &mut cpu_devices);

        let mut context_params = unsafe {
            // SAFETY: this calls the resolved llama.cpp factory and receives a
            // by-value params struct matching the pinned C ABI boundary.
            (native.context_default_params)()
        };
        apply_runtime_config_to_context_params(&mut context_params, config)?;

        let model = unsafe {
            // SAFETY: model_path_c is NUL-terminated and model_params came from
            // the same library that consumes it.
            (native.model_load_from_file)(model_path_c.as_ptr(), model_params)
        };
        if model.is_null() {
            return Err(LlamaEngineError::LoadModel {
                path: model_path.to_path_buf(),
                reason: "llama_model_load_from_file returned null",
            });
        }

        let context = unsafe {
            // SAFETY: model is a live llama_model pointer and context_params
            // came from the same library that consumes it.
            (native.init_from_model)(model, context_params)
        };
        if context.is_null() {
            unsafe {
                // SAFETY: model was returned by this library and context
                // creation failed, so the model must be released here.
                (native.model_free)(model);
            }
            return Err(LlamaEngineError::CreateContext {
                path: model_path.to_path_buf(),
                reason: "llama_init_from_model returned null",
            });
        }
        let vocab = unsafe {
            // SAFETY: model is a live llama_model pointer created by the
            // library that exports the vocabulary accessor.
            (native.model_get_vocab)(model)
        };
        if vocab.is_null() {
            unsafe {
                // SAFETY: context/model were returned by this library and
                // cannot be used without a vocabulary.
                (native.context_free)(context);
                (native.model_free)(model);
            }
            return Err(LlamaEngineError::LoadModel {
                path: model_path.to_path_buf(),
                reason: "llama_model_get_vocab returned null",
            });
        }

        Ok(Self {
            context,
            model,
            vocab,
            _cpu_devices: cpu_devices,
            context_free: native.context_free,
            model_free: native.model_free,
            get_memory: native.get_memory,
            memory_clear: native.memory_clear,
            model_has_encoder: native.model_has_encoder,
            model_decoder_start_token: native.model_decoder_start_token,
            vocab_bos: native.vocab_bos,
            tokenize: native.tokenize,
            batch_get_one: native.batch_get_one,
            encode: native.encode,
            decode: native.decode,
            sampler_chain_default_params: native.sampler_chain_default_params,
            sampler_chain_init: native.sampler_chain_init,
            sampler_chain_add: native.sampler_chain_add,
            sampler_init_greedy: native.sampler_init_greedy,
            sampler_init_temp: native.sampler_init_temp,
            sampler_init_dist: native.sampler_init_dist,
            sampler_accept: native.sampler_accept,
            sampler_sample: native.sampler_sample,
            sampler_free: native.sampler_free,
            vocab_is_eog: native.vocab_is_eog,
            token_to_piece: native.token_to_piece,
        })
    }

    /// { prompt is a bounded localmt translation prompt and config is validated }
    /// fn translate_prompt(&mut self, prompt: &LlamaTranslationPrompt, config: LlamaRuntimeConfig) -> Result<Translation, TranslationError>
    /// { ret is Ok only when llama.cpp decodes a non-empty UTF-8 completion }
    fn translate_prompt(
        &mut self,
        prompt: &LlamaTranslationPrompt,
        config: LlamaRuntimeConfig,
    ) -> Result<Translation, TranslationError> {
        self.clear_memory()?;
        let mut current_tokens = self.tokenize_prompt(prompt)?;
        validate_generation_window(current_tokens.len(), config)?;

        let mut sampler = LlamaSamplerHandle::new(self, config)?;
        let mut batch = self.batch_for_tokens(&mut current_tokens)?;
        let mut output = Vec::new();
        let mut piece_buffer = vec![0_u8; TOKEN_PIECE_BUFFER_BYTES];

        if self.model_has_encoder() {
            self.encode_batch(batch)?;
            let decoder_start_token = self.decoder_start_token();
            current_tokens.clear();
            current_tokens.push(decoder_start_token);
            batch = self.batch_for_tokens(&mut current_tokens)?;
        }

        for _ in 0..config.max_output_tokens().get() {
            self.decode_batch(batch)?;

            let next_token = sampler.sample_next(self);
            sampler.accept(next_token);
            if self.is_eog(next_token) {
                break;
            }

            self.append_token_piece(next_token, &mut output, &mut piece_buffer)?;
            current_tokens.clear();
            current_tokens.push(next_token);
            batch = self.batch_for_tokens(&mut current_tokens)?;
        }

        translation_from_llama_bytes(output)
    }

    /// { self.context is a live llama context }
    /// fn clear_memory(&mut self) -> Result<(), TranslationError>
    /// { llama memory for the next request is empty when ret is Ok }
    fn clear_memory(&mut self) -> Result<(), TranslationError> {
        let memory = unsafe {
            // SAFETY: context is owned by this LlamaModelContext and remains
            // live for the duration of the call.
            (self.get_memory)(self.context)
        };
        if memory.is_null() {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp context has no memory".to_owned(),
            ));
        }

        unsafe {
            // SAFETY: memory came from the live context and data=true clears
            // both KV metadata and backing buffers between independent calls.
            (self.memory_clear)(memory, true);
        }
        Ok(())
    }

    /// { prompt is valid UTF-8 text }
    /// fn tokenize_prompt(&self, prompt: &LlamaTranslationPrompt) -> Result<Vec<LlamaToken>, TranslationError>
    /// { ret contains at least one prompt token when llama.cpp tokenization succeeds }
    fn tokenize_prompt(
        &self,
        prompt: &LlamaTranslationPrompt,
    ) -> Result<Vec<LlamaToken>, TranslationError> {
        let bytes = prompt.as_str().as_bytes();
        let text_len = i32::try_from(bytes.len()).map_err(|_error| {
            TranslationError::EngineUnavailable(
                "llama.cpp prompt is too large to tokenize".to_owned(),
            )
        })?;

        let needed = unsafe {
            // SAFETY: bytes points to valid memory for text_len bytes. The
            // first pass asks llama.cpp for the required token capacity.
            (self.tokenize)(
                self.vocab,
                bytes.as_ptr().cast::<c_char>(),
                text_len,
                core::ptr::null_mut(),
                0,
                true,
                true,
            )
        };
        let capacity = token_capacity_from_llama_result(needed)?;
        let mut tokens = vec![0; capacity];
        let max_tokens = i32::try_from(tokens.len()).map_err(|_error| {
            TranslationError::EngineUnavailable(
                "llama.cpp token buffer exceeds ABI limits".to_owned(),
            )
        })?;

        let written = unsafe {
            // SAFETY: tokens owns max_tokens writable llama_token slots and
            // bytes remains alive for the duration of tokenization.
            (self.tokenize)(
                self.vocab,
                bytes.as_ptr().cast::<c_char>(),
                text_len,
                tokens.as_mut_ptr(),
                max_tokens,
                true,
                true,
            )
        };
        if written < 0 {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp token buffer was too small".to_owned(),
            ));
        }
        if written == 0 {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp tokenized the prompt to zero tokens".to_owned(),
            ));
        }

        let written = usize::try_from(written).map_err(|_error| {
            TranslationError::EngineUnavailable(
                "llama.cpp returned an invalid token count".to_owned(),
            )
        })?;
        tokens.truncate(written);
        Ok(tokens)
    }

    /// { tokens is non-empty and stored for at least the next llama call }
    /// fn batch_for_tokens(&self, tokens: &mut [LlamaToken]) -> Result<LlamaBatch, TranslationError>
    /// { ret is a llama batch view over tokens }
    fn batch_for_tokens(&self, tokens: &mut [LlamaToken]) -> Result<LlamaBatch, TranslationError> {
        if tokens.is_empty() {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp cannot decode an empty token batch".to_owned(),
            ));
        }
        let token_count = i32::try_from(tokens.len()).map_err(|_error| {
            TranslationError::EngineUnavailable(
                "llama.cpp token batch exceeds ABI limits".to_owned(),
            )
        })?;

        let batch = unsafe {
            // SAFETY: tokens is a live mutable slice and llama_batch_get_one
            // returns a by-value view consumed immediately by encode/decode.
            (self.batch_get_one)(tokens.as_mut_ptr(), token_count)
        };
        Ok(batch)
    }

    /// { batch references live token storage }
    /// fn encode_batch(&mut self, batch: LlamaBatch) -> Result<(), TranslationError>
    /// { ret is Ok only when llama_encode accepts the prompt batch }
    fn encode_batch(&mut self, batch: LlamaBatch) -> Result<(), TranslationError> {
        let status = unsafe {
            // SAFETY: context is live and batch was built from live token
            // storage for this call.
            (self.encode)(self.context, batch)
        };
        if status == 0 {
            return Ok(());
        }

        Err(TranslationError::EngineUnavailable(format!(
            "llama.cpp encode failed with status {status}"
        )))
    }

    /// { batch references live token storage }
    /// fn decode_batch(&mut self, batch: LlamaBatch) -> Result<(), TranslationError>
    /// { ret is Ok only when llama_decode accepts the batch }
    fn decode_batch(&mut self, batch: LlamaBatch) -> Result<(), TranslationError> {
        let status = unsafe {
            // SAFETY: context is live and batch was built from live token
            // storage for this call.
            (self.decode)(self.context, batch)
        };
        if status == 0 {
            return Ok(());
        }

        Err(TranslationError::EngineUnavailable(format!(
            "llama.cpp decode failed with status {status}"
        )))
    }

    /// { self.model is a live llama model }
    /// fn model_has_encoder(&self) -> bool
    /// { ret mirrors llama_model_has_encoder for the loaded model }
    fn model_has_encoder(&self) -> bool {
        unsafe {
            // SAFETY: model is owned by this context and remains live.
            (self.model_has_encoder)(self.model)
        }
    }

    /// { self.model and self.vocab are live }
    /// fn decoder_start_token(&self) -> LlamaToken
    /// { ret is the model decoder start token or BOS fallback }
    fn decoder_start_token(&self) -> LlamaToken {
        let token = unsafe {
            // SAFETY: model is owned by this context and remains live.
            (self.model_decoder_start_token)(self.model)
        };
        if token != LLAMA_TOKEN_NULL {
            return token;
        }

        unsafe {
            // SAFETY: vocab belongs to the live model.
            (self.vocab_bos)(self.vocab)
        }
    }

    /// { token came from llama_sampler_sample for this vocabulary }
    /// fn is_eog(&self, token: LlamaToken) -> bool
    /// { ret is true only for llama.cpp end-of-generation tokens }
    fn is_eog(&self, token: LlamaToken) -> bool {
        unsafe {
            // SAFETY: vocab belongs to the live model and token is a llama
            // token id returned by the same runtime.
            (self.vocab_is_eog)(self.vocab, token)
        }
    }

    /// { token is a non-EOG token from this vocabulary }
    /// fn append_token_piece(&self, token: LlamaToken, output: &mut Vec<u8>, buffer: &mut Vec<u8>) -> Result<(), TranslationError>
    /// { output is extended with the rendered llama token bytes when ret is Ok }
    fn append_token_piece(
        &self,
        token: LlamaToken,
        output: &mut Vec<u8>,
        buffer: &mut Vec<u8>,
    ) -> Result<(), TranslationError> {
        loop {
            let length = i32::try_from(buffer.len()).map_err(|_error| {
                TranslationError::EngineUnavailable(
                    "llama.cpp token piece buffer exceeds ABI limits".to_owned(),
                )
            })?;
            let written = unsafe {
                // SAFETY: buffer is writable for length bytes and vocab belongs
                // to the live model.
                (self.token_to_piece)(
                    self.vocab,
                    token,
                    buffer.as_mut_ptr().cast::<c_char>(),
                    length,
                    0,
                    true,
                )
            };
            if written >= 0 {
                let written = usize::try_from(written).map_err(|_error| {
                    TranslationError::EngineUnavailable(
                        "llama.cpp returned an invalid token text length".to_owned(),
                    )
                })?;
                output.extend_from_slice(&buffer[..written]);
                return Ok(());
            }

            let capacity = token_piece_retry_capacity(written, buffer.len())?;
            buffer.resize(capacity, 0);
        }
    }
}

#[cfg(feature = "llama-runtime")]
impl Drop for LlamaModelContext {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: context and model were returned by the same llama.cpp
            // library and are released in the required context-before-model
            // order.
            (self.context_free)(self.context);
            (self.model_free)(self.model);
        }
    }
}

#[cfg(feature = "llama-runtime")]
struct LlamaSamplerHandle {
    sampler: *mut LlamaSampler,
    sampler_accept: LlamaSamplerAccept,
    sampler_sample: LlamaSamplerSample,
    sampler_free: LlamaSamplerFree,
}

#[cfg(feature = "llama-runtime")]
impl LlamaSamplerHandle {
    /// { context owns resolved sampler symbols and config was validated }
    /// fn new(context: &LlamaModelContext, config: LlamaRuntimeConfig) -> Result<Self, TranslationError>
    /// { ret owns a llama sampler chain until dropped }
    fn new(
        context: &LlamaModelContext,
        config: LlamaRuntimeConfig,
    ) -> Result<Self, TranslationError> {
        let mut params = unsafe {
            // SAFETY: this calls the resolved llama.cpp factory and receives a
            // by-value params struct matching the pinned C ABI boundary.
            (context.sampler_chain_default_params)()
        };
        params.no_perf = true;

        let sampler = unsafe {
            // SAFETY: params came from the same library that consumes it.
            (context.sampler_chain_init)(params)
        };
        if sampler.is_null() {
            return Err(TranslationError::EngineUnavailable(
                "llama.cpp failed to create sampler chain".to_owned(),
            ));
        }

        let handle = Self {
            sampler,
            sampler_accept: context.sampler_accept,
            sampler_sample: context.sampler_sample,
            sampler_free: context.sampler_free,
        };
        handle.add_sampling_stage(context, config)?;
        Ok(handle)
    }

    /// { self.sampler is a live sampler chain }
    /// fn add_sampling_stage(&self, context: &LlamaModelContext, config: LlamaRuntimeConfig) -> Result<(), TranslationError>
    /// { self.sampler ends with a token-selecting sampler when ret is Ok }
    fn add_sampling_stage(
        &self,
        context: &LlamaModelContext,
        config: LlamaRuntimeConfig,
    ) -> Result<(), TranslationError> {
        if config.temperature() == 0.0 {
            let greedy = unsafe {
                // SAFETY: initializes an owned greedy sampler from the same
                // library as the chain.
                (context.sampler_init_greedy)()
            };
            return self.add_component(context, greedy, "greedy");
        }

        let temperature = unsafe {
            // SAFETY: initializes an owned temperature sampler from the same
            // library as the chain.
            (context.sampler_init_temp)(config.temperature())
        };
        self.add_component(context, temperature, "temperature")?;

        let distribution = unsafe {
            // SAFETY: initializes an owned distribution sampler from the same
            // library as the chain.
            (context.sampler_init_dist)(LLAMA_DEFAULT_SEED)
        };
        self.add_component(context, distribution, "distribution")
    }

    /// { self.sampler is live and component is either null or owned by llama.cpp }
    /// fn add_component(&self, context: &LlamaModelContext, component: *mut LlamaSampler, name: &'static str) -> Result<(), TranslationError>
    /// { component ownership has moved into self.sampler when ret is Ok }
    fn add_component(
        &self,
        context: &LlamaModelContext,
        component: *mut LlamaSampler,
        name: &'static str,
    ) -> Result<(), TranslationError> {
        if component.is_null() {
            return Err(TranslationError::EngineUnavailable(format!(
                "llama.cpp failed to create {name} sampler"
            )));
        }

        unsafe {
            // SAFETY: sampler and component were created by the same library;
            // the chain takes ownership of the component.
            (context.sampler_chain_add)(self.sampler, component);
        }
        Ok(())
    }

    /// { self.sampler and context.context are live after a successful decode }
    /// fn sample_next(&mut self, context: &mut LlamaModelContext) -> LlamaToken
    /// { ret is the token selected from the latest logits }
    fn sample_next(&mut self, context: &mut LlamaModelContext) -> LlamaToken {
        unsafe {
            // SAFETY: sampler and context are live; -1 asks llama.cpp to sample
            // from the latest token logits.
            (self.sampler_sample)(self.sampler, context.context, -1)
        }
    }

    /// { token came from this sampler }
    /// fn accept(&mut self, token: LlamaToken)
    /// { self sampler state includes token }
    fn accept(&mut self, token: LlamaToken) {
        unsafe {
            // SAFETY: sampler is live and token came from the same sampler.
            (self.sampler_accept)(self.sampler, token);
        }
    }
}

#[cfg(feature = "llama-runtime")]
impl Drop for LlamaSamplerHandle {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: sampler is an owned chain returned by llama.cpp and is
            // released once here.
            (self.sampler_free)(self.sampler);
        }
    }
}

/// { bytes are raw token pieces emitted by llama.cpp }
/// fn translation_from_llama_bytes(bytes: Vec<u8>) -> Result<Translation, TranslationError>
/// { ret is Ok only when bytes are non-empty valid UTF-8 after text validation }
#[cfg(any(test, feature = "llama-runtime"))]
fn translation_from_llama_bytes(bytes: Vec<u8>) -> Result<Translation, TranslationError> {
    let output = String::from_utf8(bytes).map_err(|_error| {
        TranslationError::EngineUnavailable("llama.cpp produced non-UTF-8 output".to_owned())
    })?;
    let text = NonEmptyText::new(output).map_err(TranslationError::InvalidOutput)?;
    Ok(Translation::new(text))
}

/// { true }
/// fn lock_backend_refcount() -> MutexGuard<'static, usize>
/// { ret is the global llama backend refcount lock, recovering from poison }
#[cfg(feature = "llama-runtime")]
fn lock_backend_refcount() -> MutexGuard<'static, usize> {
    match LLAMA_BACKEND_REFCOUNT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// { context is the loaded llama model/context mutex }
/// fn lock_model_context(context: &Mutex<LlamaModelContext>) -> MutexGuard<'_, LlamaModelContext>
/// { ret is the model/context lock, recovering from poison }
#[cfg(feature = "llama-runtime")]
fn lock_model_context(context: &Mutex<LlamaModelContext>) -> MutexGuard<'_, LlamaModelContext> {
    match context.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// { library is a loaded dynamic library and symbol names a required function }
/// fn load_symbol<T: Copy>(library: &libloading::Library, path: &Path, symbol: &'static str) -> Result<T, LlamaEngineError>
/// { ret is Ok only when the symbol is present and copied while library stays owned }
#[cfg(feature = "llama-runtime")]
fn load_symbol<T: Copy>(
    library: &libloading::Library,
    path: &Path,
    symbol: &'static str,
) -> Result<T, LlamaEngineError> {
    let symbol = unsafe {
        // SAFETY: this only resolves a typed function pointer and copies it;
        // the owning Library remains stored in LlamaNativeLibrary.
        library.get::<T>(symbol.as_bytes()).map_err(|source| {
            LlamaEngineError::MissingNativeSymbol {
                path: path.to_path_buf(),
                symbol,
                reason: source.to_string(),
            }
        })?
    };
    Ok(*symbol)
}

/// { path is the verified GGUF model path }
/// fn c_model_path(path: &Path) -> Result<CString, LlamaEngineError>
/// { ret is a NUL-terminated path accepted by llama.cpp }
#[cfg(feature = "llama-runtime")]
fn c_model_path(path: &Path) -> Result<CString, LlamaEngineError> {
    let Some(path) = path.to_str() else {
        return Err(LlamaEngineError::InvalidModelPath {
            path: path.to_path_buf(),
            reason: "path must be UTF-8",
        });
    };

    CString::new(path).map_err(|_error| LlamaEngineError::InvalidModelPath {
        path: PathBuf::from(path),
        reason: "path contains an interior NUL byte",
    })
}

/// { params came from llama_model_default_params and devices is stable for the model lifetime }
/// fn apply_cpu_only_model_params(params: &mut LlamaModelParams, devices: &mut LlamaCpuDeviceList)
/// { params cannot select GPU/device-backed model layers }
#[cfg(feature = "llama-runtime")]
fn apply_cpu_only_model_params(params: &mut LlamaModelParams, devices: &mut LlamaCpuDeviceList) {
    params.devices = devices.as_mut_ptr();
    params.n_gpu_layers = 0;
}

/// { params came from llama_context_default_params and config is validated localmt config }
/// fn apply_runtime_config_to_context_params(params: &mut LlamaContextParams, config: LlamaRuntimeConfig) -> Result<(), LlamaEngineError>
/// { params carries localmt-owned context/thread overrides when ret is Ok }
#[cfg(feature = "llama-runtime")]
fn apply_runtime_config_to_context_params(
    params: &mut LlamaContextParams,
    config: LlamaRuntimeConfig,
) -> Result<(), LlamaEngineError> {
    params.n_ctx = nonzero_to_u32("context_tokens", config.context_tokens())?;
    params.n_batch = params.n_ctx;
    if params.n_ubatch == 0 || params.n_ubatch > params.n_batch {
        params.n_ubatch = params.n_batch;
    }
    params.n_threads = nonzero_to_i32("cpu_threads", config.cpu_threads())?;
    params.n_threads_batch = params.n_threads;
    params.offload_kqv = false;
    params.no_perf = true;
    params.op_offload = false;
    Ok(())
}

#[derive(Deserialize)]
struct RawLlamaRuntimeConfig {
    context_tokens: Option<usize>,
    cpu_threads: Option<usize>,
    max_output_tokens: Option<usize>,
    temperature: Option<f32>,
}

/// { result came from llama_tokenize first pass }
/// fn token_capacity_from_llama_result(result: i32) -> Result<usize, TranslationError>
/// { ret is Ok only when llama.cpp reported a positive token capacity }
#[cfg(feature = "llama-runtime")]
fn token_capacity_from_llama_result(result: i32) -> Result<usize, TranslationError> {
    let capacity = if result < 0 {
        result.checked_neg().ok_or_else(|| {
            TranslationError::EngineUnavailable(
                "llama.cpp returned an invalid token capacity".to_owned(),
            )
        })?
    } else {
        result
    };
    if capacity == 0 {
        return Err(TranslationError::EngineUnavailable(
            "llama.cpp reported zero prompt tokens".to_owned(),
        ));
    }

    usize::try_from(capacity).map_err(|_error| {
        TranslationError::EngineUnavailable(
            "llama.cpp returned an invalid token capacity".to_owned(),
        )
    })
}

/// { result is a negative llama_token_to_piece return and current_capacity is positive }
/// fn token_piece_retry_capacity(result: i32, current_capacity: usize) -> Result<usize, TranslationError>
/// { ret is the next bounded token-piece buffer capacity }
#[cfg(feature = "llama-runtime")]
fn token_piece_retry_capacity(
    result: i32,
    current_capacity: usize,
) -> Result<usize, TranslationError> {
    let reported = result.checked_neg().ok_or_else(|| {
        TranslationError::EngineUnavailable(
            "llama.cpp returned an invalid token text length".to_owned(),
        )
    })?;
    let reported = usize::try_from(reported).map_err(|_error| {
        TranslationError::EngineUnavailable(
            "llama.cpp returned an invalid token text length".to_owned(),
        )
    })?;
    let doubled = current_capacity.checked_mul(2).ok_or_else(|| {
        TranslationError::EngineUnavailable(
            "llama.cpp token text buffer resize overflowed".to_owned(),
        )
    })?;
    let next = reported.max(doubled);
    if next <= MAX_TOKEN_PIECE_BUFFER_BYTES {
        return Ok(next);
    }

    Err(TranslationError::EngineUnavailable(
        "llama.cpp token text exceeds localmt buffer limit".to_owned(),
    ))
}

/// { prompt_tokens is the tokenized prompt length and config is validated }
/// fn validate_generation_window(prompt_tokens: usize, config: LlamaRuntimeConfig) -> Result<(), TranslationError>
/// { ret is Ok only when prompt plus generation budget fits configured context }
#[cfg(feature = "llama-runtime")]
fn validate_generation_window(
    prompt_tokens: usize,
    config: LlamaRuntimeConfig,
) -> Result<(), TranslationError> {
    let required_tokens = prompt_tokens
        .checked_add(config.max_output_tokens().get())
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            TranslationError::EngineUnavailable("llama.cpp generation window overflowed".to_owned())
        })?;
    if required_tokens <= config.context_tokens().get() {
        return Ok(());
    }

    Err(TranslationError::EngineUnavailable(
        "llama.cpp prompt and output budget exceed configured context window".to_owned(),
    ))
}

fn nonzero_or_default(
    field: &'static str,
    value: Option<usize>,
    default: usize,
) -> Result<NonZeroUsize, LlamaEngineError> {
    match value {
        Some(value) => NonZeroUsize::new(value).ok_or(LlamaEngineError::InvalidRuntimeConfig {
            field,
            reason: "must be greater than zero",
        }),
        None => Ok(nonzero_default(default)),
    }
}

fn nonzero_default(value: usize) -> NonZeroUsize {
    if let Some(value) = NonZeroUsize::new(value) {
        return value;
    }

    NonZeroUsize::MIN
}

/// { value is non-zero }
/// fn nonzero_to_u32(field: &'static str, value: NonZeroUsize) -> Result<u32, LlamaEngineError>
/// { ret is Ok only when value fits the llama.cpp u32 ABI field }
#[cfg(feature = "llama-runtime")]
fn nonzero_to_u32(field: &'static str, value: NonZeroUsize) -> Result<u32, LlamaEngineError> {
    u32::try_from(value.get()).map_err(|_error| LlamaEngineError::InvalidRuntimeConfig {
        field,
        reason: "must fit into u32",
    })
}

/// { value is non-zero }
/// fn nonzero_to_i32(field: &'static str, value: NonZeroUsize) -> Result<i32, LlamaEngineError>
/// { ret is Ok only when value fits the llama.cpp i32 ABI field }
#[cfg(feature = "llama-runtime")]
fn nonzero_to_i32(field: &'static str, value: NonZeroUsize) -> Result<i32, LlamaEngineError> {
    i32::try_from(value.get()).map_err(|_error| LlamaEngineError::InvalidRuntimeConfig {
        field,
        reason: "must fit into i32",
    })
}

fn valid_temperature(value: f32) -> Result<f32, LlamaEngineError> {
    if value.is_finite() && value >= 0.0 {
        return Ok(value);
    }

    Err(LlamaEngineError::InvalidRuntimeConfig {
        field: "temperature",
        reason: "must be finite and non-negative",
    })
}

const fn language_label(language: Language) -> &'static str {
    match language {
        Language::English => "English",
        Language::Russian => "Russian",
        Language::Thai => "Thai",
        Language::Vietnamese => "Vietnamese",
        Language::Japanese => "Japanese",
    }
}

/// { llama.cpp runtime path may be configured through FFI or LLAMA_CPP_DYLIB_PATH }
/// fn resolve_llama_cpp_dylib_path() -> Result<PathBuf, LlamaEngineError>
/// { ret is Ok only when an explicit configured or environment dylib path is available }
#[cfg(feature = "llama-runtime")]
fn resolve_llama_cpp_dylib_path() -> Result<PathBuf, LlamaEngineError> {
    if let Some(path) = LLAMA_CPP_DYLIB_PATH_OVERRIDE.get() {
        return Ok(path.clone());
    }

    resolve_llama_cpp_dylib_path_from_env_value(std::env::var_os(LLAMA_CPP_DYLIB_PATH_ENV))
}

/// { value is a raw LLAMA_CPP_DYLIB_PATH environment value }
/// fn resolve_llama_cpp_dylib_path_from_env_value(value: `Option<OsString>`) -> Result<PathBuf, LlamaEngineError>
/// { ret is Ok only when value is a non-empty absolute path to a file }
#[cfg(feature = "llama-runtime")]
fn resolve_llama_cpp_dylib_path_from_env_value(
    value: Option<OsString>,
) -> Result<PathBuf, LlamaEngineError> {
    let Some(value) = value.filter(|candidate| !candidate.is_empty()) else {
        return Err(LlamaEngineError::MissingLlamaDylibPath);
    };
    let path = PathBuf::from(value);

    validate_llama_cpp_dylib_path(path)
}

/// { path is a candidate llama.cpp dynamic library path }
/// fn validate_llama_cpp_dylib_path(path: PathBuf) -> Result<PathBuf, LlamaEngineError>
/// { ret is Ok only when path is absolute and points to a file }
#[cfg(feature = "llama-runtime")]
fn validate_llama_cpp_dylib_path(path: PathBuf) -> Result<PathBuf, LlamaEngineError> {
    if !path.is_absolute() {
        return Err(LlamaEngineError::InvalidLlamaDylibPath {
            path,
            reason: "path must be absolute".to_owned(),
        });
    }
    if !path.is_file() {
        return Err(LlamaEngineError::InvalidLlamaDylibPath {
            path,
            reason: "path does not point to a file".to_owned(),
        });
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_core::{Language, NonEmptyText, TranslateRequest};
    use localmt_engine::TranslatorEngine;
    use localmt_models::{Discovered, GgufModelAssetPlan, ModelPack};

    use crate::{
        LlamaEngineError, LlamaModelPlan, LlamaRuntimeConfig, LlamaTranslationPrompt,
        LlamaTranslator,
    };

    const GGUF_MODEL_SHA256: &str =
        "a561ab462e9c80d55c2ca56fd846a30001aac1dea00c99d7e13e2b1320b88024";
    const CHAT_TEMPLATE_SHA256: &str =
        "90d69c78fb9ef942c8ce0a0d88a7455572ada06044bc1516473472664ddd013b";
    const LLAMA_RUNTIME_CONFIG_SHA256: &str =
        "4b825731b61ba8b6858381a904d807b1ae1f24bd163cc485d8082cd2198e0d0e";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn model_plan_preserves_verified_gguf_assets() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_gguf_pack()?;
        let assets = verified_gguf_assets(&root)?;

        let plan = LlamaModelPlan::from_assets(assets)?;

        assert_eq!(plan.model_id().as_str(), "hymt-1.25bit");
        assert_eq!(plan.model_path(), root.join("hymt.gguf"));
        assert_eq!(
            plan.chat_template_path(),
            Some(root.join("chat-template.jinja")).as_deref()
        );
        assert_eq!(
            plan.runtime_config_path(),
            Some(root.join("llama-runtime.json")).as_deref()
        );
        Ok(())
    }

    #[test]
    fn runtime_config_parses_optional_json_limits() -> Result<(), Box<dyn std::error::Error>> {
        let config = LlamaRuntimeConfig::from_json_str(
            r#"{"context_tokens": 2048, "cpu_threads": 4, "temperature": 0.0, "max_output_tokens": 96}"#,
        )?;

        assert_eq!(config.context_tokens().get(), 2048);
        assert_eq!(config.cpu_threads().get(), 4);
        assert_eq!(config.temperature(), 0.0);
        assert_eq!(config.max_output_tokens().get(), 96);
        Ok(())
    }

    #[test]
    fn runtime_config_rejects_invalid_numeric_limits() {
        let zero_threads =
            LlamaRuntimeConfig::from_json_str(r#"{"context_tokens": 2048, "cpu_threads": 0}"#);
        let negative_temperature = LlamaRuntimeConfig::from_json_str(r#"{"temperature": -0.1}"#);
        let zero_max_output_tokens =
            LlamaRuntimeConfig::from_json_str(r#"{"max_output_tokens": 0}"#);

        assert!(matches!(
            zero_threads,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "cpu_threads",
                ..
            })
        ));
        assert!(matches!(
            negative_temperature,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "temperature",
                ..
            })
        ));
        assert!(matches!(
            zero_max_output_tokens,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "max_output_tokens",
                ..
            })
        ));
    }

    #[test]
    fn llama_output_bytes_become_translation_text() {
        let translation = super::translation_from_llama_bytes("Привет офлайн".as_bytes().to_vec());

        assert!(matches!(
            translation,
            Ok(ref item) if item.text().as_str() == "Привет офлайн"
        ));
    }

    #[test]
    fn llama_output_bytes_reject_empty_text() {
        let translation = super::translation_from_llama_bytes(b" \n ".to_vec());

        assert!(matches!(
            translation,
            Err(localmt_engine::TranslationError::InvalidOutput(
                localmt_core::TextError::Empty
            ))
        ));
    }

    #[test]
    fn llama_output_bytes_reject_invalid_utf8() {
        let translation = super::translation_from_llama_bytes(vec![0xff]);

        assert!(matches!(
            translation,
            Err(localmt_engine::TranslationError::EngineUnavailable(reason))
                if reason == "llama.cpp produced non-UTF-8 output"
        ));
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn cpu_model_params_use_explicit_empty_device_list() {
        let mut params = super::LlamaModelParams::zeroed_for_tests();
        let mut devices = super::LlamaCpuDeviceList::new();
        params.n_gpu_layers = 99;

        super::apply_cpu_only_model_params(&mut params, &mut devices);

        assert_eq!(params.n_gpu_layers, 0);
        assert!(!params.devices.is_null());
        let first_device = unsafe {
            // SAFETY: apply_cpu_only_model_params points params.devices at the
            // stable one-element NULL-terminated list owned by devices.
            *params.devices
        };
        assert!(first_device.is_null());
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_config_applies_llama_context_limits() -> Result<(), Box<dyn std::error::Error>> {
        let config = LlamaRuntimeConfig::from_json_str(
            r#"{"context_tokens": 4096, "cpu_threads": 6, "temperature": 0.0}"#,
        )?;
        let mut params = super::LlamaContextParams::zeroed_for_tests();

        super::apply_runtime_config_to_context_params(&mut params, config)?;

        assert_eq!(params.n_ctx, 4096);
        assert_eq!(params.n_threads, 6);
        assert_eq!(params.n_threads_batch, 6);
        Ok(())
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_config_disables_device_offload_for_cpu_runtime()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = LlamaRuntimeConfig::default();
        let mut params = super::LlamaContextParams::zeroed_for_tests();
        params.offload_kqv = true;
        params.op_offload = true;

        super::apply_runtime_config_to_context_params(&mut params, config)?;

        assert!(!params.offload_kqv);
        assert!(!params.op_offload);
        Ok(())
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_config_rejects_threads_outside_llama_abi() {
        let config = LlamaRuntimeConfig::from_json_str(r#"{"cpu_threads": 2147483648}"#);

        let error = config.and_then(|config| {
            let mut params = super::LlamaContextParams::zeroed_for_tests();
            super::apply_runtime_config_to_context_params(&mut params, config)
        });

        assert!(matches!(
            error,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "cpu_threads",
                ..
            })
        ));
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn generation_window_rejects_prompt_and_output_budget_overflow()
    -> Result<(), Box<dyn std::error::Error>> {
        let config =
            LlamaRuntimeConfig::from_json_str(r#"{"context_tokens": 8, "max_output_tokens": 4}"#)?;

        let error = super::validate_generation_window(6, config);

        assert!(matches!(
            error,
            Err(localmt_engine::TranslationError::EngineUnavailable(reason))
                if reason == "llama.cpp prompt and output budget exceed configured context window"
        ));
        Ok(())
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn token_piece_retry_capacity_uses_llama_reported_size()
    -> Result<(), Box<dyn std::error::Error>> {
        let capacity = super::token_piece_retry_capacity(-300, 128)?;

        assert_eq!(capacity, 300);
        Ok(())
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn token_piece_retry_capacity_rejects_invalid_result() {
        let capacity = super::token_piece_retry_capacity(i32::MIN, 128);

        assert!(matches!(
            capacity,
            Err(localmt_engine::TranslationError::EngineUnavailable(reason))
                if reason == "llama.cpp returned an invalid token text length"
        ));
    }

    #[test]
    fn prompt_uses_owned_hymt_template_and_language_label() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = TranslateRequest::new(
            Language::English,
            Language::Russian,
            NonEmptyText::new("hello offline".to_owned())?,
        )?;

        let prompt = LlamaTranslationPrompt::from_request(&request);

        assert_eq!(
            prompt.as_str(),
            "<｜hy_begin▁of▁sentence｜><｜hy_User｜>Translate the following segment into Russian, without additional explanation.\n\nhello offline<｜hy_Assistant｜>"
        );
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "llama-runtime"))]
    fn translator_load_reports_disabled_runtime_without_feature()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_gguf_pack()?;
        let plan = LlamaModelPlan::from_assets(verified_gguf_assets(&root)?)?;

        let error = LlamaTranslator::load(plan).err();

        assert!(matches!(error, Some(LlamaEngineError::RuntimeDisabled)));
        Ok(())
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_dylib_path_rejects_missing_env_value() {
        let error = super::resolve_llama_cpp_dylib_path_from_env_value(None);

        assert!(matches!(
            error,
            Err(LlamaEngineError::MissingLlamaDylibPath)
        ));
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_dylib_path_rejects_relative_env_value() {
        let error =
            super::resolve_llama_cpp_dylib_path_from_env_value(Some("libllama.dylib".into()));

        assert!(matches!(
            error,
            Err(LlamaEngineError::InvalidLlamaDylibPath { ref reason, .. })
                if reason == "path must be absolute"
        ));
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn runtime_dylib_path_rejects_missing_file() {
        let missing_path = PathBuf::from("/tmp/localmt-missing-libllama.dylib");

        let error =
            super::resolve_llama_cpp_dylib_path_from_env_value(Some(missing_path.into_os_string()));

        assert!(matches!(
            error,
            Err(LlamaEngineError::InvalidLlamaDylibPath { ref reason, .. })
                if reason == "path does not point to a file"
        ));
    }

    #[test]
    #[cfg(feature = "llama-runtime")]
    fn native_loader_rejects_non_library_file() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        let fake_library = root.join("libllama.dylib");
        fs::write(&fake_library, "not a dynamic library\n")?;

        let error = super::LlamaNativeLibrary::load(&fake_library);

        assert!(matches!(
            error,
            Err(LlamaEngineError::LoadNativeLibrary { .. })
        ));
        Ok(())
    }

    #[test]
    fn disabled_translator_still_implements_engine_trait() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = TranslateRequest::new(
            Language::English,
            Language::Japanese,
            NonEmptyText::new("hello offline".to_owned())?,
        )?;
        let translator = LlamaTranslator::disabled_for_tests();

        let error = translator.translate(&request).err();

        assert!(matches!(
            error,
            Some(localmt_engine::TranslationError::EngineUnavailable(_))
        ));
        Ok(())
    }

    fn verified_gguf_assets(
        root: &std::path::Path,
    ) -> Result<GgufModelAssetPlan, Box<dyn std::error::Error>> {
        let pack = ModelPack::<Discovered>::discover(root)?.verify()?;
        Ok(GgufModelAssetPlan::from_pack(&pack)?)
    }

    fn create_gguf_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("hymt.gguf"), "hymt gguf\n")?;
        fs::write(root.join("chat-template.jinja"), "template\n")?;
        fs::write(root.join("llama-runtime.json"), "llama runtime\n")?;
        fs::write(
            root.join("manifest.json"),
            format!(
                r#"{{
  "schema_version": 0,
  "model_id": "hymt-1.25bit",
  "version": "0.1.0",
  "architecture": "hunyuan-dense",
  "runtime": "llama.cpp",
  "license": "Tencent Hunyuan Community",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
    {{ "path": "hymt.gguf", "kind": "gguf_model", "sha256": "{GGUF_MODEL_SHA256}" }},
    {{ "path": "chat-template.jinja", "kind": "chat_template", "sha256": "{CHAT_TEMPLATE_SHA256}" }},
    {{ "path": "llama-runtime.json", "kind": "llama_runtime_config", "sha256": "{LLAMA_RUNTIME_CONFIG_SHA256}" }}
  ]
}}"#
            ),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-engine-llama-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }
}
