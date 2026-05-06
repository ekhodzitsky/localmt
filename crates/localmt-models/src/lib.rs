//! Model-pack manifest and verification layer.

use core::fmt;
use core::marker::PhantomData;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use localmt_core::Language;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST_FILE_NAME: &str = "manifest.json";
const SCHEMA_VERSION: u16 = 0;
const REQUIRED_LANGUAGES: [Language; 5] = [
    Language::English,
    Language::Russian,
    Language::Thai,
    Language::Vietnamese,
    Language::Japanese,
];

/// Model pack state before file checksums have been verified.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Discovered;

/// Model pack state after manifest, files, and checksums have been verified.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Verified;

/// Model pack with a typed lifecycle state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPack<State> {
    root: PathBuf,
    manifest: ModelManifest,
    state: PhantomData<State>,
}

impl ModelPack<Discovered> {
    /// { root points to a model-pack directory candidate }
    /// fn discover(root: impl `AsRef<Path>`) -> Result<Self, ModelPackError>
    /// { ret is Ok only when manifest.json exists, parses, and has safe relative paths }
    pub fn discover(root: impl AsRef<Path>) -> Result<Self, ModelPackError> {
        let root = root.as_ref().to_path_buf();
        let manifest_path = root.join(MANIFEST_FILE_NAME);
        let manifest_json =
            fs::read_to_string(&manifest_path).map_err(|source| ModelPackError::ReadManifest {
                path: manifest_path,
                source,
            })?;
        let raw: RawManifest =
            serde_json::from_str(&manifest_json).map_err(ModelPackError::ParseManifest)?;
        let manifest = ModelManifest::from_raw(raw)?;

        Ok(Self {
            root,
            manifest,
            state: PhantomData,
        })
    }

    /// { self was discovered successfully }
    /// fn verify(self) -> Result<`ModelPack<Verified>`, ModelPackError>
    /// { ret is Ok only when required languages, files, and checksums are valid }
    pub fn verify(self) -> Result<ModelPack<Verified>, ModelPackError> {
        self.verify_required_languages()?;
        self.verify_files()?;

        Ok(ModelPack {
            root: self.root,
            manifest: self.manifest,
            state: PhantomData,
        })
    }

    fn verify_required_languages(&self) -> Result<(), ModelPackError> {
        for language in REQUIRED_LANGUAGES {
            if !self.manifest.supports(language) {
                return Err(ModelPackError::MissingRequiredLanguage(language));
            }
        }

        Ok(())
    }

    fn verify_files(&self) -> Result<(), ModelPackError> {
        for file in self.manifest.files() {
            let path = self.root.join(file.path().as_path());
            let actual = sha256_file(&path)?;
            if actual != *file.sha256() {
                return Err(ModelPackError::ChecksumMismatch {
                    path,
                    expected: file.sha256().clone(),
                    actual,
                });
            }
        }

        Ok(())
    }
}

impl<State> ModelPack<State> {
    /// { true }
    /// fn root(&self) -> &Path
    /// { ret is the model-pack root directory }
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// { true }
    /// fn manifest(&self) -> &ModelManifest
    /// { ret is the parsed model-pack manifest }
    pub const fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }
}

/// Validated model-pack manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelManifest {
    schema_version: u16,
    model_id: ModelId,
    version: ModelPackVersion,
    architecture: ModelArchitecture,
    runtime: ModelRuntime,
    license: ModelLicense,
    languages: Vec<Language>,
    files: Vec<ModelFile>,
}

impl ModelManifest {
    fn from_raw(raw: RawManifest) -> Result<Self, ModelPackError> {
        if raw.schema_version != SCHEMA_VERSION {
            return Err(ModelPackError::UnsupportedSchema(raw.schema_version));
        }

        let languages = parse_languages(raw.languages)?;
        let files = parse_files(raw.files)?;

        Ok(Self {
            schema_version: raw.schema_version,
            model_id: ModelId::new(raw.model_id)?,
            version: ModelPackVersion::new(raw.version)?,
            architecture: ModelArchitecture::new(raw.architecture)?,
            runtime: ModelRuntime::new(raw.runtime)?,
            license: ModelLicense::new(raw.license)?,
            languages,
            files,
        })
    }

    /// { true }
    /// fn schema_version(&self) -> u16
    /// { ret is the manifest schema version }
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// { true }
    /// fn model_id(&self) -> &ModelId
    /// { ret is the model-pack id }
    pub const fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    /// { true }
    /// fn version(&self) -> &ModelPackVersion
    /// { ret is the model-pack version string }
    pub const fn version(&self) -> &ModelPackVersion {
        &self.version
    }

    /// { true }
    /// fn architecture(&self) -> &ModelArchitecture
    /// { ret is the declared model architecture }
    pub const fn architecture(&self) -> &ModelArchitecture {
        &self.architecture
    }

    /// { true }
    /// fn runtime(&self) -> &ModelRuntime
    /// { ret is the declared inference runtime }
    pub const fn runtime(&self) -> &ModelRuntime {
        &self.runtime
    }

    /// { true }
    /// fn license(&self) -> &ModelLicense
    /// { ret is the declared model license }
    pub const fn license(&self) -> &ModelLicense {
        &self.license
    }

    /// { true }
    /// fn languages(&self) -> &[Language]
    /// { ret contains every declared supported language }
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }

    /// { true }
    /// fn files(&self) -> &[ModelFile]
    /// { ret contains every declared model-pack file }
    pub fn files(&self) -> &[ModelFile] {
        &self.files
    }

    /// { true }
    /// fn supports(&self, language: Language) -> bool
    /// { ret is true only when language is in self.languages() }
    pub fn supports(&self, language: Language) -> bool {
        self.languages.contains(&language)
    }
}

macro_rules! string_newtype {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// { value may be any string }
            /// fn new(value: String) -> Result<Self, ModelPackError>
            /// { ret is Ok only when value is non-empty after trimming }
            pub fn new(value: String) -> Result<Self, ModelPackError> {
                if value.trim().is_empty() {
                    Err(ModelPackError::EmptyField($label))
                } else {
                    Ok(Self(value))
                }
            }

            /// { true }
            /// fn as_str(&self) -> &str
            /// { ret is the validated field value }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

string_newtype!(ModelId, "model_id");
string_newtype!(ModelPackVersion, "version");
string_newtype!(ModelArchitecture, "architecture");
string_newtype!(ModelRuntime, "runtime");
string_newtype!(ModelLicense, "license");
string_newtype!(ModelFileKind, "files.kind");

/// Safe relative model-pack path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ModelRelativePath(PathBuf);

impl ModelRelativePath {
    /// { value may be any manifest path string }
    /// fn new(value: String) -> Result<Self, ModelPackError>
    /// { ret is Ok only when value is relative and cannot escape the pack root }
    pub fn new(value: String) -> Result<Self, ModelPackError> {
        let path = PathBuf::from(&value);
        if path.components().next().is_none() || path.is_absolute() {
            return Err(ModelPackError::InvalidRelativePath(value));
        }

        for component in path.components() {
            if !matches!(component, Component::Normal(_)) {
                return Err(ModelPackError::InvalidRelativePath(value));
            }
        }

        Ok(Self(path))
    }

    /// { true }
    /// fn as_path(&self) -> &Path
    /// { ret is the safe relative path }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for ModelRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(formatter)
    }
}

/// Lowercase SHA-256 digest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// { value may be any string }
    /// fn new(value: String) -> Result<Self, ModelPackError>
    /// { ret is Ok only when value is a 64-character lowercase hex digest }
    pub fn new(value: String) -> Result<Self, ModelPackError> {
        if value.len() == 64 && value.bytes().all(is_lower_hex) {
            Ok(Self(value))
        } else {
            Err(ModelPackError::InvalidSha256(value))
        }
    }

    /// { true }
    /// fn as_str(&self) -> &str
    /// { ret is the lowercase hexadecimal digest }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Manifest-declared file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFile {
    path: ModelRelativePath,
    kind: ModelFileKind,
    sha256: Sha256Digest,
}

impl ModelFile {
    /// { all fields are validated }
    /// fn new(path: ModelRelativePath, kind: ModelFileKind, sha256: Sha256Digest) -> Self
    /// { ret contains exactly those fields }
    pub const fn new(path: ModelRelativePath, kind: ModelFileKind, sha256: Sha256Digest) -> Self {
        Self { path, kind, sha256 }
    }

    /// { true }
    /// fn path(&self) -> &ModelRelativePath
    /// { ret is the file path relative to pack root }
    pub const fn path(&self) -> &ModelRelativePath {
        &self.path
    }

    /// { true }
    /// fn kind(&self) -> &ModelFileKind
    /// { ret is the manifest-declared file kind }
    pub const fn kind(&self) -> &ModelFileKind {
        &self.kind
    }

    /// { true }
    /// fn sha256(&self) -> &Sha256Digest
    /// { ret is the manifest-declared SHA-256 digest }
    pub const fn sha256(&self) -> &Sha256Digest {
        &self.sha256
    }
}

/// Model-pack manifest and verification error.
#[derive(Debug)]
pub enum ModelPackError {
    /// `manifest.json` could not be read.
    ReadManifest {
        /// Manifest path.
        path: PathBuf,
        /// I/O source error.
        source: std::io::Error,
    },
    /// Manifest JSON could not be parsed.
    ParseManifest(serde_json::Error),
    /// Manifest schema is not supported.
    UnsupportedSchema(u16),
    /// Required string field is empty.
    EmptyField(&'static str),
    /// Language code is not supported by localmt.
    InvalidLanguage(localmt_core::LanguageCodeError),
    /// A language is declared more than once.
    DuplicateLanguage(Language),
    /// File list is empty.
    EmptyFileList,
    /// A manifest file path is not a safe relative path.
    InvalidRelativePath(String),
    /// SHA-256 value is not a lowercase 64-character hex digest.
    InvalidSha256(String),
    /// Required MVP language is not declared.
    MissingRequiredLanguage(Language),
    /// Manifest-declared file could not be read.
    ReadModelFile {
        /// Model-pack file path.
        path: PathBuf,
        /// I/O source error.
        source: std::io::Error,
    },
    /// File digest does not match manifest.
    ChecksumMismatch {
        /// Model-pack file path.
        path: PathBuf,
        /// Manifest digest.
        expected: Sha256Digest,
        /// Actual digest.
        actual: Sha256Digest,
    },
}

impl fmt::Display for ModelPackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadManifest { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::ParseManifest(source) => write!(formatter, "failed to parse manifest: {source}"),
            Self::UnsupportedSchema(version) => write!(formatter, "unsupported schema: {version}"),
            Self::EmptyField(field) => write!(formatter, "manifest field is empty: {field}"),
            Self::InvalidLanguage(error) => write!(formatter, "{error}"),
            Self::DuplicateLanguage(language) => {
                write!(formatter, "duplicate language: {language}")
            }
            Self::EmptyFileList => formatter.write_str("manifest files list must not be empty"),
            Self::InvalidRelativePath(path) => write!(formatter, "invalid relative path: {path}"),
            Self::InvalidSha256(value) => write!(formatter, "invalid sha256 digest: {value}"),
            Self::MissingRequiredLanguage(language) => {
                write!(formatter, "missing required language: {language}")
            }
            Self::ReadModelFile { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::ChecksumMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "checksum mismatch for {}: expected {expected}, got {actual}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ModelPackError {}

#[derive(Deserialize)]
struct RawManifest {
    schema_version: u16,
    model_id: String,
    version: String,
    architecture: String,
    runtime: String,
    license: String,
    languages: Vec<String>,
    files: Vec<RawModelFile>,
}

#[derive(Deserialize)]
struct RawModelFile {
    path: String,
    kind: String,
    sha256: String,
}

fn parse_languages(values: Vec<String>) -> Result<Vec<Language>, ModelPackError> {
    let mut seen = BTreeSet::new();
    let mut languages = Vec::with_capacity(values.len());
    for value in values {
        let language = Language::from_iso_639_1(&value).map_err(ModelPackError::InvalidLanguage)?;
        if !seen.insert(language) {
            return Err(ModelPackError::DuplicateLanguage(language));
        }
        languages.push(language);
    }

    Ok(languages)
}

fn parse_files(values: Vec<RawModelFile>) -> Result<Vec<ModelFile>, ModelPackError> {
    if values.is_empty() {
        return Err(ModelPackError::EmptyFileList);
    }

    values.into_iter().map(parse_file).collect()
}

fn parse_file(value: RawModelFile) -> Result<ModelFile, ModelPackError> {
    Ok(ModelFile::new(
        ModelRelativePath::new(value.path)?,
        ModelFileKind::new(value.kind)?,
        Sha256Digest::new(value.sha256)?,
    ))
}

fn sha256_file(path: &Path) -> Result<Sha256Digest, ModelPackError> {
    let mut file = fs::File::open(path).map_err(|source| ModelPackError::ReadModelFile {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let bytes_read =
            file.read(&mut buffer)
                .map_err(|source| ModelPackError::ReadModelFile {
                    path: path.to_path_buf(),
                    source,
                })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let digest = hasher.finalize();
    Ok(sha256_bytes_to_hex(&digest))
}

const fn is_lower_hex(byte: u8) -> bool {
    matches!(byte, b'0'..=b'9' | b'a'..=b'f')
}

fn sha256_bytes_to_hex(bytes: &[u8]) -> Sha256Digest {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[(byte >> 4) as usize]));
        output.push(char::from(HEX[(byte & 0x0f) as usize]));
    }

    Sha256Digest(output)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_core::Language;

    use crate::{Discovered, ModelPack, ModelPackError};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn complete_pack_verifies_and_exposes_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&["en", "ru", "th", "vi", "ja"], ENCODER_SHA256)?;

        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        assert_eq!(pack.manifest().model_id().as_str(), "m2m100-418m-int8");
        assert_eq!(pack.manifest().languages().len(), 5);
        assert!(pack.manifest().supports(Language::Japanese));
        Ok(())
    }

    #[test]
    fn verification_rejects_missing_required_language() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&["en", "ru", "th", "vi"], ENCODER_SHA256)?;

        let error = ModelPack::<Discovered>::discover(&root)?.verify();

        assert!(matches!(
            error,
            Err(ModelPackError::MissingRequiredLanguage(Language::Japanese))
        ));
        Ok(())
    }

    #[test]
    fn verification_rejects_checksum_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(
            &["en", "ru", "th", "vi", "ja"],
            "0000000000000000000000000000000000000000000000000000000000000000",
        )?;

        let error = ModelPack::<Discovered>::discover(&root)?.verify();

        assert!(matches!(
            error,
            Err(ModelPackError::ChecksumMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn discovery_rejects_relative_path_escape() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(
            root.join("manifest.json"),
            manifest_json(
                &["en", "ru", "th", "vi", "ja"],
                "../encoder.onnx",
                ENCODER_SHA256,
            ),
        )?;

        let error = ModelPack::<Discovered>::discover(&root);

        assert!(matches!(error, Err(ModelPackError::InvalidRelativePath(_))));
        Ok(())
    }

    fn create_pack(
        languages: &[&str],
        encoder_sha256: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(
            root.join("manifest.json"),
            manifest_json(languages, "encoder.onnx", encoder_sha256),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-models-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn manifest_json(languages: &[&str], encoder_path: &str, encoder_sha256: &str) -> String {
        let language_json = languages
            .iter()
            .map(|language| format!("\"{language}\""))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"{{
  "schema_version": 0,
  "model_id": "m2m100-418m-int8",
  "version": "0.1.0",
  "architecture": "m2m100",
  "runtime": "onnx-runtime",
  "license": "MIT",
  "languages": [{language_json}],
  "files": [
    {{ "path": "{encoder_path}", "kind": "encoder", "sha256": "{encoder_sha256}" }},
    {{ "path": "tokenizer.json", "kind": "tokenizer", "sha256": "{TOKENIZER_SHA256}" }}
  ]
}}"#
        )
    }
}
