use std::fs;

use axis_core_test_utils::{
    account_data, account_lamports, axis_core_program_artifact_path, axis_core_program_id,
    fresh_vm, load_axis_core_program, load_axis_core_program_from_path,
    provision_deterministic_payer, workspace_root, AxisTestError, DEFAULT_PAYER_LAMPORTS,
};

#[test]
fn fresh_vm_and_payer_setup_are_deterministic() {
    let mut first_vm = fresh_vm();
    let first_payer = provision_deterministic_payer(&mut first_vm).unwrap();

    let mut second_vm = fresh_vm();
    let second_payer = provision_deterministic_payer(&mut second_vm).unwrap();

    assert_eq!(first_vm.latest_blockhash(), second_vm.latest_blockhash());
    assert_eq!(first_payer.pubkey(), second_payer.pubkey());
    assert_eq!(
        account_lamports(&first_vm, &first_payer.pubkey()).unwrap(),
        DEFAULT_PAYER_LAMPORTS
    );
    assert_eq!(
        account_lamports(&second_vm, &second_payer.pubkey()).unwrap(),
        DEFAULT_PAYER_LAMPORTS
    );
    assert!(account_data(&first_vm, &first_payer.pubkey())
        .unwrap()
        .is_empty());
}

#[test]
fn axis_core_artifact_path_and_load_diagnostics_are_exercised() {
    let mut vm = fresh_vm();
    let expected_path = axis_core_program_artifact_path();

    match load_axis_core_program(&mut vm) {
        Ok(load) => {
            assert_eq!(load.artifact_path, expected_path);
            assert_eq!(load.program_id, axis_core_program_id());
            assert!(vm.get_account(&load.program_id).is_some());
        }
        Err(AxisTestError::MissingProgramArtifact { path, diagnostic }) => {
            assert_eq!(path, expected_path);
            assert!(diagnostic.contains("not found"));
            assert!(diagnostic.contains("scripts/build-sbf.sh"));
            eprintln!("{diagnostic}");
        }
        Err(err) => panic!("unexpected Axis Core artifact load failure: {err}"),
    }
}

#[test]
fn missing_program_artifact_has_clear_diagnostic() {
    let mut vm = fresh_vm();
    let missing_path = workspace_root()
        .join("target")
        .join("axis-core-test-artifacts")
        .join("missing-axis-core.so");
    let _ = fs::remove_file(&missing_path);

    let err = load_axis_core_program_from_path(&mut vm, axis_core_program_id(), &missing_path)
        .unwrap_err();

    match err {
        AxisTestError::MissingProgramArtifact { path, diagnostic } => {
            assert_eq!(path, missing_path);
            assert!(diagnostic.contains("not found"));
            assert!(diagnostic.contains("AXIS_CORE_PROGRAM_ARTIFACT"));
            assert!(diagnostic.contains("Do not treat the program as loaded"));
        }
        err => panic!("expected missing artifact diagnostic, got {err}"),
    }
}

#[test]
fn invalid_program_artifact_has_clear_diagnostic() {
    let mut vm = fresh_vm();
    let artifact_dir = workspace_root()
        .join("target")
        .join("axis-core-test-artifacts");
    fs::create_dir_all(&artifact_dir).unwrap();

    let invalid_path = artifact_dir.join("invalid-axis-core.so");
    fs::write(&invalid_path, b"not a solana sbf shared object").unwrap();

    let err = load_axis_core_program_from_path(&mut vm, axis_core_program_id(), &invalid_path)
        .unwrap_err();

    match err {
        AxisTestError::InvalidProgramArtifact { path, diagnostic } => {
            assert_eq!(path, invalid_path);
            assert!(diagnostic.contains("invalid"));
            assert!(diagnostic.contains("LiteSVM failed to load it"));
            assert!(diagnostic.contains("Do not treat the program as loaded"));
        }
        err => panic!("expected invalid artifact diagnostic, got {err}"),
    }
}
