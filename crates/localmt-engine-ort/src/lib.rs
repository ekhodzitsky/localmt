//! ONNX Runtime adapter boundary for localmt.

use core::{fmt, num::NonZeroUsize};
#[cfg(feature = "ort-runtime")]
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "ort-runtime")]
use std::sync::OnceLock;

use localmt_models::{ModelFileRole, ModelPack, Verified};
use localmt_pipeline::{
    GenerationConfig, GenerationConfigParseError, GeneratorAssetPlan, TokenGenerator,
    TokenGeneratorError,
};
use localmt_tokenizer::{TokenId, TokenSequence, TokenizerOutput};
use serde::Deserialize;

const ONNX_RUNTIME: &str = "onnx-runtime";
const ORT_DYLIB_PATH_ENV: &str = "ORT_DYLIB_PATH";
const ORT_THREAD_CAP: usize = 4;
#[cfg(not(feature = "ort-runtime"))]
const GENERATION_LOOP_UNIMPLEMENTED: &str = "ONNX token generation loop is not implemented";
#[cfg(feature = "ort-runtime")]
static ORT_DYLIB_PATH_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

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

/// Strict config required before ORT token generation can run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGeneratorRuntimeConfig {
    generation_config: GenerationConfig,
    ort_io_config: OrtIoConfig,
}

impl OrtGeneratorRuntimeConfig {
    /// { generation_config and ort_io_config were parsed from verified model-pack files }
    /// fn new(generation_config: GenerationConfig, ort_io_config: OrtIoConfig) -> Self
    /// { ret contains the strict decoder-loop and tensor-name runtime contract }
    pub const fn new(generation_config: GenerationConfig, ort_io_config: OrtIoConfig) -> Self {
        Self {
            generation_config,
            ort_io_config,
        }
    }

    /// { true }
    /// fn generation_config(&self) -> GenerationConfig
    /// { ret is the parsed decoder-loop generation config }
    pub const fn generation_config(&self) -> GenerationConfig {
        self.generation_config
    }

    /// { true }
    /// fn ort_io_config(&self) -> &OrtIoConfig
    /// { ret is the parsed ORT tensor-name contract }
    pub const fn ort_io_config(&self) -> &OrtIoConfig {
        &self.ort_io_config
    }
}

/// ONNX-friendly inputs prepared before ORT encoder/decoder execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGenerationInputs {
    encoder_input_ids: Vec<i64>,
    encoder_attention_mask: Vec<i64>,
    decoder_input_ids: Vec<i64>,
    max_new_tokens: usize,
    eos_token_id: i64,
}

impl OrtGenerationInputs {
    /// { input contains source tokens and config contains generation policy }
    /// fn from_tokenizer_output(input: &TokenizerOutput, config: GenerationConfig) -> Self
    /// { ret contains ONNX-friendly token ids, masks, and decoder-loop limits }
    pub fn from_tokenizer_output(input: &TokenizerOutput, config: GenerationConfig) -> Self {
        let encoder_input_ids = token_sequence_to_i64(input.tokens());
        let encoder_attention_mask = vec![1_i64; encoder_input_ids.len()];
        let mut decoder_input_ids = Vec::with_capacity(2);
        if let Some(token) = config.decoder_start_token_id() {
            decoder_input_ids.push(i64::from(token.value()));
        }
        decoder_input_ids.push(i64::from(
            config.target_language_token(input.target()).value(),
        ));
        let max_new_tokens = config.max_new_tokens().value();
        let eos_token_id = i64::from(config.eos_token_id().value());

        Self {
            encoder_input_ids,
            encoder_attention_mask,
            decoder_input_ids,
            max_new_tokens,
            eos_token_id,
        }
    }

    /// { true }
    /// fn encoder_input_ids(&self) -> &[i64]
    /// { ret is the source token ids in ONNX tensor scalar representation }
    pub fn encoder_input_ids(&self) -> &[i64] {
        &self.encoder_input_ids
    }

    /// { true }
    /// fn encoder_attention_mask(&self) -> &[i64]
    /// { ret is one attention-mask value per encoder token }
    pub fn encoder_attention_mask(&self) -> &[i64] {
        &self.encoder_attention_mask
    }

    /// { true }
    /// fn decoder_input_ids(&self) -> &[i64]
    /// { ret is the initial decoder token seed }
    pub fn decoder_input_ids(&self) -> &[i64] {
        &self.decoder_input_ids
    }

    /// { true }
    /// fn max_new_tokens(&self) -> usize
    /// { ret is the configured upper bound for generated target tokens }
    pub const fn max_new_tokens(&self) -> usize {
        self.max_new_tokens
    }

    /// { true }
    /// fn eos_token_id(&self) -> i64
    /// { ret is the configured end-of-sequence token id }
    pub const fn eos_token_id(&self) -> i64 {
        self.eos_token_id
    }
}

/// Owned row-shaped `i64` tensor payload for ONNX Runtime inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtI64TensorInput {
    shape: [usize; 2],
    values: Vec<i64>,
}

impl OrtI64TensorInput {
    /// { values is the contiguous scalar payload for one batch row }
    /// fn row(values: &[i64]) -> Self
    /// { ret has shape [1, values.len()] and preserves all scalar values }
    pub fn row(values: &[i64]) -> Self {
        Self {
            shape: [1, values.len()],
            values: values.to_vec(),
        }
    }

    /// { true }
    /// fn shape(&self) -> [usize; 2]
    /// { ret is the ORT tensor shape for the owned values }
    pub const fn shape(&self) -> [usize; 2] {
        self.shape
    }

    /// { true }
    /// fn values(&self) -> &[i64]
    /// { ret is the contiguous row-major tensor payload }
    pub fn values(&self) -> &[i64] {
        &self.values
    }
}

/// ONNX Runtime row tensors prepared from semantic generation inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGenerationTensorInputs {
    encoder_input_ids: OrtI64TensorInput,
    encoder_attention_mask: OrtI64TensorInput,
    decoder_input_ids: OrtI64TensorInput,
}

impl OrtGenerationTensorInputs {
    /// { inputs contains ONNX-friendly generation vectors }
    /// fn from_generation_inputs(inputs: &OrtGenerationInputs) -> Self
    /// { ret maps every generation vector into a [1, N] ORT row tensor payload }
    pub fn from_generation_inputs(inputs: &OrtGenerationInputs) -> Self {
        Self {
            encoder_input_ids: OrtI64TensorInput::row(inputs.encoder_input_ids()),
            encoder_attention_mask: OrtI64TensorInput::row(inputs.encoder_attention_mask()),
            decoder_input_ids: OrtI64TensorInput::row(inputs.decoder_input_ids()),
        }
    }

    /// { inputs contains stable encoder-side tensors and state contains current decoder ids }
    /// fn from_generation_state(inputs: &OrtGenerationInputs, state: &OrtGenerationState) -> Self
    /// { ret preserves encoder tensors and uses the state's current decoder input row }
    pub fn from_generation_state(inputs: &OrtGenerationInputs, state: &OrtGenerationState) -> Self {
        Self {
            encoder_input_ids: OrtI64TensorInput::row(inputs.encoder_input_ids()),
            encoder_attention_mask: OrtI64TensorInput::row(inputs.encoder_attention_mask()),
            decoder_input_ids: OrtI64TensorInput::row(state.decoder_input_ids()),
        }
    }

    /// { true }
    /// fn encoder_input_ids(&self) -> &OrtI64TensorInput
    /// { ret is the encoder token-id tensor payload }
    pub const fn encoder_input_ids(&self) -> &OrtI64TensorInput {
        &self.encoder_input_ids
    }

    /// { true }
    /// fn encoder_attention_mask(&self) -> &OrtI64TensorInput
    /// { ret is the encoder attention-mask tensor payload }
    pub const fn encoder_attention_mask(&self) -> &OrtI64TensorInput {
        &self.encoder_attention_mask
    }

    /// { true }
    /// fn decoder_input_ids(&self) -> &OrtI64TensorInput
    /// { ret is the decoder input-id tensor payload }
    pub const fn decoder_input_ids(&self) -> &OrtI64TensorInput {
        &self.decoder_input_ids
    }
}

/// Owned `f32` tensor output copied out of an ONNX Runtime session.
#[derive(Clone, Debug, PartialEq)]
pub struct OrtFloatTensorOutput {
    shape: Vec<usize>,
    values: Vec<f32>,
}

impl OrtFloatTensorOutput {
    /// { true }
    /// fn shape(&self) -> &[usize]
    /// { ret is the concrete tensor output shape }
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// { true }
    /// fn values(&self) -> &[f32]
    /// { ret is the contiguous row-major tensor output payload }
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    #[cfg(feature = "ort-runtime")]
    fn from_ort_tensor(
        output_name: &str,
        shape: &[i64],
        values: &[f32],
    ) -> Result<Self, OrtEngineError> {
        let shape = ort_output_shape_to_usize(output_name, shape)?;

        Ok(Self {
            shape,
            values: values.to_vec(),
        })
    }
}

/// Decoder logits shape interpretation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrtDecoderLogitsError {
    /// Decoder logits rank is not `[batch, sequence, vocabulary]`.
    InvalidRank {
        /// Actual rank.
        rank: usize,
    },
    /// Decoder logits batch is not the supported single-row batch.
    InvalidBatch {
        /// Actual batch size.
        batch: usize,
    },
    /// Decoder logits contain no sequence positions.
    EmptySequence,
    /// Decoder logits contain no vocabulary scores.
    EmptyVocabulary,
    /// Shape multiplication overflowed usize.
    ElementCountOverflow,
    /// Shape element count and payload length do not match.
    ElementCountMismatch {
        /// Expected element count from shape.
        expected: usize,
        /// Actual payload element count.
        actual: usize,
    },
    /// Next-token selection failed after logits extraction.
    Selection(OrtNextTokenSelectionError),
}

impl fmt::Display for OrtDecoderLogitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRank { rank } => {
                write!(formatter, "decoder logits rank must be 3, got {rank}")
            }
            Self::InvalidBatch { batch } => {
                write!(formatter, "decoder logits batch must be 1, got {batch}")
            }
            Self::EmptySequence => formatter.write_str("decoder logits sequence is empty"),
            Self::EmptyVocabulary => formatter.write_str("decoder logits vocabulary is empty"),
            Self::ElementCountOverflow => {
                formatter.write_str("decoder logits shape element count overflowed")
            }
            Self::ElementCountMismatch { expected, actual } => write!(
                formatter,
                "decoder logits expected {expected} values, got {actual}"
            ),
            Self::Selection(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for OrtDecoderLogitsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Selection(error) => Some(error),
            _ => None,
        }
    }
}

/// Shape-aware decoder logits helper.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrtDecoderLogits;

impl OrtDecoderLogits {
    /// { output is a copied decoder logits tensor candidate }
    /// fn final_token_logits(output: &OrtFloatTensorOutput) -> Result<&[f32], OrtDecoderLogitsError>
    /// { ret is Ok only for the final sequence-position vocabulary row }
    pub fn final_token_logits(
        output: &OrtFloatTensorOutput,
    ) -> Result<&[f32], OrtDecoderLogitsError> {
        let shape = output.shape();
        if shape.len() != 3 {
            return Err(OrtDecoderLogitsError::InvalidRank { rank: shape.len() });
        }

        let batch = shape[0];
        let sequence = shape[1];
        let vocabulary = shape[2];
        validate_decoder_logits_shape(batch, sequence, vocabulary, output.values().len())?;
        let start = (sequence - 1)
            .checked_mul(vocabulary)
            .ok_or(OrtDecoderLogitsError::ElementCountOverflow)?;
        let end = start
            .checked_add(vocabulary)
            .ok_or(OrtDecoderLogitsError::ElementCountOverflow)?;

        output
            .values()
            .get(start..end)
            .ok_or(OrtDecoderLogitsError::ElementCountMismatch {
                expected: end,
                actual: output.values().len(),
            })
    }

    /// { output is a copied decoder logits tensor candidate }
    /// fn select_next_token(output: &OrtFloatTensorOutput) -> Result<TokenId, OrtDecoderLogitsError>
    /// { ret is Ok only when the final logits row yields a valid next token id }
    pub fn select_next_token(
        output: &OrtFloatTensorOutput,
    ) -> Result<TokenId, OrtDecoderLogitsError> {
        OrtNextTokenSelector::select_argmax(Self::final_token_logits(output)?)
            .map_err(OrtDecoderLogitsError::Selection)
    }
}

fn validate_decoder_logits_shape(
    batch: usize,
    sequence: usize,
    vocabulary: usize,
    actual_values: usize,
) -> Result<(), OrtDecoderLogitsError> {
    if batch != 1 {
        return Err(OrtDecoderLogitsError::InvalidBatch { batch });
    }
    if sequence == 0 {
        return Err(OrtDecoderLogitsError::EmptySequence);
    }
    if vocabulary == 0 {
        return Err(OrtDecoderLogitsError::EmptyVocabulary);
    }

    let expected = batch
        .checked_mul(sequence)
        .and_then(|count| count.checked_mul(vocabulary))
        .ok_or(OrtDecoderLogitsError::ElementCountOverflow)?;
    if expected != actual_values {
        return Err(OrtDecoderLogitsError::ElementCountMismatch {
            expected,
            actual: actual_values,
        });
    }

    Ok(())
}

/// { tokens is a non-empty bounded tokenizer sequence }
/// fn token_sequence_to_i64(tokens: &TokenSequence) -> Vec<i64>
/// { ret preserves token order while converting ids for ONNX tensors }
fn token_sequence_to_i64(tokens: &TokenSequence) -> Vec<i64> {
    tokens
        .as_slice()
        .iter()
        .map(|token| i64::from(token.value()))
        .collect()
}

/// Decoder-loop state error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtGenerationStateError {
    /// The decoder loop already reached EOS or the max-new-token limit.
    AlreadyFinished,
}

impl fmt::Display for OrtGenerationStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyFinished => {
                formatter.write_str("ORT generation state is already finished")
            }
        }
    }
}

impl std::error::Error for OrtGenerationStateError {}

/// Deterministic decoder-loop state before ORT execution is wired.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGenerationState {
    decoder_input_ids: Vec<i64>,
    generated_token_ids: Vec<TokenId>,
    max_new_tokens: usize,
    eos_token_id: i64,
    finished: bool,
}

impl OrtGenerationState {
    /// { inputs were prepared from tokenizer output and generation config }
    /// fn new(inputs: OrtGenerationInputs) -> Self
    /// { ret starts a decoder loop with the configured decoder seed }
    pub fn new(inputs: OrtGenerationInputs) -> Self {
        Self {
            decoder_input_ids: inputs.decoder_input_ids,
            generated_token_ids: Vec::new(),
            max_new_tokens: inputs.max_new_tokens,
            eos_token_id: inputs.eos_token_id,
            finished: false,
        }
    }

    /// { true }
    /// fn decoder_input_ids(&self) -> &[i64]
    /// { ret is the decoder seed followed by accepted generated token ids }
    pub fn decoder_input_ids(&self) -> &[i64] {
        &self.decoder_input_ids
    }

    /// { true }
    /// fn generated_token_ids(&self) -> &[TokenId]
    /// { ret is the accepted generated target token ids }
    pub fn generated_token_ids(&self) -> &[TokenId] {
        &self.generated_token_ids
    }

    /// { true }
    /// fn is_finished(&self) -> bool
    /// { ret is true only after EOS or max-new-token limit has been reached }
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// { token is the next selected decoder token }
    /// fn accept_next_token(&mut self, token: TokenId) -> Result<(), OrtGenerationStateError>
    /// { ret is Ok only when token was appended to unfinished state }
    pub fn accept_next_token(&mut self, token: TokenId) -> Result<(), OrtGenerationStateError> {
        if self.finished {
            return Err(OrtGenerationStateError::AlreadyFinished);
        }

        let token_id = i64::from(token.value());
        self.generated_token_ids.push(token);
        self.decoder_input_ids.push(token_id);
        self.finished =
            token_id == self.eos_token_id || self.generated_token_ids.len() >= self.max_new_tokens;

        Ok(())
    }
}

/// ORT decoder-output generation-step error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrtGenerationStepError {
    /// Decoder logits could not produce a valid next token.
    DecoderLogits(OrtDecoderLogitsError),
    /// Generation state rejected the selected token.
    State(OrtGenerationStateError),
}

impl fmt::Display for OrtGenerationStepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecoderLogits(error) => write!(formatter, "{error}"),
            Self::State(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for OrtGenerationStepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DecoderLogits(error) => Some(error),
            Self::State(error) => Some(error),
        }
    }
}

/// Pure ORT decoder-output application step.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrtGenerationStep;

impl OrtGenerationStep {
    /// { state is the current decoder-loop state and output is copied decoder logits }
    /// fn accept_decoder_output(state: &mut OrtGenerationState, output: &OrtFloatTensorOutput) -> Result<TokenId, OrtGenerationStepError>
    /// { ret is Ok only when the selected token has been appended to state }
    pub fn accept_decoder_output(
        state: &mut OrtGenerationState,
        output: &OrtFloatTensorOutput,
    ) -> Result<TokenId, OrtGenerationStepError> {
        let token = OrtDecoderLogits::select_next_token(output)
            .map_err(OrtGenerationStepError::DecoderLogits)?;
        state
            .accept_next_token(token)
            .map_err(OrtGenerationStepError::State)?;

        Ok(token)
    }
}

/// ORT decoder loop error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrtGenerationLoopError {
    /// Decoder execution failed.
    Decoder(String),
    /// Decoder output could not be applied to generation state.
    Step(OrtGenerationStepError),
    /// Generated token ids could not be converted into a bounded sequence.
    InvalidGeneratedTokens(String),
}

impl fmt::Display for OrtGenerationLoopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decoder(reason) => write!(formatter, "ORT decoder execution failed: {reason}"),
            Self::Step(error) => write!(formatter, "ORT generation step failed: {error}"),
            Self::InvalidGeneratedTokens(reason) => {
                write!(formatter, "invalid ORT generated tokens: {reason}")
            }
        }
    }
}

impl std::error::Error for OrtGenerationLoopError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Step(error) => Some(error),
            _ => None,
        }
    }
}

/// Non-cached ORT decoder-loop driver.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrtGenerationLoop;

impl OrtGenerationLoop {
    /// { inputs contains validated generation vectors and decode_next runs one decoder step }
    /// fn run_non_cached(inputs: &OrtGenerationInputs, decode_next: F) -> Result<TokenSequence, OrtGenerationLoopError>
    /// { ret is Ok only when decoder steps produce a non-empty bounded generated-token sequence }
    pub fn run_non_cached<F>(
        inputs: &OrtGenerationInputs,
        mut decode_next: F,
    ) -> Result<TokenSequence, OrtGenerationLoopError>
    where
        F: FnMut(
            &OrtGenerationTensorInputs,
        ) -> Result<OrtFloatTensorOutput, OrtGenerationLoopError>,
    {
        let mut state = OrtGenerationState::new(inputs.clone());
        while !state.is_finished() {
            let tensor_inputs = OrtGenerationTensorInputs::from_generation_state(inputs, &state);
            let output = decode_next(&tensor_inputs)?;
            OrtGenerationStep::accept_decoder_output(&mut state, &output)
                .map_err(OrtGenerationLoopError::Step)?;
        }

        TokenSequence::new(state.generated_token_ids().to_vec())
            .map_err(|error| OrtGenerationLoopError::InvalidGeneratedTokens(error.to_string()))
    }
}

/// Next-token selection error for decoder logits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtNextTokenSelectionError {
    /// Decoder output did not contain any vocabulary logits.
    EmptyLogits,
    /// A decoder logit was NaN or infinite.
    NonFiniteLogit {
        /// Position of the invalid logit.
        index: usize,
    },
    /// Selected vocabulary index cannot fit the token-id type.
    VocabularyIndexTooLarge {
        /// Selected vocabulary index.
        index: usize,
    },
}

impl fmt::Display for OrtNextTokenSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLogits => formatter.write_str("decoder logits must not be empty"),
            Self::NonFiniteLogit { index } => {
                write!(formatter, "decoder logit at index {index} is not finite")
            }
            Self::VocabularyIndexTooLarge { index } => {
                write!(formatter, "vocabulary index {index} does not fit token id")
            }
        }
    }
}

impl std::error::Error for OrtNextTokenSelectionError {}

/// Deterministic next-token selector for decoder logits.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OrtNextTokenSelector;

impl OrtNextTokenSelector {
    /// { logits is one decoder vocabulary score row }
    /// fn select_argmax(logits: &[f32]) -> Result<TokenId, OrtNextTokenSelectionError>
    /// { ret is the first token id with the highest finite score }
    pub fn select_argmax(logits: &[f32]) -> Result<TokenId, OrtNextTokenSelectionError> {
        let mut best_index = None;
        let mut best_score = f32::NEG_INFINITY;

        for (index, score) in logits.iter().copied().enumerate() {
            if !score.is_finite() {
                return Err(OrtNextTokenSelectionError::NonFiniteLogit { index });
            }

            if best_index.is_none() || score > best_score {
                best_index = Some(index);
                best_score = score;
            }
        }

        let index = best_index.ok_or(OrtNextTokenSelectionError::EmptyLogits)?;
        let token_id = u32::try_from(index)
            .map_err(|_error| OrtNextTokenSelectionError::VocabularyIndexTooLarge { index })?;

        Ok(TokenId::new(token_id))
    }
}

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

/// Session-level CPU threading policy for ONNX Runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrtSessionThreadingPolicy {
    ort_intra_threads: usize,
    ort_inter_threads: usize,
    xnnpack_threads: Option<NonZeroUsize>,
}

impl OrtSessionThreadingPolicy {
    /// { available_parallelism is the runtime CPU parallelism hint and xnnpack_available reflects ORT EP availability }
    /// fn for_available_parallelism(available_parallelism: usize, xnnpack_available: bool) -> Self
    /// { ret uses XNNPACK's threadpool when available and otherwise caps ORT CPU threads for mobile latency }
    pub fn for_available_parallelism(
        available_parallelism: usize,
        xnnpack_available: bool,
    ) -> Self {
        let runtime_threads = capped_ort_threads(available_parallelism);
        if xnnpack_available {
            return Self {
                ort_intra_threads: 1,
                ort_inter_threads: 1,
                xnnpack_threads: NonZeroUsize::new(runtime_threads),
            };
        }

        Self {
            ort_intra_threads: runtime_threads,
            ort_inter_threads: 1,
            xnnpack_threads: None,
        }
    }

    /// { true }
    /// fn ort_intra_threads(&self) -> usize
    /// { ret is the ORT intra-op thread count configured on every session }
    pub const fn ort_intra_threads(&self) -> usize {
        self.ort_intra_threads
    }

    /// { true }
    /// fn ort_inter_threads(&self) -> usize
    /// { ret is the ORT inter-op thread count configured on every session }
    pub const fn ort_inter_threads(&self) -> usize {
        self.ort_inter_threads
    }

    /// { true }
    /// fn xnnpack_threads(&self) -> `Option<NonZeroUsize>`
    /// { ret is Some only when XNNPACK should own the operator threadpool }
    pub const fn xnnpack_threads(&self) -> Option<NonZeroUsize> {
        self.xnnpack_threads
    }

    #[cfg(feature = "ort-runtime")]
    fn for_current_runtime() -> Self {
        Self::for_available_parallelism(available_parallelism(), xnnpack_is_available())
    }
}

/// { available_parallelism is a process CPU parallelism hint }
/// fn capped_ort_threads(available_parallelism: usize) -> usize
/// { ret is in 1..=ORT_THREAD_CAP }
fn capped_ort_threads(available_parallelism: usize) -> usize {
    available_parallelism.clamp(1, ORT_THREAD_CAP)
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

    /// { self was built from a verified ONNX Runtime model pack }
    /// fn parse_runtime_config(&self) -> Result<OrtGeneratorRuntimeConfig, OrtEngineError>
    /// { ret is Ok only when generation_config and ort_io config are both present and valid }
    pub fn parse_runtime_config(&self) -> Result<OrtGeneratorRuntimeConfig, OrtEngineError> {
        let generation_config = self
            .parse_generation_config()?
            .ok_or(OrtEngineError::MissingGenerationConfig)?;
        let ort_io_config = match self.parse_ort_io_config() {
            Ok(Some(config)) => config,
            Ok(None) => return Err(OrtEngineError::MissingOrtIoConfig),
            Err(OrtEngineError::IoConfig(OrtIoConfigParseError::MissingOrtIoConfig)) => {
                return Err(OrtEngineError::MissingOrtIoConfig);
            }
            Err(error) => return Err(error),
        };

        Ok(OrtGeneratorRuntimeConfig::new(
            generation_config,
            ort_io_config,
        ))
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
    /// Runtime generation requires generation_config but the pack lacks one.
    MissingGenerationConfig,
    /// Runtime generation requires ort_io config but the pack lacks one.
    MissingOrtIoConfig,
    /// ORT session did not return an expected named output.
    MissingOrtOutput(String),
    /// ORT session returned an output shape that cannot become a concrete Rust shape.
    InvalidOrtOutputShape {
        /// Output tensor name.
        output: String,
        /// Invalid output dimension.
        dimension: i64,
    },
    /// A mutable ORT session lock was poisoned.
    OrtSessionLock(OrtModelRole),
    /// Parallel ORT session loading failed while joining a load thread.
    OrtSessionLoadThread(OrtModelRole),
    /// Crate was compiled without the `ort-runtime` feature.
    OrtRuntimeFeatureDisabled,
    /// Dynamic ONNX Runtime loading requires an explicit dylib path.
    MissingOrtDylibPath,
    /// Dynamic ONNX Runtime dylib path is not usable.
    InvalidOrtDylibPath {
        /// Path configured through ORT_DYLIB_PATH.
        path: PathBuf,
        /// Stable validation reason.
        reason: String,
    },
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
            Self::MissingGenerationConfig => {
                formatter.write_str("missing generation_config for ORT runtime")
            }
            Self::MissingOrtIoConfig => {
                formatter.write_str("missing ort_io config for ORT runtime")
            }
            Self::MissingOrtOutput(output) => write!(formatter, "missing ORT output: {output}"),
            Self::InvalidOrtOutputShape { output, dimension } => write!(
                formatter,
                "ORT output {output} has invalid shape dimension {dimension}"
            ),
            Self::OrtSessionLock(role) => write!(formatter, "ORT session lock poisoned: {role}"),
            Self::OrtSessionLoadThread(role) => {
                write!(formatter, "ORT session load thread failed: {role}")
            }
            Self::OrtRuntimeFeatureDisabled => {
                formatter.write_str("ort-runtime feature is not enabled")
            }
            Self::MissingOrtDylibPath => {
                write!(
                    formatter,
                    "{ORT_DYLIB_PATH_ENV} must point to libonnxruntime"
                )
            }
            Self::InvalidOrtDylibPath { path, reason } => write!(
                formatter,
                "{ORT_DYLIB_PATH_ENV} {} is invalid: {reason}",
                path.display()
            ),
            Self::Ort(error) => write!(formatter, "ONNX Runtime error: {error}"),
        }
    }
}

impl std::error::Error for OrtEngineError {}

/// { path is a candidate ONNX Runtime dynamic library path }
/// `fn configure_ort_dylib_path(path: impl Into<PathBuf>) -> Result<(), OrtEngineError>`
/// { ret is Ok only when future ORT session loads may use path as the explicit dylib }
#[cfg(feature = "ort-runtime")]
pub fn configure_ort_dylib_path(path: impl Into<PathBuf>) -> Result<(), OrtEngineError> {
    let path = validate_ort_dylib_path(path.into())?;

    match ORT_DYLIB_PATH_OVERRIDE.set(path) {
        Ok(()) => Ok(()),
        Err(path) => match ORT_DYLIB_PATH_OVERRIDE.get() {
            Some(existing) if existing == &path => Ok(()),
            Some(existing) => Err(OrtEngineError::InvalidOrtDylibPath {
                path,
                reason: format!("runtime path already configured as {}", existing.display()),
            }),
            None => Err(OrtEngineError::InvalidOrtDylibPath {
                path,
                reason: "runtime path could not be configured".to_owned(),
            }),
        },
    }
}

/// Thread-safe mutable slot for a loaded ONNX Runtime session.
#[cfg(feature = "ort-runtime")]
#[derive(Debug)]
pub struct OrtEngineSlot {
    role: OrtModelRole,
    engine: std::sync::Mutex<OrtEngine>,
}

#[cfg(feature = "ort-runtime")]
impl OrtEngineSlot {
    /// { engine is a loaded ORT session wrapper }
    /// fn new(engine: OrtEngine) -> Self
    /// { ret owns engine behind a mutable session lock and preserves its role }
    pub fn new(engine: OrtEngine) -> Self {
        let role = engine.plan().role();

        Self {
            role,
            engine: std::sync::Mutex::new(engine),
        }
    }

    /// { true }
    /// fn role(&self) -> OrtModelRole
    /// { ret is the ORT model role held by this slot }
    pub const fn role(&self) -> OrtModelRole {
        self.role
    }

    /// { self owns a loaded ORT session }
    /// `fn with_mut<T>(&self, operation: impl FnOnce(&mut OrtEngine) -> Result<T, OrtEngineError>) -> Result<T, OrtEngineError>`
    /// { ret is operation result only when the mutable session lock is available }
    pub fn with_mut<T>(
        &self,
        operation: impl FnOnce(&mut OrtEngine) -> Result<T, OrtEngineError>,
    ) -> Result<T, OrtEngineError> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|_error| OrtEngineError::OrtSessionLock(self.role))?;

        operation(&mut engine)
    }

    /// { self owns a loaded ORT session }
    /// fn input_count(&self) -> Result<usize, OrtEngineError>
    /// { ret is the number of ONNX graph inputs when the session lock is available }
    pub fn input_count(&self) -> Result<usize, OrtEngineError> {
        self.with_mut(|engine| Ok(engine.input_count()))
    }

    /// { self owns a loaded ORT session }
    /// fn output_count(&self) -> Result<usize, OrtEngineError>
    /// { ret is the number of ONNX graph outputs when the session lock is available }
    pub fn output_count(&self) -> Result<usize, OrtEngineError> {
        self.with_mut(|engine| Ok(engine.output_count()))
    }
}

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
    runtime_config: OrtGeneratorRuntimeConfig,
    encoder: OrtEngineSlot,
    decoder: OrtEngineSlot,
    decoder_with_past: Option<OrtEngineSlot>,
}

#[cfg(feature = "ort-runtime")]
impl OrtTokenGenerator {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when ONNX Runtime loads required generator sessions }
    pub fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError> {
        let runtime_config = plan.parse_runtime_config()?;
        let (encoder, decoder) = load_required_generator_sessions(&plan)?;
        let decoder_with_past = plan
            .decoder_with_past()
            .cloned()
            .map(OrtEngine::load)
            .transpose()?
            .map(OrtEngineSlot::new);

        Ok(Self {
            plan,
            runtime_config,
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
    /// fn runtime_config(&self) -> &OrtGeneratorRuntimeConfig
    /// { ret is the strict generation config parsed before ORT sessions loaded }
    pub const fn runtime_config(&self) -> &OrtGeneratorRuntimeConfig {
        &self.runtime_config
    }

    /// { self was loaded successfully }
    /// fn encoder(&self) -> &OrtEngineSlot
    /// { ret is the loaded encoder session slot }
    pub const fn encoder(&self) -> &OrtEngineSlot {
        &self.encoder
    }

    /// { self was loaded successfully }
    /// fn decoder(&self) -> &OrtEngineSlot
    /// { ret is the loaded decoder session slot }
    pub const fn decoder(&self) -> &OrtEngineSlot {
        &self.decoder
    }

    /// { self was loaded successfully }
    /// fn decoder_with_past(&self) -> `Option<&OrtEngineSlot>`
    /// { ret is Some only when the cached decoder session slot was loaded }
    pub const fn decoder_with_past(&self) -> Option<&OrtEngineSlot> {
        self.decoder_with_past.as_ref()
    }

    /// { self was loaded successfully and input contains validated source tokens }
    /// fn run_encoder_for_input(&self, input: &TokenizerOutput) -> Result<OrtFloatTensorOutput, OrtEngineError>
    /// { ret is Ok only when the configured encoder session returns last_hidden_state }
    fn run_encoder_for_input(
        &self,
        input: &TokenizerOutput,
    ) -> Result<OrtFloatTensorOutput, OrtEngineError> {
        let generation_inputs = OrtGenerationInputs::from_tokenizer_output(
            input,
            self.runtime_config.generation_config(),
        );
        let tensor_inputs = OrtGenerationTensorInputs::from_generation_inputs(&generation_inputs);

        self.encoder.with_mut(|engine| {
            engine.run_encoder(
                self.runtime_config.ort_io_config().encoder(),
                &tensor_inputs,
            )
        })
    }
}

#[cfg(not(feature = "ort-runtime"))]
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

#[cfg(feature = "ort-runtime")]
impl TokenGenerator for OrtTokenGenerator {
    /// { input contains validated source tokens and target language }
    /// fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError>
    /// { ret is Ok only when ORT encoder and non-cached decoder execution produce generated tokens }
    fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
        let generation_inputs = OrtGenerationInputs::from_tokenizer_output(
            input,
            self.runtime_config.generation_config(),
        );
        let encoder_output = self.run_encoder_for_input(input).map_err(|error| {
            TokenGeneratorError::BackendUnavailable(format!(
                "ORT encoder execution failed: {error}"
            ))
        })?;
        let decoder_names = self.runtime_config.ort_io_config().decoder();

        OrtGenerationLoop::run_non_cached(&generation_inputs, |tensor_inputs| {
            self.decoder
                .with_mut(|engine| {
                    engine.run_decoder(decoder_names, tensor_inputs, &encoder_output)
                })
                .map_err(|error| OrtGenerationLoopError::Decoder(error.to_string()))
        })
        .map_err(map_ort_generation_loop_error)
    }
}

/// { error came from the ORT decoder loop }
/// fn map_ort_generation_loop_error(error: OrtGenerationLoopError) -> TokenGeneratorError
/// { ret preserves invalid token errors and reports runtime failures as backend unavailability }
#[cfg(feature = "ort-runtime")]
fn map_ort_generation_loop_error(error: OrtGenerationLoopError) -> TokenGeneratorError {
    match error {
        OrtGenerationLoopError::Decoder(reason) => TokenGeneratorError::BackendUnavailable(
            format!("ORT decoder execution failed: {reason}"),
        ),
        OrtGenerationLoopError::Step(error) => TokenGeneratorError::BackendUnavailable(format!(
            "ORT decoder generation step failed: {error}"
        )),
        OrtGenerationLoopError::InvalidGeneratedTokens(reason) => {
            TokenGeneratorError::InvalidGeneratedTokens(reason)
        }
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
        prepare_ort_runtime()?;
        let policy = OrtSessionThreadingPolicy::for_current_runtime();

        Self::load_after_runtime_prepared(plan, policy)
    }

    fn load_after_runtime_prepared(
        plan: OrtSessionPlan,
        policy: OrtSessionThreadingPolicy,
    ) -> Result<Self, OrtEngineError> {
        let session = load_ort_session(plan.model_path(), policy)?;

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

    /// { self was loaded successfully and names match the encoder graph contract }
    /// fn run_encoder(&mut self, names: &OrtEncoderIoNames, inputs: &OrtGenerationTensorInputs) -> Result<OrtFloatTensorOutput, OrtEngineError>
    /// { ret is Ok only when encoder execution returns named f32 last_hidden_state }
    pub fn run_encoder(
        &mut self,
        names: &OrtEncoderIoNames,
        inputs: &OrtGenerationTensorInputs,
    ) -> Result<OrtFloatTensorOutput, OrtEngineError> {
        let input_ids = ort_i64_tensor(inputs.encoder_input_ids())?;
        let attention_mask = ort_i64_tensor(inputs.encoder_attention_mask())?;
        let outputs = self
            .session
            .run(ort::inputs! {
                names.input_ids() => input_ids,
                names.attention_mask() => attention_mask,
            })
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;
        let output = outputs.get(names.last_hidden_state()).ok_or_else(|| {
            OrtEngineError::MissingOrtOutput(names.last_hidden_state().to_owned())
        })?;
        let (shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;

        OrtFloatTensorOutput::from_ort_tensor(names.last_hidden_state(), shape, values)
    }

    /// { self was loaded successfully and names match the decoder graph contract }
    /// fn run_decoder(&mut self, names: &OrtDecoderIoNames, inputs: &OrtGenerationTensorInputs, encoder_output: &OrtFloatTensorOutput) -> Result<OrtFloatTensorOutput, OrtEngineError>
    /// { ret is Ok only when decoder execution returns named f32 logits }
    pub fn run_decoder(
        &mut self,
        names: &OrtDecoderIoNames,
        inputs: &OrtGenerationTensorInputs,
        encoder_output: &OrtFloatTensorOutput,
    ) -> Result<OrtFloatTensorOutput, OrtEngineError> {
        let input_ids = ort_i64_tensor(inputs.decoder_input_ids())?;
        let encoder_attention_mask = ort_i64_tensor(inputs.encoder_attention_mask())?;
        let encoder_hidden_states = ort_f32_tensor(encoder_output)?;
        let outputs = self
            .session
            .run(ort::inputs! {
                names.input_ids() => input_ids,
                names.encoder_attention_mask() => encoder_attention_mask,
                names.encoder_hidden_states() => encoder_hidden_states,
            })
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;
        let output = outputs
            .get(names.logits())
            .ok_or_else(|| OrtEngineError::MissingOrtOutput(names.logits().to_owned()))?;
        let (shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;

        OrtFloatTensorOutput::from_ort_tensor(names.logits(), shape, values)
    }
}

/// { plan was built from a verified ONNX Runtime model pack }
/// fn load_required_generator_sessions(plan: &OrtGeneratorPlan) -> Result<(OrtEngineSlot, OrtEngineSlot), OrtEngineError>
/// { ret is Ok only when required encoder and decoder ORT sessions load successfully }
#[cfg(feature = "ort-runtime")]
fn load_required_generator_sessions(
    plan: &OrtGeneratorPlan,
) -> Result<(OrtEngineSlot, OrtEngineSlot), OrtEngineError> {
    prepare_ort_runtime()?;
    let policy = OrtSessionThreadingPolicy::for_current_runtime();

    let encoder_plan = plan.encoder().clone();
    let decoder_plan = plan.decoder().clone();

    std::thread::scope(|scope| {
        let encoder_thread =
            scope.spawn(move || OrtEngine::load_after_runtime_prepared(encoder_plan, policy));
        let decoder_thread =
            scope.spawn(move || OrtEngine::load_after_runtime_prepared(decoder_plan, policy));

        let encoder = join_ort_session_load(encoder_thread, OrtModelRole::Encoder);
        let decoder = join_ort_session_load(decoder_thread, OrtModelRole::Decoder);

        Ok((encoder?, decoder?))
    })
}

/// { thread is a scoped ORT session load handle }
/// fn join_ort_session_load(thread: ScopedJoinHandle<Result<OrtEngine, OrtEngineError>>, role: OrtModelRole) -> Result<OrtEngineSlot, OrtEngineError>
/// { ret is Ok only when the load thread joined and produced a loaded session }
#[cfg(feature = "ort-runtime")]
fn join_ort_session_load(
    thread: std::thread::ScopedJoinHandle<'_, Result<OrtEngine, OrtEngineError>>,
    role: OrtModelRole,
) -> Result<OrtEngineSlot, OrtEngineError> {
    thread
        .join()
        .map_err(|_error| OrtEngineError::OrtSessionLoadThread(role))?
        .map(OrtEngineSlot::new)
}

/// { ONNX Runtime dynamic library was initialized and model_path points to an ONNX graph }
/// fn load_ort_session(model_path: &Path, policy: OrtSessionThreadingPolicy) -> Result<ort::session::Session, OrtEngineError>
/// { ret is Ok only when ORT creates a session with the localmt mobile CPU policy }
#[cfg(feature = "ort-runtime")]
fn load_ort_session(
    model_path: &Path,
    policy: OrtSessionThreadingPolicy,
) -> Result<ort::session::Session, OrtEngineError> {
    let mut builder = ort::session::Session::builder()
        .map_err(|source| OrtEngineError::Ort(source.to_string()))?
        .with_inter_threads(policy.ort_inter_threads())
        .map_err(|source| OrtEngineError::Ort(source.to_string()))?
        .with_intra_threads(policy.ort_intra_threads())
        .map_err(|source| OrtEngineError::Ort(source.to_string()))?
        .with_intra_op_spinning(false)
        .map_err(|source| OrtEngineError::Ort(source.to_string()))?;

    if let Some(threads) = policy.xnnpack_threads() {
        builder = builder
            .with_execution_providers([ort::ep::XNNPACK::default()
                .with_intra_op_num_threads(threads)
                .build()
                .fail_silently()])
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;
    }

    builder
        .commit_from_file(model_path)
        .map_err(|source| OrtEngineError::Ort(source.to_string()))
}

/// { ORT_DYLIB_PATH may or may not be set in the process environment }
/// fn prepare_ort_runtime() -> Result<(), OrtEngineError>
/// { ret is Ok only when ONNX Runtime dynamic loading has an explicit dylib path }
#[cfg(feature = "ort-runtime")]
fn prepare_ort_runtime() -> Result<(), OrtEngineError> {
    let dylib_path = resolve_ort_dylib_path()?;
    let builder = ort::init_from(&dylib_path).map_err(|source| {
        OrtEngineError::Ort(format!(
            "failed to load {ORT_DYLIB_PATH_ENV} {}: {source}",
            dylib_path.display()
        ))
    })?;
    let _ = builder.commit();

    Ok(())
}

/// { true }
/// fn available_parallelism() -> usize
/// { ret is the process CPU parallelism hint, or 1 when the platform cannot report it }
#[cfg(feature = "ort-runtime")]
fn available_parallelism() -> usize {
    std::thread::available_parallelism().map_or(1, NonZeroUsize::get)
}

/// { ONNX Runtime dynamic library was initialized }
/// fn xnnpack_is_available() -> bool
/// { ret is true only when the loaded ORT runtime advertises the XNNPACK execution provider }
#[cfg(feature = "ort-runtime")]
fn xnnpack_is_available() -> bool {
    use ort::ep::ExecutionProvider;

    ort::ep::XNNPACK::default().is_available().unwrap_or(false)
}

/// { ORT runtime path may be configured through FFI or ORT_DYLIB_PATH }
/// fn resolve_ort_dylib_path() -> Result<PathBuf, OrtEngineError>
/// { ret is Ok only when an explicit configured or environment dylib path is available }
#[cfg(feature = "ort-runtime")]
fn resolve_ort_dylib_path() -> Result<PathBuf, OrtEngineError> {
    if let Some(path) = ORT_DYLIB_PATH_OVERRIDE.get() {
        return Ok(path.clone());
    }

    resolve_ort_dylib_path_from_env_value(std::env::var_os(ORT_DYLIB_PATH_ENV))
}

/// { value is a raw ORT_DYLIB_PATH environment value }
/// fn resolve_ort_dylib_path_from_env_value(value: Option<OsString>) -> Result<PathBuf, OrtEngineError>
/// { ret is Ok only when value is a non-empty absolute path to a file }
#[cfg(feature = "ort-runtime")]
fn resolve_ort_dylib_path_from_env_value(
    value: Option<OsString>,
) -> Result<PathBuf, OrtEngineError> {
    let Some(value) = value.filter(|candidate| !candidate.is_empty()) else {
        return Err(OrtEngineError::MissingOrtDylibPath);
    };
    let path = PathBuf::from(value);

    validate_ort_dylib_path(path)
}

/// { path is a candidate ONNX Runtime dynamic library path }
/// fn validate_ort_dylib_path(path: PathBuf) -> Result<PathBuf, OrtEngineError>
/// { ret is Ok only when path is absolute and points to a file }
#[cfg(feature = "ort-runtime")]
fn validate_ort_dylib_path(path: PathBuf) -> Result<PathBuf, OrtEngineError> {
    if !path.is_absolute() {
        return Err(OrtEngineError::InvalidOrtDylibPath {
            path,
            reason: "path must be absolute".to_owned(),
        });
    }
    if !path.is_file() {
        return Err(OrtEngineError::InvalidOrtDylibPath {
            path,
            reason: "path does not point to a file".to_owned(),
        });
    }

    Ok(path)
}

#[cfg(feature = "ort-runtime")]
fn ort_i64_tensor(input: &OrtI64TensorInput) -> Result<ort::value::Tensor<i64>, OrtEngineError> {
    ort::value::Tensor::from_array((input.shape(), input.values().to_vec()))
        .map_err(|source| OrtEngineError::Ort(source.to_string()))
}

#[cfg(feature = "ort-runtime")]
fn ort_f32_tensor(input: &OrtFloatTensorOutput) -> Result<ort::value::Tensor<f32>, OrtEngineError> {
    ort::value::Tensor::from_array((input.shape().to_vec(), input.values().to_vec()))
        .map_err(|source| OrtEngineError::Ort(source.to_string()))
}

#[cfg(feature = "ort-runtime")]
fn ort_output_shape_to_usize(
    output_name: &str,
    shape: &[i64],
) -> Result<Vec<usize>, OrtEngineError> {
    shape
        .iter()
        .copied()
        .map(|dimension| {
            usize::try_from(dimension).map_err(|_error| OrtEngineError::InvalidOrtOutputShape {
                output: output_name.to_owned(),
                dimension,
            })
        })
        .collect()
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
    #[cfg(feature = "ort-runtime")]
    use localmt_tokenizer::TokenizerOutput;

    #[cfg(feature = "ort-runtime")]
    use crate::{
        OrtDecoderIoNames, OrtEncoderIoNames, OrtEngine, OrtEngineSlot, OrtTokenGenerator,
        resolve_ort_dylib_path_from_env_value,
    };
    use crate::{
        OrtDecoderLogits, OrtDecoderLogitsError, OrtEngineError, OrtFloatTensorOutput,
        OrtGenerationInputs, OrtGenerationLoop, OrtGenerationLoopError, OrtGenerationState,
        OrtGenerationStateError, OrtGenerationStep, OrtGenerationStepError,
        OrtGenerationTensorInputs, OrtGeneratorPlan, OrtIoConfig, OrtIoConfigError,
        OrtIoConfigParseError, OrtModelRole, OrtNextTokenSelectionError, OrtNextTokenSelector,
        OrtSessionPlan, OrtSessionThreadingPolicy,
    };
    #[cfg(not(feature = "ort-runtime"))]
    use crate::{OrtEngine, OrtTokenGenerator};

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
    fn session_threading_policy_uses_xnnpack_pool_when_available() {
        let policy = OrtSessionThreadingPolicy::for_available_parallelism(8, true);

        assert_eq!(policy.ort_intra_threads(), 1);
        assert_eq!(policy.ort_inter_threads(), 1);
        assert_eq!(
            policy.xnnpack_threads().map(core::num::NonZeroUsize::get),
            Some(4)
        );
    }

    #[test]
    fn session_threading_policy_uses_capped_cpu_pool_without_xnnpack() {
        let policy = OrtSessionThreadingPolicy::for_available_parallelism(8, false);

        assert_eq!(policy.ort_intra_threads(), 4);
        assert_eq!(policy.ort_inter_threads(), 1);
        assert_eq!(policy.xnnpack_threads(), None);
    }

    #[test]
    fn session_threading_policy_keeps_at_least_one_thread() {
        let policy = OrtSessionThreadingPolicy::for_available_parallelism(0, false);

        assert_eq!(policy.ort_intra_threads(), 1);
        assert_eq!(policy.ort_inter_threads(), 1);
        assert_eq!(policy.xnnpack_threads(), None);
    }

    #[test]
    fn ort_engine_error_reports_session_load_thread_role() {
        let error = OrtEngineError::OrtSessionLoadThread(OrtModelRole::Decoder);

        assert_eq!(error.to_string(), "ORT session load thread failed: decoder");
    }

    #[test]
    fn next_token_selector_selects_highest_logit() -> Result<(), Box<dyn std::error::Error>> {
        let token = OrtNextTokenSelector::select_argmax(&[0.1, 2.5, 1.3])?;

        assert_eq!(token, TokenId::new(1));
        Ok(())
    }

    #[test]
    fn next_token_selector_keeps_first_index_on_tie() -> Result<(), Box<dyn std::error::Error>> {
        let token = OrtNextTokenSelector::select_argmax(&[2.0, 2.0, 1.0])?;

        assert_eq!(token, TokenId::new(0));
        Ok(())
    }

    #[test]
    fn next_token_selector_rejects_empty_logits() {
        let token = OrtNextTokenSelector::select_argmax(&[]);

        assert!(matches!(
            token,
            Err(OrtNextTokenSelectionError::EmptyLogits)
        ));
    }

    #[test]
    fn next_token_selector_rejects_non_finite_logits() {
        let token = OrtNextTokenSelector::select_argmax(&[1.0, f32::NAN]);

        assert!(matches!(
            token,
            Err(OrtNextTokenSelectionError::NonFiniteLogit { index: 1 })
        ));
    }

    #[test]
    fn decoder_logits_extract_final_token_row() -> Result<(), Box<dyn std::error::Error>> {
        let output = decoder_logits_output();

        let logits = OrtDecoderLogits::final_token_logits(&output)?;

        assert_eq!(logits, &[4.0, 2.0, 1.0]);
        Ok(())
    }

    #[test]
    fn decoder_logits_select_next_token_from_final_row() -> Result<(), Box<dyn std::error::Error>> {
        let output = decoder_logits_output();

        let token = OrtDecoderLogits::select_next_token(&output)?;

        assert_eq!(token, TokenId::new(0));
        Ok(())
    }

    #[test]
    fn decoder_logits_rejects_non_three_dimensional_output() {
        let output = OrtFloatTensorOutput {
            shape: vec![1, 3],
            values: vec![0.1, 0.2, 0.3],
        };

        let logits = OrtDecoderLogits::final_token_logits(&output);

        assert!(matches!(
            logits,
            Err(OrtDecoderLogitsError::InvalidRank { rank: 2 })
        ));
    }

    #[test]
    fn decoder_logits_rejects_mismatched_element_count() {
        let output = OrtFloatTensorOutput {
            shape: vec![1, 2, 3],
            values: vec![0.1, 0.2, 0.3],
        };

        let logits = OrtDecoderLogits::final_token_logits(&output);

        assert!(matches!(
            logits,
            Err(OrtDecoderLogitsError::ElementCountMismatch {
                expected: 6,
                actual: 3
            })
        ));
    }

    #[test]
    fn generation_step_accepts_decoder_output_into_state() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut state = generation_state(3);
        let output = OrtFloatTensorOutput {
            shape: vec![1, 1, 3],
            values: vec![0.1, 0.2, 5.0],
        };

        let token = OrtGenerationStep::accept_decoder_output(&mut state, &output)?;

        assert_eq!(token, TokenId::new(2));
        assert_eq!(state.generated_token_ids(), &[TokenId::new(2)]);
        assert_eq!(state.decoder_input_ids(), &[11, 2]);
        assert!(!state.is_finished());
        Ok(())
    }

    #[test]
    fn generation_step_marks_state_finished_on_eos() -> Result<(), Box<dyn std::error::Error>> {
        let mut state = generation_state(3);
        let output = OrtFloatTensorOutput {
            shape: vec![1, 1, 3],
            values: vec![0.1, 9.0, 0.2],
        };

        let token = OrtGenerationStep::accept_decoder_output(&mut state, &output)?;

        assert_eq!(token, TokenId::new(1));
        assert_eq!(state.generated_token_ids(), &[TokenId::new(1)]);
        assert!(state.is_finished());
        Ok(())
    }

    #[test]
    fn generation_step_propagates_logits_error_without_state_mutation() {
        let mut state = generation_state(3);
        let before = state.clone();
        let output = OrtFloatTensorOutput {
            shape: vec![1, 3],
            values: vec![0.1, 0.2, 0.3],
        };

        let token = OrtGenerationStep::accept_decoder_output(&mut state, &output);

        assert!(matches!(
            token,
            Err(OrtGenerationStepError::DecoderLogits(
                OrtDecoderLogitsError::InvalidRank { rank: 2 }
            ))
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn generation_step_rejects_finished_state_without_extra_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut state = generation_state(1);
        let output = OrtFloatTensorOutput {
            shape: vec![1, 1, 3],
            values: vec![0.1, 0.2, 5.0],
        };
        OrtGenerationStep::accept_decoder_output(&mut state, &output)?;
        let before = state.clone();

        let token = OrtGenerationStep::accept_decoder_output(&mut state, &output);

        assert!(matches!(
            token,
            Err(OrtGenerationStepError::State(
                OrtGenerationStateError::AlreadyFinished
            ))
        ));
        assert_eq!(state, before);
        Ok(())
    }

    #[test]
    fn generation_tensor_inputs_from_state_match_initial_inputs() {
        let inputs = raw_generation_inputs();
        let state = OrtGenerationState::new(inputs.clone());

        let tensor_inputs = OrtGenerationTensorInputs::from_generation_state(&inputs, &state);

        assert_eq!(tensor_inputs.encoder_input_ids().shape(), [1, 2]);
        assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
        assert_eq!(tensor_inputs.encoder_attention_mask().shape(), [1, 2]);
        assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
        assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 1]);
        assert_eq!(tensor_inputs.decoder_input_ids().values(), &[11]);
    }

    #[test]
    fn generation_tensor_inputs_from_state_follow_decoder_growth()
    -> Result<(), Box<dyn std::error::Error>> {
        let inputs = raw_generation_inputs();
        let mut state = OrtGenerationState::new(inputs.clone());
        state.accept_next_token(TokenId::new(21))?;
        state.accept_next_token(TokenId::new(22))?;

        let tensor_inputs = OrtGenerationTensorInputs::from_generation_state(&inputs, &state);

        assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
        assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
        assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 3]);
        assert_eq!(tensor_inputs.decoder_input_ids().values(), &[11, 21, 22]);
        Ok(())
    }

    #[test]
    fn generation_loop_runs_until_eos() -> Result<(), Box<dyn std::error::Error>> {
        let inputs = raw_generation_inputs();
        let mut calls = 0_usize;
        let mut decoder_rows = Vec::<Vec<i64>>::new();

        let tokens = OrtGenerationLoop::run_non_cached(&inputs, |tensor_inputs| {
            calls += 1;
            decoder_rows.push(tensor_inputs.decoder_input_ids().values().to_vec());
            let output = if calls == 1 {
                decoder_logits([0.1, 0.2, 5.0])
            } else {
                decoder_logits([0.1, 9.0, 0.2])
            };

            Ok(output)
        })?;

        assert_eq!(tokens.as_slice(), &[TokenId::new(2), TokenId::new(1)]);
        assert_eq!(decoder_rows, vec![vec![11], vec![11, 2]]);
        assert_eq!(calls, 2);
        Ok(())
    }

    #[test]
    fn generation_loop_stops_at_max_new_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let inputs = raw_generation_inputs_with_limit(2);
        let mut calls = 0_usize;

        let tokens = OrtGenerationLoop::run_non_cached(&inputs, |_tensor_inputs| {
            calls += 1;
            Ok(decoder_logits([0.1, 0.2, 5.0]))
        })?;

        assert_eq!(tokens.as_slice(), &[TokenId::new(2), TokenId::new(2)]);
        assert_eq!(calls, 2);
        Ok(())
    }

    #[test]
    fn generation_loop_propagates_decoder_error() {
        let inputs = raw_generation_inputs();

        let tokens = OrtGenerationLoop::run_non_cached(&inputs, |_tensor_inputs| {
            Err(OrtGenerationLoopError::Decoder(
                "synthetic decoder failure".to_owned(),
            ))
        });

        assert!(matches!(
            tokens,
            Err(OrtGenerationLoopError::Decoder(ref reason))
                if reason == "synthetic decoder failure"
        ));
    }

    #[test]
    #[ignore = "compile-only signature guard; execution requires a real ONNX encoder model"]
    #[cfg(feature = "ort-runtime")]
    fn encoder_run_method_accepts_tensor_inputs() {
        fn assert_signature(
            engine: &mut OrtEngine,
            names: &OrtEncoderIoNames,
            inputs: &OrtGenerationTensorInputs,
        ) {
            let result: Result<OrtFloatTensorOutput, OrtEngineError> =
                engine.run_encoder(names, inputs);
            let _ = result;
        }

        let _signature: fn(&mut OrtEngine, &OrtEncoderIoNames, &OrtGenerationTensorInputs) =
            assert_signature;
    }

    #[test]
    #[ignore = "compile-only signature guard; construction requires real ONNX model assets"]
    #[cfg(feature = "ort-runtime")]
    fn token_generator_exposes_locked_sessions_for_generate_boundary() {
        fn assert_signature(generator: &OrtTokenGenerator) {
            let _encoder: &OrtEngineSlot = generator.encoder();
            let _decoder: &OrtEngineSlot = generator.decoder();
            let _past: Option<&OrtEngineSlot> = generator.decoder_with_past();
        }

        let _signature: fn(&OrtTokenGenerator) = assert_signature;
    }

    #[test]
    #[ignore = "compile-only signature guard; construction requires real ONNX model assets"]
    #[cfg(feature = "ort-runtime")]
    fn token_generator_encoder_stage_accepts_tokenizer_output() {
        fn assert_signature(generator: &OrtTokenGenerator, input: &TokenizerOutput) {
            let result: Result<OrtFloatTensorOutput, OrtEngineError> =
                generator.run_encoder_for_input(input);
            let _ = result;
        }

        let _signature: fn(&OrtTokenGenerator, &TokenizerOutput) = assert_signature;
    }

    #[test]
    #[ignore = "compile-only signature guard; execution requires a real ONNX decoder model"]
    #[cfg(feature = "ort-runtime")]
    fn decoder_run_method_accepts_encoder_hidden_states() {
        fn assert_signature(
            engine: &mut OrtEngine,
            names: &OrtDecoderIoNames,
            inputs: &OrtGenerationTensorInputs,
            encoder_output: &OrtFloatTensorOutput,
        ) {
            let result: Result<OrtFloatTensorOutput, OrtEngineError> =
                engine.run_decoder(names, inputs, encoder_output);
            let _ = result;
        }

        let _signature: fn(
            &mut OrtEngine,
            &OrtDecoderIoNames,
            &OrtGenerationTensorInputs,
            &OrtFloatTensorOutput,
        ) = assert_signature;
    }

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
    #[cfg(feature = "ort-runtime")]
    fn runtime_dylib_path_rejects_missing_env_value() {
        let error = resolve_ort_dylib_path_from_env_value(None);

        assert!(matches!(error, Err(OrtEngineError::MissingOrtDylibPath)));
    }

    #[test]
    #[cfg(feature = "ort-runtime")]
    fn runtime_dylib_path_rejects_relative_env_value() {
        let error = resolve_ort_dylib_path_from_env_value(Some("libonnxruntime.dylib".into()));

        assert!(matches!(
            error,
            Err(OrtEngineError::InvalidOrtDylibPath { ref reason, .. })
                if reason == "path must be absolute"
        ));
    }

    #[test]
    #[cfg(feature = "ort-runtime")]
    fn runtime_dylib_path_rejects_missing_file() {
        let missing_path = PathBuf::from("/tmp/localmt-missing-libonnxruntime.dylib");

        let error = resolve_ort_dylib_path_from_env_value(Some(missing_path.into_os_string()));

        assert!(matches!(
            error,
            Err(OrtEngineError::InvalidOrtDylibPath { ref reason, .. })
                if reason == "path does not point to a file"
        ));
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
    fn generator_plan_parses_runtime_config() -> Result<(), Box<dyn std::error::Error>> {
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
                (
                    "config.json",
                    "config",
                    VALID_ORT_IO_CONFIG_SHA256,
                    VALID_ORT_IO_CONFIG,
                ),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let runtime_config = plan.parse_runtime_config()?;

        assert_eq!(
            runtime_config.generation_config().max_new_tokens().value(),
            32
        );
        assert_eq!(
            runtime_config.ort_io_config().decoder().logits(),
            "decoder_logits"
        );
        Ok(())
    }

    #[test]
    fn generator_plan_requires_generation_config_for_runtime_config()
    -> Result<(), Box<dyn std::error::Error>> {
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
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let runtime_config = plan.parse_runtime_config();

        assert!(matches!(
            runtime_config,
            Err(OrtEngineError::MissingGenerationConfig)
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_requires_ort_io_config_for_runtime_config()
    -> Result<(), Box<dyn std::error::Error>> {
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

        let runtime_config = plan.parse_runtime_config();

        assert!(matches!(
            runtime_config,
            Err(OrtEngineError::MissingOrtIoConfig)
        ));
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

    fn decoder_logits_output() -> OrtFloatTensorOutput {
        OrtFloatTensorOutput {
            shape: vec![1, 2, 3],
            values: vec![0.1, 0.2, 0.3, 4.0, 2.0, 1.0],
        }
    }

    fn generation_state(max_new_tokens: usize) -> OrtGenerationState {
        OrtGenerationState {
            decoder_input_ids: vec![11],
            generated_token_ids: Vec::new(),
            max_new_tokens,
            eos_token_id: 1,
            finished: false,
        }
    }

    fn raw_generation_inputs() -> OrtGenerationInputs {
        raw_generation_inputs_with_limit(3)
    }

    fn raw_generation_inputs_with_limit(max_new_tokens: usize) -> OrtGenerationInputs {
        OrtGenerationInputs {
            encoder_input_ids: vec![7, 8],
            encoder_attention_mask: vec![1, 1],
            decoder_input_ids: vec![11],
            max_new_tokens,
            eos_token_id: 1,
        }
    }

    fn decoder_logits(values: [f32; 3]) -> OrtFloatTensorOutput {
        OrtFloatTensorOutput {
            shape: vec![1, 1, 3],
            values: values.to_vec(),
        }
    }
}
