//! End-to-end proof that the SBF artifact loads and executes in LiteSVM.
//!
//! These tests only mean anything when `target/deploy/axis_core.so` exists, so
//! they fail loudly rather than skipping if it does not: a silently skipped
//! integration test is how a broken toolchain stays invisible.

use axis_core::constants::{MARKET_SEED, PROTOCOL_CONFIG_SEED};
use axis_core::state::{DTFMarket, MarketStatus, ProtocolConfig, TokenProgramKind};
use axis_core_test_utils::{fresh_vm, load_axis_core_program, provision_deterministic_payer};
use litesvm::LiteSVM;
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_message::Message;
use solana_transaction::Transaction;

fn to_pubkey(a: &Address) -> solana_pubkey::Pubkey {
    solana_pubkey::Pubkey::from(a.to_bytes())
}

/// An account owned by Axis Core, allocated and rent-funded but never written.
/// This is exactly the state a System Program `create_account` CPI leaves
/// behind, which the handlers are written to expect.
fn blank_program_account(vm: &mut LiteSVM, address: &Address, len: usize, owner: &Address) {
    vm.set_account(
        address.clone(),
        Account {
            lamports: 10_000_000,
            data: vec![0u8; len],
            owner: owner.clone(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("account injection");
}

fn setup() -> (LiteSVM, Keypair, Address) {
    let mut vm = fresh_vm();
    let payer = provision_deterministic_payer(&mut vm).expect("payer");
    let load = load_axis_core_program(&mut vm)
        .expect("target/deploy/axis_core.so must exist; build it with scripts/build-sbf.sh");
    (vm, payer.keypair, load.program_id)
}

fn send(
    vm: &mut LiteSVM,
    payer: &Keypair,
    ix: Instruction,
) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], message, vm.latest_blockhash());
    vm.send_transaction(tx).map(|_| ())
}

fn init_config_ix(
    program_id: &Address,
    authority: &Address,
    config: &Address,
    usdc_mint: &Address,
    treasury: &Address,
    mint_fee_bps: u16,
    creator_share_bps: u16,
    max_mint_fee_bps: u16,
    bump: u8,
) -> Instruction {
    let mut data = vec![0u8]; // InitializeProtocolConfig
    data.extend_from_slice(usdc_mint.as_ref());
    data.extend_from_slice(treasury.as_ref());
    data.extend_from_slice(&mint_fee_bps.to_le_bytes());
    data.extend_from_slice(&creator_share_bps.to_le_bytes());
    data.extend_from_slice(&max_mint_fee_bps.to_le_bytes());
    data.push(bump);
    Instruction {
        program_id: to_pubkey(program_id),
        accounts: vec![
            AccountMeta::new(to_pubkey(authority), true),
            AccountMeta::new(to_pubkey(config), false),
        ],
        data,
    }
}

#[test]
fn the_program_loads_and_initializes_protocol_config_on_chain() {
    let (mut vm, payer, program_id) = setup();
    let (config, bump) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    blank_program_account(&mut vm, &config, ProtocolConfig::LEN, &program_id);

    let authority = Address::try_from(payer.pubkey().as_ref()).unwrap();
    let usdc = Address::new_from_array([9u8; 32]);
    let treasury = Address::new_from_array([8u8; 32]);

    send(
        &mut vm,
        &payer,
        init_config_ix(
            &program_id,
            &authority,
            &config,
            &usdc,
            &treasury,
            100,
            4_000,
            300,
            bump,
        ),
    )
    .expect("initialize protocol config");

    let raw = vm.get_account(&config).expect("config account").data;
    let state = ProtocolConfig::unpack(&raw).expect("decode");
    assert_eq!(state.protocol_authority, authority);
    assert_eq!(state.usdc_mint, usdc);
    assert_eq!(state.mint_fee_bps, 100);
    assert_eq!(state.bump, bump);
}

/// The singleton is enforced by the fixed PDA address plus the
/// already-initialized check, not by an authority list.
#[test]
fn protocol_config_cannot_be_initialized_twice() {
    let (mut vm, payer, program_id) = setup();
    let (config, bump) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    blank_program_account(&mut vm, &config, ProtocolConfig::LEN, &program_id);

    let authority = Address::try_from(payer.pubkey().as_ref()).unwrap();
    let usdc = Address::new_from_array([9u8; 32]);
    let treasury = Address::new_from_array([8u8; 32]);
    let ix = init_config_ix(
        &program_id,
        &authority,
        &config,
        &usdc,
        &treasury,
        100,
        4_000,
        300,
        bump,
    );

    send(&mut vm, &payer, ix.clone()).expect("first init");
    vm.expire_blockhash();
    let err = send(&mut vm, &payer, ix).expect_err("second init must fail");
    assert!(
        format!("{err:?}").contains("Custom(8)"),
        "expected AccountAlreadyInitialized, got {err:?}"
    );
}

/// A look-alike config account the caller owns must not be accepted, or the
/// caller sets their own fee terms.
#[test]
fn a_config_account_owned_by_someone_else_is_rejected() {
    let (mut vm, payer, program_id) = setup();
    let (config, bump) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    let impostor = Address::new_from_array([7u8; 32]);
    blank_program_account(&mut vm, &config, ProtocolConfig::LEN, &impostor);

    let authority = Address::try_from(payer.pubkey().as_ref()).unwrap();
    let err = send(
        &mut vm,
        &payer,
        init_config_ix(
            &program_id,
            &authority,
            &config,
            &Address::new_from_array([9u8; 32]),
            &Address::new_from_array([8u8; 32]),
            100,
            4_000,
            300,
            bump,
        ),
    )
    .expect_err("foreign owner must fail");
    assert!(
        format!("{err:?}").contains("Custom(5)"),
        "expected InvalidAccountOwner, got {err:?}"
    );
}

#[test]
fn create_market_writes_the_inline_asset_table() {
    let (mut vm, payer, program_id) = setup();
    let authority = Address::try_from(payer.pubkey().as_ref()).unwrap();
    let usdc = Address::new_from_array([9u8; 32]);
    let treasury = Address::new_from_array([8u8; 32]);

    let (config, cfg_bump) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    blank_program_account(&mut vm, &config, ProtocolConfig::LEN, &program_id);
    send(
        &mut vm,
        &payer,
        init_config_ix(
            &program_id,
            &authority,
            &config,
            &usdc,
            &treasury,
            100,
            4_000,
            300,
            cfg_bump,
        ),
    )
    .expect("config");
    vm.expire_blockhash();

    let dtf_mint = Address::new_from_array([5u8; 32]);
    let (market, bump) =
        Address::find_program_address(&[MARKET_SEED, dtf_mint.as_ref()], &program_id);
    blank_program_account(&mut vm, &market, DTFMarket::LEN, &program_id);

    let mints = [[11u8; 32], [12u8; 32], [13u8; 32]];
    let vaults = [[21u8; 32], [22u8; 32], [23u8; 32]];
    let weights = [5_000u16, 3_000, 2_000];

    let mut data = vec![1u8]; // CreateMarket
    data.extend_from_slice(&[3u8; 32]); // creator_fee_destination
    data.push(3); // asset_count
    data.push(bump);
    for i in 0..3 {
        data.extend_from_slice(&mints[i]);
        data.extend_from_slice(&vaults[i]);
        data.extend_from_slice(&weights[i].to_le_bytes());
        data.push(TokenProgramKind::LegacySplToken as u8);
    }

    send(
        &mut vm,
        &payer,
        Instruction {
            program_id: to_pubkey(&program_id),
            accounts: vec![
                AccountMeta::new(to_pubkey(&authority), true),
                AccountMeta::new_readonly(to_pubkey(&config), false),
                AccountMeta::new(to_pubkey(&market), false),
                AccountMeta::new_readonly(to_pubkey(&dtf_mint), false),
            ],
            data,
        },
    )
    .expect("create market");

    let raw = vm.get_account(&market).expect("market account").data;
    let state = DTFMarket::unpack(&raw).expect("decode");

    assert_eq!(state.status, MarketStatus::Created);
    assert_eq!(state.asset_count, 3);
    // Fee terms are snapshotted, so a later protocol change cannot reprice
    // this market.
    assert_eq!(state.mint_fee_bps, 100);
    assert_eq!(state.creator_share_bps, 4_000);

    let live: Vec<_> = state.assets().collect();
    assert_eq!(live.len(), 3);
    for (i, a) in live.iter().enumerate() {
        assert_eq!(a.asset_mint.to_bytes(), mints[i]);
        assert_eq!(a.reserve_vault.to_bytes(), vaults[i]);
        assert_eq!(a.weight_bps, weights[i]);
    }
    state.validate_composition().expect("composition");
}

#[test]
fn create_market_rejects_a_four_asset_composition() {
    let (mut vm, payer, program_id) = setup();
    let authority = Address::try_from(payer.pubkey().as_ref()).unwrap();
    let (config, cfg_bump) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    blank_program_account(&mut vm, &config, ProtocolConfig::LEN, &program_id);
    send(
        &mut vm,
        &payer,
        init_config_ix(
            &program_id,
            &authority,
            &config,
            &Address::new_from_array([9u8; 32]),
            &Address::new_from_array([8u8; 32]),
            100,
            4_000,
            300,
            cfg_bump,
        ),
    )
    .expect("config");
    vm.expire_blockhash();

    let dtf_mint = Address::new_from_array([5u8; 32]);
    let (market, bump) =
        Address::find_program_address(&[MARKET_SEED, dtf_mint.as_ref()], &program_id);
    blank_program_account(&mut vm, &market, DTFMarket::LEN, &program_id);

    let mut data = vec![1u8];
    data.extend_from_slice(&[3u8; 32]);
    data.push(4); // above the measured 3-asset cap
    data.push(bump);
    for i in 0..4u8 {
        data.extend_from_slice(&[11 + i; 32]);
        data.extend_from_slice(&[21 + i; 32]);
        data.extend_from_slice(&2_500u16.to_le_bytes());
        data.push(0);
    }

    let err = send(
        &mut vm,
        &payer,
        Instruction {
            program_id: to_pubkey(&program_id),
            accounts: vec![
                AccountMeta::new(to_pubkey(&authority), true),
                AccountMeta::new_readonly(to_pubkey(&config), false),
                AccountMeta::new(to_pubkey(&market), false),
                AccountMeta::new_readonly(to_pubkey(&dtf_mint), false),
            ],
            data,
        },
    )
    .expect_err("four assets must fail");
    assert!(
        format!("{err:?}").contains("Custom(21)"),
        "expected TooManyAssets, got {err:?}"
    );
}
