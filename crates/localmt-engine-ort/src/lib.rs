//! ONNX Runtime adapter boundary for localmt.

use core::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use localmt_models::{ModelFileRole, ModelPack, Verified};
use localmt_pipeline::{
    GenerationConfig, GenerationConfigParseError, GeneratorAssetPlan, TokenGenerator,
    TokenGeneratorError,
};
use localmt_tokenizer::{TokenSequence, TokenizerOutput};
use serde::Deserialize;

const ONNX_RUNTIME: &str = "onnx-runtime";
const GENERATION_LOOP_UNIMPLEMENTED: &str = "ONNX token generation loop is not implemented";

/// Typed ONNX tensor names required by the ORT generation loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtIoConfig {
    encoder: OrtEncoderIoNames,
    decoder: OrtDecoderIoNames,
    decoder_with_past: Option<OrtDecoderIoNames>,
}

impl OrtIoConfig {
    /// { path points to a readable JSON config file }
    /// fn from_json_file(path: &Path) -> Result<Self, OrtIoConfigParseError>
    /// { ret is Ok only when file contents contain a valid ort_io object }
    pub fn from_json_file(path: &Path) -> Result<Self, OrtIoConfigParseError> {
        let contents =
            fs::read_to_string(path).map_err(|source| OrtIoConfigParseError::ReadFile {
                path: path.to_path_buf(),
                reason: source.to_string(),
            })?;

        Self::from_json_str(&contents)
    }

    /// { contents is a JSON config document }
    /// fn from_json_str(contents: &str) -> Result<Self, OrtIoConfigParseError>
    /// { ret is Ok only when contents contain a valid ort_io object }
    pub fn from_json_str(contents: &str) -> Result<Self, OrtIoConfigParseError> {
        let raw = serde_json::from_str::<RawOrtIoConfigEnvelope>(contents).map_err(|source| {
            OrtIoConfigParseError::ParseJson {
                reason: source.to_string(),
            }
        })?;

        raw.try_into_config()
    }

    /// { true }
    /// fn encoder(&self) -> &OrtEncoderIoNames
    /// { ret is the configured encoder tensor-name contract }
    pub const fn encoder(&self) -> &OrtEncoderIoNames {
        &self.encoder
    }

    /// { true }
    /// fn decoder(&self) -> &OrtDecoderIoNames
    /// { ret is the configured decoder tensor-name contract }
    pub const fn decoder(&self) -> &OrtDecoderIoNames {
        &self.decoder
    }

    /// { true }
    /// fn decoder_with_past(&self) -> `Option<&OrtDecoderIoNames>`
    /// { ret is Some only when cached-decoder tensor names are configured }
    pub const fn decoder_with_past(&self) -> Option<&OrtDecoderIoNames> {
        self.decoder_with_past.as_ref()
    }
}

/// Encoder ONNX tensor names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtEncoderIoNames {
    input_ids: String,
    attention_mask: String,
    last_hidden_state: String,
}

impl OrtEncoderIoNames {
    /// { tensor name strings are candidate ONNX graph names }
    /// fn new(input_ids: String, attention_mask: String, last_hidden_state: String) -> Result<Self, OrtIoConfigError>
    /// { ret is Ok only when all tensor names are non-empty after trimming }
    pub fn new(
        input_ids: String,
        attention_mask: String,
        last_hidden_state: String,
    ) -> Result<Self, OrtIoConfigError> {
        Ok(Self {
            input_ids: normalized_tensor_name("ort_io.encoder.input_ids", input_ids)?,
            attention_mask: normalized_tensor_name(
                "ort_io.encoder.attention_mask",
                attention_mask,
            )?,
            last_hidden_state: normalized_tensor_name(
                "ort_io.encoder.last_hidden_state",
                last_hidden_state,
            )?,
        })
    }

    /// { true }
    /// fn input_ids(&self) -> &str
    /// { ret is the encoder token-id input tensor name }
    pub fn input_ids(&self) -> &str {
        &self.input_ids
    }

    /// { true }
    /// fn attention_mask(&self) -> &str
    /// { ret is the encoder attention-mask input tensor name }
    pub fn attention_mask(&self) -> &str {
        &self.attention_mask
    }

    /// { true }
    /// fn last_hidden_state(&self) -> &str
    /// { ret is the encoder hidden-state output tensor name }
    pub fn last_hidden_state(&self) -> &str {
        &self.last_hidden_state
    }
}

/// Decoder ONNX tensor names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtDecoderIoNames {
    input_ids: String,
    encoder_attention_mask: String,
    encoder_hidden_states: String,
    logits: String,
}

impl OrtDecoderIoNames {
    /// { tensor name strings are candidate ONNX graph names }
    /// fn new(input_ids: String, encoder_attention_mask: String, encoder_hidden_states: String, logits: String) -> Result<Self, OrtIoConfigError>
    /// { ret is Ok only when all tensor names are non-empty after trimming }
    pub fn new(
        input_ids: String,
        encoder_attention_mask: String,
        encoder_hidden_states: String,
        logits: String,
    ) -> Result<Self, OrtIoConfigError> {
        Self::new_with_fields(
            DECODER_IO_FIELDS,
            input_ids,
            encoder_attention_mask,
            encoder_hidden_states,
            logits,
        )
    }

    /// { true }
    /// fn input_ids(&self) -> &str
    /// { ret is the decoder token-id input tensor name }
    pub fn input_ids(&self) -> &str {
        &self.input_ids
    }

    /// { true }
    /// fn encoder_attention_mask(&self) -> &str
    /// { ret is the decoder encoder-attention-mask input tensor name }
    pub fn encoder_attention_mask(&self) -> &str {
        &self.encoder_attention_mask
    }

    /// { true }
    /// fn encoder_hidden_states(&self) -> &str
    /// { ret is the decoder encoder-hidden-states input tensor name }
    pub fn encoder_hidden_states(&self) -> &str {
        &self.encoder_hidden_states
    }

    /// { true }
    /// fn logits(&self) -> &str
    /// { ret is the decoder logits output tensor name }
    pub fn logits(&self) -> &str {
        &self.logits
    }

    fn new_with_fields(
        fields: OrtDecoderIoFieldNames,
        input_ids: String,
        encoder_attention_mask: String,
        encoder_hidden_states: String,
        logits: String,
    ) -> Result<Self, OrtIoConfigError> {
        Ok(Self {
            input_ids: normalized_tensor_name(fields.input_ids, input_ids)?,
            encoder_attention_mask: normalized_tensor_name(
                fields.encoder_attention_mask,
                encoder_attention_mask,
            )?,
            encoder_hidden_states: normalized_tensor_name(
                fields.encoder_hidden_states,
                encoder_hidden_states,
            )?,
            logits: normalized_tensor_name(fields.logits, logits)?,
        })
    }
}

#[derive(Clone, Copy)]
struct OrtDecoderIoFieldNames {
    input_ids: &'static str,
    encoder_attention_mask: &'static str,
    encoder_hidden_states: &'static str,
    logits: &'static str,
}

const DECODER_IO_FIELDS: OrtDecoderIoFieldNames = OrtDecoderIoFieldNames {
    input_ids: "ort_io.decoder.input_ids",
    encoder_attention_mask: "ort_io.decoder.encoder_attention_mask",
    encoder_hidden_states: "ort_io.decoder.encoder_hidden_states",
    logits: "ort_io.decoder.logits",
};

const DECODER_WITH_PAST_IO_FIELDS: OrtDecoderIoFieldNames = OrtDecoderIoFieldNames {
    input_ids: "ort_io.decoder_with_past.input_ids",
    encoder_attention_mask: "ort_io.decoder_with_past.encoder_attention_mask",
    encoder_hidden_states: "ort_io.decoder_with_past.encoder_hidden_states",
    logits: "ort_io.decoder_with_past.logits",
};

/// ORT I/O config semantic validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtIoConfigError {
    /// A required tensor name is empty after trimming.
    EmptyTensorName {
        /// Field path inside the JSON document.
        field: &'static str,
    },
}

impl fmt::Display for OrtIoConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTensorName { field } => {
                write!(formatter, "tensor name {field} must not be empty")
            }
        }
    }
}

impl std::error::Error for OrtIoConfigError {}

/// ORT I/O config JSON parsing error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrtIoConfigParseError {
    /// Config file could not be read.
    ReadFile {
        /// Path attempted.
        path: PathBuf,
        /// Filesystem error message.
        reason: String,
    },
    /// Config JSON could not be decoded.
    ParseJson {
        /// Parser error message.
        reason: String,
    },
    /// Config JSON decoded but has no ort_io object.
    MissingOrtIoConfig,
    /// Config JSON decoded but failed semantic validation.
    InvalidConfig(OrtIoConfigError),
}

impl fmt::Display for OrtIoConfigParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFile { path, reason } => {
                write!(
                    formatter,
                    "failed to read ORT I/O config {}: {reason}",
                    path.display()
                )
            }
            Self::ParseJson { reason } => write!(formatter, "invalid ORT I/O JSON: {reason}"),
            Self::MissingOrtIoConfig => formatter.write_str("missing ort_io config object"),
            Self::InvalidConfig(error) => write!(formatter, "invalid ORT I/O config: {error}"),
        }
    }
}

impl std::error::Error for OrtIoConfigParseError {}

#[derive(Deserialize)]
struct RawOrtIoConfigEnvelope {
    ort_io: Option<RawOrtIoConfig>,
}

impl RawOrtIoConfigEnvelope {
    fn try_into_config(self) -> Result<OrtIoConfig, OrtIoConfigParseError> {
        let raw_config = self
            .ort_io
            .ok_or(OrtIoConfigParseError::MissingOrtIoConfig)?;

        raw_config
            .try_into_config()
            .map_err(OrtIoConfigParseError::InvalidConfig)
    }
}

#[derive(Deserialize)]
struct RawOrtIoConfig {
    encoder: RawOrtEncoderIoNames,
    decoder: RawOrtDecoderIoNames,
    decoder_with_past: Option<RawOrtDecoderIoNames>,
}

impl RawOrtIoConfig {
    fn try_into_config(self) -> Result<OrtIoConfig, OrtIoConfigError> {
        Ok(OrtIoConfig {
            encoder: self.encoder.try_into_names()?,
            decoder: self.decoder.try_into_names(DECODER_IO_FIELDS)?,
            decoder_with_past: self
                .decoder_with_past
                .map(|names| names.try_into_names(DECODER_WITH_PAST_IO_FIELDS))
                .transpose()?,
        })
    }
}

#[derive(Deserialize)]
struct RawOrtEncoderIoNames {
    input_ids: String,
    attention_mask: String,
    last_hidden_state: String,
}

impl RawOrtEncoderIoNames {
    fn try_into_names(self) -> Result<OrtEncoderIoNames, OrtIoConfigError> {
        OrtEncoderIoNames::new(self.input_ids, self.attention_mask, self.last_hidden_state)
    }
}

#[derive(Deserialize)]
struct RawOrtDecoderIoNames {
    input_ids: String,
    encoder_attention_mask: String,
    encoder_hidden_states: String,
    logits: String,
}

impl RawOrtDecoderIoNames {
    fn try_into_names(
        self,
        fields: OrtDecoderIoFieldNames,
    ) -> Result<OrtDecoderIoNames, OrtIoConfigError> {
        OrtDecoderIoNames::new_with_fields(
            fields,
            self.input_ids,
            self.encoder_attention_mask,
            self.encoder_hidden_states,
            self.logits,
        )
    }
}

fn normalized_tensor_name(field: &'static str, value: String) -> Result<String, OrtIoConfigError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OrtIoConfigError::EmptyTensorName { field });
    }

    Ok(trimmed.to_owned())
}

/// ONNX model role selected from a verified model pack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtModelRole {
    /// Encoder graph file.
    Encoder,
    /// Decoder graph file.
    Decoder,
    /// Decoder graph file with cached past-key-values.
    DecoderWithPast,
}

impl OrtModelRole {
    /// { true }
    /// fn as_str(self) -> &'static str
    /// { ret is the stable role id }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Encoder => "encoder",
            Self::Decoder => "decoder",
            Self::DecoderWithPast => "decoder_with_past",
        }
    }

    /// { true }
    /// fn model_file_role(self) -> ModelFileRole
    /// { ret is the model-pack file role required for this ORT model role }
    pub const fn model_file_role(self) -> ModelFileRole {
        match self {
            Self::Encoder => ModelFileRole::Encoder,
            Self::Decoder => ModelFileRole::Decoder,
            Self::DecoderWithPast => ModelFileRole::DecoderWithPast,
        }
    }
}

impl fmt::Display for OrtModelRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Verified ONNX session load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtSessionPlan {
    model_id: String,
    role: OrtModelRole,
    model_path: PathBuf,
}

impl OrtSessionPlan {
    fn new(model_id: String, role: OrtModelRole, model_path: PathBuf) -> Self {
        Self {
            model_id,
            role,
            model_path,
        }
    }

    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`, role: OrtModelRole) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when pack declares an ONNX Runtime file for role }
    pub fn from_pack(
        pack: &ModelPack<Verified>,
        role: OrtModelRole,
    ) -> Result<Self, OrtEngineError> {
        let runtime = pack.manifest().runtime().as_str();
        if runtime != ONNX_RUNTIME {
            return Err(OrtEngineError::UnsupportedRuntime(runtime.to_owned()));
        }

        let model_path = pack
            .file_path(role.model_file_role())
            .ok_or(OrtEngineError::MissingModelFile(role))?;

        Ok(Self::new(
            pack.manifest().model_id().as_str().to_owned(),
            role,
            model_path,
        ))
    }

    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model-pack id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn role(&self) -> OrtModelRole
    /// { ret is the model role selected for this session }
    pub const fn role(&self) -> OrtModelRole {
        self.role
    }

    /// { true }
    /// fn model_path(&self) -> &Path
    /// { ret is the absolute path to the selected ONNX file }
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

/// Verified ONNX generator load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGeneratorPlan {
    model_id: String,
    encoder: OrtSessionPlan,
    decoder: OrtSessionPlan,
    decoder_with_past: Option<OrtSessionPlan>,
    config_path: Option<PathBuf>,
    generation_config_path: Option<PathBuf>,
}

impl OrtGeneratorPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when pack targets ONNX Runtime and generator assets are present }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, OrtEngineError> {
        let runtime = pack.manifest().runtime().as_str();
        if runtime != ONNX_RUNTIME {
            return Err(OrtEngineError::UnsupportedRuntime(runtime.to_owned()));
        }

        let assets = GeneratorAssetPlan::from_pack(pack).map_err(OrtEngineError::GeneratorAsset)?;
        let model_id = pack.manifest().model_id().as_str().to_owned();
        let encoder = OrtSessionPlan::new(
            model_id.clone(),
            OrtModelRole::Encoder,
            assets.encoder_path().to_path_buf(),
        );
        let decoder = OrtSessionPlan::new(
            model_id.clone(),
            OrtModelRole::Decoder,
            assets.decoder_path().to_path_buf(),
        );
        let decoder_with_past = assets.decoder_with_past_path().map(|path| {
            OrtSessionPlan::new(
                model_id.clone(),
                OrtModelRole::DecoderWithPast,
                path.to_path_buf(),
            )
        });

        Ok(Self {
            model_id,
            encoder,
            decoder,
            decoder_with_past,
            config_path: pack.file_path(ModelFileRole::Config),
            generation_config_path: assets.generation_config_path().map(Path::to_path_buf),
        })
    }

    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model-pack id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn encoder(&self) -> &OrtSessionPlan
    /// { ret is the encoder session plan }
    pub const fn encoder(&self) -> &OrtSessionPlan {
        &self.encoder
    }

    /// { true }
    /// fn decoder(&self) -> &OrtSessionPlan
    /// { ret is the decoder session plan }
    pub const fn decoder(&self) -> &OrtSessionPlan {
        &self.decoder
    }

    /// { true }
    /// fn decoder_with_past(&self) -> `Option<&OrtSessionPlan>`
    /// { ret is Some only when the pack declares decoder_with_past }
    pub const fn decoder_with_past(&self) -> Option<&OrtSessionPlan> {
        self.decoder_with_past.as_ref()
    }

    /// { true }
    /// fn generation_config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares generation_config }
    pub fn generation_config_path(&self) -> Option<&Path> {
        self.generation_config_path.as_deref()
    }

    /// { true }
    /// fn config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares a config file }
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// { self was built from a verified ONNX Runtime model pack }
    /// fn parse_generation_config(&self) -> `Result<Option<GenerationConfig>, OrtEngineError>`
    /// { ret is Ok(Some) only when a declared generation_config file parses and validates }
    pub fn parse_generation_config(&self) -> Result<Option<GenerationConfig>, OrtEngineError> {
        self.generation_config_path()
            .map(GenerationConfig::from_json_file)
            .transpose()
            .map_err(OrtEngineError::GenerationConfig)
    }

    /// { self was built from a verified ONNX Runtime model pack }
    /// fn parse_ort_io_config(&self) -> `Result<Option<OrtIoConfig>, OrtEngineError>`
    /// { ret is Ok(Some) only when a declared config file contains valid ort_io tensor names }
    pub fn parse_ort_io_config(&self) -> Result<Option<OrtIoConfig>, OrtEngineError> {
        self.config_path()
            .map(OrtIoConfig::from_json_file)
            .transpose()
            .map_err(OrtEngineError::IoConfig)
    }
}

/// ONNX Runtime adapter error.
#[derive(Debug)]
pub enum OrtEngineError {
    /// Model pack targets a different runtime.
    UnsupportedRuntime(String),
    /// Model pack has no ONNX file for the requested role.
    MissingModelFile(OrtModelRole),
    /// Generator asset planning failed before ORT session planning.
    GeneratorAsset(TokenGeneratorError),
    /// Generation config parsing failed.
    GenerationConfig(GenerationConfigParseError),
    /// ORT I/O config parsing failed.
    IoConfig(OrtIoConfigParseError),
    /// Crate was compiled without the `ort-runtime` feature.
    OrtRuntimeFeatureDisabled,
    /// ONNX Runtime failed to create a session.
    Ort(String),
}

impl fmt::Display for OrtEngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRuntime(runtime) => {
                write!(formatter, "unsupported model runtime for ORT: {runtime}")
            }
            Self::MissingModelFile(role) => write!(formatter, "missing ORT model file: {role}"),
            Self::GeneratorAsset(error) => write!(formatter, "{error}"),
            Self::GenerationConfig(error) => write!(formatter, "{error}"),
            Self::IoConfig(error) => write!(formatter, "{error}"),
            Self::OrtRuntimeFeatureDisabled => {
                formatter.write_str("ort-runtime feature is not enabled")
            }
            Self::Ort(error) => write!(formatter, "ONNX Runtime error: {error}"),
        }
    }
}

impl std::error::Error for OrtEngineError {}

/// ONNX Runtime token generator handle.
#[cfg(not(feature = "ort-runtime"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrtTokenGenerator;

#[cfg(not(feature = "ort-runtime"))]
impl OrtTokenGenerator {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError>
    /// { ret is Err when compiled without ort-runtime }
    pub fn load(_plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError> {
        Err(OrtEngineError::OrtRuntimeFeatureDisabled)
    }
}

/// ONNX Runtime token generator handle.
#[cfg(feature = "ort-runtime")]
#[derive(Debug)]
pub struct OrtTokenGenerator {
    plan: OrtGeneratorPlan,
    encoder: OrtEngine,
    decoder: OrtEngine,
    decoder_with_past: Option<OrtEngine>,
}

#[cfg(feature = "ort-runtime")]
impl OrtTokenGenerator {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when ONNX Runtime loads required generator sessions }
    pub fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError> {
        let encoder = OrtEngine::load(plan.encoder().clone())?;
        let decoder = OrtEngine::load(plan.decoder().clone())?;
        let decoder_with_past = plan
            .decoder_with_past()
            .cloned()
            .map(OrtEngine::load)
            .transpose()?;

        Ok(Self {
            plan,
            encoder,
            decoder,
            decoder_with_past,
        })
    }

    /// { self was loaded successfully }
    /// fn plan(&self) -> &OrtGeneratorPlan
    /// { ret is the verified generator plan used for loading }
    pub const fn plan(&self) -> &OrtGeneratorPlan {
        &self.plan
    }

    /// { self was loaded successfully }
    /// fn encoder(&self) -> &OrtEngine
    /// { ret is the loaded encoder session wrapper }
    pub const fn encoder(&self) -> &OrtEngine {
        &self.encoder
    }

    /// { self was loaded successfully }
    /// fn decoder(&self) -> &OrtEngine
    /// { ret is the loaded decoder session wrapper }
    pub const fn decoder(&self) -> &OrtEngine {
        &self.decoder
    }

    /// { self was loaded successfully }
    /// fn decoder_with_past(&self) -> `Option<&OrtEngine>`
    /// { ret is Some only when the cached decoder session was loaded }
    pub const fn decoder_with_past(&self) -> Option<&OrtEngine> {
        self.decoder_with_past.as_ref()
    }
}

impl TokenGenerator for OrtTokenGenerator {
    /// { input contains validated source tokens and target language }
    /// fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError>
    /// { ret is Err until ONNX encoder/decoder token generation is implemented }
    fn generate(&self, _input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
        Err(TokenGeneratorError::BackendUnavailable(
            GENERATION_LOOP_UNIMPLEMENTED.to_owned(),
        ))
    }
}

/// ONNX Runtime engine handle.
#[cfg(not(feature = "ort-runtime"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrtEngine;

#[cfg(not(feature = "ort-runtime"))]
impl OrtEngine {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError>
    /// { ret is Err when compiled without ort-runtime }
    pub fn load(_plan: OrtSessionPlan) -> Result<Self, OrtEngineError> {
        Err(OrtEngineError::OrtRuntimeFeatureDisabled)
    }
}

/// ONNX Runtime engine handle.
#[cfg(feature = "ort-runtime")]
#[derive(Debug)]
pub struct OrtEngine {
    plan: OrtSessionPlan,
    session: ort::session::Session,
}

#[cfg(feature = "ort-runtime")]
impl OrtEngine {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when ONNX Runtime creates a session from plan.model_path() }
    pub fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError> {
        let session = ort::session::Session::builder()
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?
            .commit_from_file(plan.model_path())
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;

        Ok(Self { plan, session })
    }

    /// { self was loaded successfully }
    /// fn plan(&self) -> &OrtSessionPlan
    /// { ret is the verified session load plan }
    pub const fn plan(&self) -> &OrtSessionPlan {
        &self.plan
    }

    /// { self was loaded successfully }
    /// fn input_count(&self) -> usize
    /// { ret is the number of ONNX graph inputs }
    pub fn input_count(&self) -> usize {
        self.session.inputs().len()
    }

    /// { self was loaded successfully }
    /// fn output_count(&self) -> usize
    /// { ret is the number of ONNX graph outputs }
    pub fn output_count(&self) -> usize {
        self.session.outputs().len()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_models::{Discovered, ModelFileRole, ModelPack};
    use localmt_pipeline::TokenGeneratorError;
    use localmt_tokenizer::TokenId;

    #[cfg(not(feature = "ort-runtime"))]
    use crate::{OrtEngine, OrtTokenGenerator};
    use crate::{
        OrtEngineError, OrtGeneratorPlan, OrtIoConfig, OrtIoConfigError, OrtIoConfigParseError,
        OrtModelRole, OrtSessionPlan,
    };

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const DECODER_WITH_PAST_SHA256: &str =
        "ef37d12b277fe98960af949b560ded559157bf42d2eea244dcfc64ac08da666c";
    const GENERATION_CONFIG_SHA256: &str =
        "75f35767c145e896ca56dba402cc97c3879445dace669c745ecea5fc0c9552b6";
    const VALID_GENERATION_CONFIG_SHA256: &str =
        "a8a99326d564beb1fc16cb59526dd5ed7b5fd673f969591f47845e17fbed401d";
    const VALID_GENERATION_CONFIG: &str = r#"{
  "max_new_tokens": 32,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#;
    const VALID_ORT_IO_CONFIG_SHA256: &str =
        "3ed8af0b5a58d51e246f3081e973d8683b89bf3cacf23c26243fec19ab31a721";
    const VALID_ORT_IO_CONFIG: &str = r#"{
  "ort_io": {
    "encoder": {
      "input_ids": "encoder_input_ids",
      "attention_mask": "encoder_attention_mask",
      "last_hidden_state": "encoder_last_hidden_state"
    },
    "decoder": {
      "input_ids": "decoder_input_ids",
      "encoder_attention_mask": "decoder_encoder_attention_mask",
      "encoder_hidden_states": "decoder_encoder_hidden_states",
      "logits": "decoder_logits"
    },
    "decoder_with_past": {
      "input_ids": "past_input_ids",
      "encoder_attention_mask": "past_encoder_attention_mask",
      "encoder_hidden_states": "past_encoder_hidden_states",
      "logits": "past_logits"
    }
  }
}"#;
    const EMPTY_ORT_IO_CONFIG: &str = r#"{
  "ort_io": {
    "encoder": {
      "input_ids": " ",
      "attention_mask": "encoder_attention_mask",
      "last_hidden_state": "encoder_last_hidden_state"
    },
    "decoder": {
      "input_ids": "decoder_input_ids",
      "encoder_attention_mask": "decoder_encoder_attention_mask",
      "encoder_hidden_states": "decoder_encoder_hidden_states",
      "logits": "decoder_logits"
    }
  }
}"#;
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn session_plan_selects_encoder_file_from_verified_pack()
    -> Result<(), Box<dyn std::error::Error>> {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(true, "onnx-runtime")?)?.verify()?;

        let plan = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder)?;

        assert_eq!(plan.model_id(), "m2m100-418m-int8");
        assert_eq!(plan.role(), OrtModelRole::Encoder);
        assert_eq!(
            plan.model_path(),
            pack.root().join("encoder.onnx").as_path()
        );
        Ok(())
    }

    #[test]
    fn session_plan_rejects_missing_encoder_file() -> Result<(), Box<dyn std::error::Error>> {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(false, "onnx-runtime")?)?.verify()?;

        let error = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder);

        assert!(matches!(
            error,
            Err(OrtEngineError::MissingModelFile(OrtModelRole::Encoder))
        ));
        Ok(())
    }

    #[test]
    fn session_plan_rejects_non_ort_runtime() -> Result<(), Box<dyn std::error::Error>> {
        let pack = ModelPack::<Discovered>::discover(create_pack(true, "candle")?)?.verify()?;

        let error = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder);

        assert!(matches!(
            error,
            Err(OrtEngineError::UnsupportedRuntime(ref runtime)) if runtime == "candle"
        ));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn load_returns_feature_disabled_without_ort_runtime() -> Result<(), Box<dyn std::error::Error>>
    {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(true, "onnx-runtime")?)?.verify()?;
        let plan = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder)?;

        let error = OrtEngine::load(plan);

        assert!(matches!(
            error,
            Err(OrtEngineError::OrtRuntimeFeatureDisabled)
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_selects_required_and_optional_assets()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
                (
                    "decoder-with-past.onnx",
                    "decoder_with_past",
                    DECODER_WITH_PAST_SHA256,
                    "decoder-with-past\n",
                ),
                (
                    "generation.json",
                    "generation_config",
                    GENERATION_CONFIG_SHA256,
                    "generation\n",
                ),
            ],
        )?;
        let decoder_with_past_path = root.join("decoder-with-past.onnx");
        let generation_config_path = root.join("generation.json");
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        assert_eq!(plan.model_id(), "m2m100-418m-int8");
        assert_eq!(plan.encoder().role(), OrtModelRole::Encoder);
        assert_eq!(plan.decoder().role(), OrtModelRole::Decoder);
        assert_eq!(
            plan.decoder_with_past().map(OrtSessionPlan::role),
            Some(OrtModelRole::DecoderWithPast)
        );
        assert_eq!(
            plan.decoder_with_past().map(OrtSessionPlan::model_path),
            Some(decoder_with_past_path.as_path())
        );
        assert_eq!(
            plan.generation_config_path(),
            Some(generation_config_path.as_path())
        );
        Ok(())
    }

    #[test]
    fn generator_plan_rejects_non_ort_runtime() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "candle",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OrtEngineError::UnsupportedRuntime(ref runtime)) if runtime == "candle"
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_rejects_missing_decoder_asset() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n")],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OrtEngineError::GeneratorAsset(
                TokenGeneratorError::MissingGeneratorAsset(ModelFileRole::Decoder)
            ))
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_parses_optional_generation_config() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
                (
                    "generation.json",
                    "generation_config",
                    VALID_GENERATION_CONFIG_SHA256,
                    VALID_GENERATION_CONFIG,
                ),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let config = plan.parse_generation_config()?;

        assert!(matches!(
            config,
            Some(item)
                if item.max_new_tokens().value() == 32
                    && item.bos_token_id() == TokenId::new(0)
                    && item.eos_token_id() == TokenId::new(1)
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_returns_none_without_generation_config()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let config = plan.parse_generation_config()?;

        assert_eq!(config, None);
        Ok(())
    }

    #[test]
    fn ort_io_config_parses_tensor_names_from_json_str() -> Result<(), Box<dyn std::error::Error>> {
        let config = OrtIoConfig::from_json_str(VALID_ORT_IO_CONFIG)?;

        assert_eq!(config.encoder().input_ids(), "encoder_input_ids");
        assert_eq!(config.encoder().attention_mask(), "encoder_attention_mask");
        assert_eq!(
            config.encoder().last_hidden_state(),
            "encoder_last_hidden_state"
        );
        assert_eq!(config.decoder().input_ids(), "decoder_input_ids");
        assert_eq!(config.decoder().logits(), "decoder_logits");
        assert_eq!(
            config.decoder_with_past().map(|decoder| decoder.logits()),
            Some("past_logits")
        );
        Ok(())
    }

    #[test]
    fn ort_io_config_rejects_empty_tensor_names() {
        let config = OrtIoConfig::from_json_str(EMPTY_ORT_IO_CONFIG);

        assert!(matches!(
            config,
            Err(OrtIoConfigParseError::InvalidConfig(
                OrtIoConfigError::EmptyTensorName { field }
            )) if field == "ort_io.encoder.input_ids"
        ));
    }

    #[test]
    fn generator_plan_parses_optional_ort_io_config() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
                (
                    "config.json",
                    "config",
                    VALID_ORT_IO_CONFIG_SHA256,
                    VALID_ORT_IO_CONFIG,
                ),
            ],
        )?;
        let config_path = root.join("config.json");
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let config = plan.parse_ort_io_config()?;

        assert_eq!(plan.config_path(), Some(config_path.as_path()));
        assert!(matches!(
            config,
            Some(item)
                if item.encoder().input_ids() == "encoder_input_ids"
                    && item.decoder().encoder_hidden_states()
                        == "decoder_encoder_hidden_states"
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_returns_none_without_ort_io_config() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let config = plan.parse_ort_io_config()?;

        assert_eq!(plan.config_path(), None);
        assert_eq!(config, None);
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn token_generator_load_returns_feature_disabled_without_ort_runtime()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let error = OrtTokenGenerator::load(plan);

        assert!(matches!(
            error,
            Err(OrtEngineError::OrtRuntimeFeatureDisabled)
        ));
        Ok(())
    }

    fn create_pack(
        include_encoder: bool,
        runtime: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(
            root.join("manifest.json"),
            manifest_json(include_encoder, runtime),
        )?;
        Ok(root)
    }

    fn create_pack_with_files(
        runtime: &str,
        files: &[(&str, &str, &str, &str)],
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        for (path, _kind, _sha256, contents) in files {
            fs::write(root.join(path), contents)?;
        }
        fs::write(
            root.join("manifest.json"),
            manifest_json_for_files(runtime, files),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-engine-ort-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn manifest_json(include_encoder: bool, runtime: &str) -> String {
        let files = if include_encoder {
            format!(
                r#"{{
      "path": "encoder.onnx",
      "kind": "encoder",
      "sha256": "{ENCODER_SHA256}"
    }},
    {{
      "path": "tokenizer.json",
      "kind": "tokenizer",
      "sha256": "{TOKENIZER_SHA256}"
    }}"#
            )
        } else {
            format!(
                r#"{{
      "path": "tokenizer.json",
      "kind": "tokenizer",
      "sha256": "{TOKENIZER_SHA256}"
    }}"#
            )
        };

        format!(
            r#"{{
  "schema_version": 0,
  "model_id": "m2m100-418m-int8",
  "version": "0.1.0",
  "architecture": "m2m100",
  "runtime": "{runtime}",
  "license": "MIT",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
    {files}
  ]
}}"#
        )
    }

    fn manifest_json_for_files(runtime: &str, files: &[(&str, &str, &str, &str)]) -> String {
        let file_json = files
            .iter()
            .map(|(path, kind, sha256, _contents)| {
                format!(r#"    {{ "path": "{path}", "kind": "{kind}", "sha256": "{sha256}" }}"#)
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            r#"{{
  "schema_version": 0,
  "model_id": "m2m100-418m-int8",
  "version": "0.1.0",
  "architecture": "m2m100",
  "runtime": "{runtime}",
  "license": "MIT",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
{file_json}
  ]
}}"#
        )
    }
}
