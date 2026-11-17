use std::{
    env, fs,
    path::{Path, PathBuf},
};

use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_address::Address;
use solana_keypair::{Keypair, Signer};

use crate::{AxisTestError, Result};

pub const AXIS_CORE_PROGRAM_ARTIFACT_ENV: &str = "AXIS_CORE_PROGRAM_ARTIFACT";
pub const DEFAULT_AXIS_CORE_PROGRAM_ARTIFACT: &str = "target/deploy/axis_core.so";
pub const DEFAULT_PAYER_LAMPORTS: u64 = 10_000_000_000;

const KEYPAIR_DOMAIN: &[u8] = b"axis-core-test-utils:keypair:v1:";

#[derive(Debug)]
pub struct FundedTestSigner {
    pub label: String,
    pub keypair: Keypair,
    pub lamports: u64,
}

impl FundedTestSigner {
    pub fn pubkey(&self) -> Address {
        self.keypair.pubkey()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramArtifactLoad {
    pub program_id: Address,
    pub artifact_path: PathBuf,
}

pub fn fresh_vm() -> LiteSVM {
    LiteSVM::new()
}

pub fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("axis-core-test-utils must live under <workspace>/crates")
        .to_path_buf()
}

pub fn deterministic_keypair(label: impl AsRef<[u8]>) -> Keypair {
    let mut hasher = Sha256::new();
    hasher.update(KEYPAIR_DOMAIN);
    hasher.update(label.as_ref());

    let digest = hasher.finalize();
    let mut seed = [0_u8; Keypair::SECRET_KEY_LENGTH];
    seed.copy_from_slice(&digest);

    Keypair::new_from_array(seed)
}

pub fn axis_core_program_id() -> Address {
    deterministic_keypair(b"axis-core-program-id").pubkey()
}

pub fn provision_deterministic_payer(vm: &mut LiteSVM) -> Result<FundedTestSigner> {
    provision_funded_signer(vm, "payer", DEFAULT_PAYER_LAMPORTS)
}

pub fn provision_funded_signer(
    vm: &mut LiteSVM,
    label: impl AsRef<[u8]>,
    lamports: u64,
) -> Result<FundedTestSigner> {
    let label = label.as_ref();
    let keypair = deterministic_keypair(label);
    let address = keypair.pubkey();

    vm.airdrop(&address, lamports)
        .map_err(|err| AxisTestError::FundingFailed {
            address: address.clone(),
            details: format!("{err:?}"),
        })?;

    Ok(FundedTestSigner {
        label: String::from_utf8_lossy(label).into_owned(),
        keypair,
        lamports,
    })
}

/// `AXIS_CORE_PROGRAM_ARTIFACT` overrides the default workspace artifact path.
pub fn axis_core_program_artifact_path() -> PathBuf {
    match env::var_os(AXIS_CORE_PROGRAM_ARTIFACT_ENV) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => workspace_root().join(DEFAULT_AXIS_CORE_PROGRAM_ARTIFACT),
    }
}

pub fn load_axis_core_program(vm: &mut LiteSVM) -> Result<ProgramArtifactLoad> {
    let artifact_path = axis_core_program_artifact_path();
    load_axis_core_program_from_path(vm, axis_core_program_id(), artifact_path)
}

pub fn load_axis_core_program_from_path(
    vm: &mut LiteSVM,
    program_id: Address,
    artifact_path: impl AsRef<Path>,
) -> Result<ProgramArtifactLoad> {
    let artifact_path = artifact_path.as_ref();
    ensure_program_artifact(artifact_path)?;

    vm.add_program_from_file(program_id.clone(), artifact_path)
        .map_err(|err| {
            invalid_artifact(artifact_path, format!("LiteSVM failed to load it: {err}"))
        })?;

    Ok(ProgramArtifactLoad {
        program_id,
        artifact_path: artifact_path.to_path_buf(),
    })
}

fn ensure_program_artifact(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => missing_artifact(path),
        _ => invalid_artifact(path, format!("failed to inspect metadata: {err}")),
    })?;

    if !metadata.is_file() {
        return Err(invalid_artifact(
            path,
            "path exists but is not a regular file".to_string(),
        ));
    }

    if metadata.len() == 0 {
        return Err(invalid_artifact(path, "artifact file is empty".to_string()));
    }

    if path.extension().and_then(|extension| extension.to_str()) != Some("so") {
        return Err(invalid_artifact(
            path,
            "expected a .so Solana SBF shared object".to_string(),
        ));
    }

    Ok(())
}

fn missing_artifact(path: &Path) -> AxisTestError {
    AxisTestError::MissingProgramArtifact {
        path: path.to_path_buf(),
        diagnostic: format!(
            "Axis Core LiteSVM program artifact not found at {}. {}",
            path.display(),
            artifact_blocker_note()
        ),
    }
}

fn invalid_artifact(path: &Path, reason: String) -> AxisTestError {
    AxisTestError::InvalidProgramArtifact {
        path: path.to_path_buf(),
        diagnostic: format!(
            "Axis Core LiteSVM program artifact at {} is invalid: {reason}. {}",
            path.display(),
            artifact_blocker_note()
        ),
    }
}

fn artifact_blocker_note() -> &'static str {
    "Build it with scripts/build-sbf.sh, which pins platform-tools v1.57 and works around the cargo-build-sbf toolchain-link bug. Set AXIS_CORE_PROGRAM_ARTIFACT to override the path. Do not treat the program as loaded unless a LiteSVM-loadable .so exists."
}
