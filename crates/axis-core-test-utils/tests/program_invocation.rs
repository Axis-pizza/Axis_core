//! End-to-end proof that the SBF artifact loads and executes in LiteSVM.
//!
//! These tests only mean anything when `target/deploy/axis_core.so` exists, so
//! they fail loudly rather than skipping if it does not: a silently skipped
//! integration test is how a broken toolchain stays invisible.
//!
//! No test injects an Axis-owned state account before the instruction that
//! creates it. The program allocates its own PDAs through the System Program,
//! exactly as it must on-chain. `set_account` appears only to model state
//! that exists outside the program: the loader's upgrade authority, lamports
//! sent to an address in advance, and look-alike accounts.

use axis_core::constants::{
    BPF_LOADER_UPGRADEABLE_ID, MARKET_SEED, PROTOCOL_CONFIG_SEED, SYSTEM_PROGRAM_ID,
};
use axis_core::error::AxisCoreError;
use axis_core::state::{DTFMarket, MarketStatus, ProtocolConfig, TokenProgramKind};
use axis_core_test_utils::{
    fresh_vm, load_axis_core_program, provision_deterministic_payer, provision_funded_signer,
};
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

fn address_of(k: &Keypair) -> Address {
    Address::try_from(k.pubkey().as_ref()).unwrap()
}

fn program_data_address(program_id: &Address) -> Address {
    Address::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID).0
}

fn config_address(program_id: &Address) -> Address {
    Address::find_program_address(&[PROTOCOL_CONFIG_SEED], program_id).0
}

fn market_address(program_id: &Address, dtf_mint: &Address) -> (Address, u8) {
    Address::find_program_address(&[MARKET_SEED, dtf_mint.as_ref()], program_id)
}

/// LiteSVM deploys with no upgrade authority, which is what a finalized
/// program looks like. A real deployment has one until it is finalized.
fn set_upgrade_authority(vm: &mut LiteSVM, program_id: &Address, authority: &Address) {
    let address = program_data_address(program_id);
    let mut account = vm.get_account(&address).expect("program data account");
    account.data[12] = 1;
    account.data[13..45].copy_from_slice(authority.as_ref());
    vm.set_account(address, account)
        .expect("set upgrade authority");
}

/// A loaded program whose upgrade authority is the returned payer.
fn setup() -> (LiteSVM, Keypair, Address) {
    let mut vm = fresh_vm();
    let payer = provision_deterministic_payer(&mut vm).expect("payer");
    let load = load_axis_core_program(&mut vm)
        .expect("target/deploy/axis_core.so must exist; build it with scripts/build-sbf.sh");
    set_upgrade_authority(&mut vm, &load.program_id, &address_of(&payer.keypair));
    (vm, payer.keypair, load.program_id)
}

type TxResult = Result<(), litesvm::types::FailedTransactionMetadata>;

fn send(vm: &mut LiteSVM, payer: &Keypair, ix: Instruction) -> TxResult {
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], message, vm.latest_blockhash());
    vm.send_transaction(tx).map(|_| ())
}

fn assert_fails_with(result: TxResult, want: AxisCoreError) {
    let err = result.expect_err("transaction must fail");
    let code = format!("Custom({})", want as u32);
    assert!(
        format!("{err:?}").contains(&code),
        "expected {want:?} ({code}), got {err:?}"
    );
}

const USDC: [u8; 32] = [9u8; 32];
const TREASURY: [u8; 32] = [8u8; 32];
const DTF_MINT: [u8; 32] = [5u8; 32];

fn init_config_ix_at(
    program_id: &Address,
    authority: &Address,
    config: &Address,
    program_data: &Address,
) -> Instruction {
    let mut data = vec![0u8]; // InitializeProtocolConfig
    data.extend_from_slice(&USDC);
    data.extend_from_slice(&TREASURY);
    Instruction {
        program_id: to_pubkey(program_id),
        accounts: vec![
            AccountMeta::new(to_pubkey(authority), true),
            AccountMeta::new(to_pubkey(config), false),
            AccountMeta::new_readonly(to_pubkey(&SYSTEM_PROGRAM_ID), false),
            AccountMeta::new_readonly(to_pubkey(program_data), false),
        ],
        data,
    }
}

fn init_config_ix(program_id: &Address, authority: &Address) -> Instruction {
    init_config_ix_at(
        program_id,
        authority,
        &config_address(program_id),
        &program_data_address(program_id),
    )
}

struct Entry {
    mint: [u8; 32],
    vault: [u8; 32],
    weight: u16,
}

fn three_assets() -> Vec<Entry> {
    [(11, 21, 5_000), (12, 22, 3_000), (13, 23, 2_000)]
        .into_iter()
        .map(|(m, v, weight)| Entry {
            mint: [m; 32],
            vault: [v; 32],
            weight,
        })
        .collect()
}

fn create_market_ix_with_config(
    program_id: &Address,
    creator: &Address,
    config: &Address,
    entries: &[Entry],
) -> Instruction {
    let mut data = vec![1u8]; // CreateMarket
    data.push(entries.len() as u8);
    for e in entries {
        data.extend_from_slice(&e.mint);
        data.extend_from_slice(&e.vault);
        data.extend_from_slice(&e.weight.to_le_bytes());
        data.push(TokenProgramKind::LegacySplToken as u8);
    }
    let dtf_mint = Address::new_from_array(DTF_MINT);
    let (market, _) = market_address(program_id, &dtf_mint);
    Instruction {
        program_id: to_pubkey(program_id),
        accounts: vec![
            AccountMeta::new(to_pubkey(creator), true),
            AccountMeta::new_readonly(to_pubkey(config), false),
            AccountMeta::new(to_pubkey(&market), false),
            AccountMeta::new_readonly(to_pubkey(&dtf_mint), false),
            AccountMeta::new_readonly(to_pubkey(&SYSTEM_PROGRAM_ID), false),
        ],
        data,
    }
}

fn create_market_ix(program_id: &Address, creator: &Address, entries: &[Entry]) -> Instruction {
    create_market_ix_with_config(program_id, creator, &config_address(program_id), entries)
}

fn assert_created_by_program(vm: &LiteSVM, address: &Address, program_id: &Address, len: usize) {
    let account = vm
        .get_account(address)
        .expect("account exists after the instruction");
    assert_eq!(&account.owner, program_id, "owned by Axis Core");
    assert_eq!(account.data.len(), len, "allocated to the documented size");
    assert!(
        account.lamports >= vm.minimum_balance_for_rent_exemption(len),
        "rent-exempt"
    );
}

#[test]
fn the_upgrade_authority_creates_protocol_config_on_chain() {
    let (mut vm, payer, program_id) = setup();
    let config = config_address(&program_id);
    assert!(
        vm.get_account(&config).is_none(),
        "nothing exists before init"
    );

    let authority = address_of(&payer);
    send(&mut vm, &payer, init_config_ix(&program_id, &authority)).expect("init");

    assert_created_by_program(&vm, &config, &program_id, ProtocolConfig::LEN);
    let state = ProtocolConfig::unpack(&vm.get_account(&config).unwrap().data).expect("decode");
    assert_eq!(state.protocol_authority, authority);
    assert_eq!(state.usdc_mint.to_bytes(), USDC);
    assert_eq!(state.protocol_treasury.to_bytes(), TREASURY);
    assert_eq!(
        state.bump,
        Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id).1
    );
}

#[test]
fn protocol_config_cannot_be_initialized_twice() {
    let (mut vm, payer, program_id) = setup();
    let ix = init_config_ix(&program_id, &address_of(&payer));
    send(&mut vm, &payer, ix.clone()).expect("first init");
    vm.expire_blockhash();
    assert_fails_with(
        send(&mut vm, &payer, ix),
        AxisCoreError::AccountAlreadyInitialized,
    );
}

/// Without this, whoever lands the first transaction after deployment owns
/// the protocol.
#[test]
fn only_the_upgrade_authority_can_initialize() {
    let (mut vm, _payer, program_id) = setup();
    let intruder = provision_funded_signer(&mut vm, "intruder", 1_000_000_000).expect("fund");
    let ix = init_config_ix(&program_id, &intruder.pubkey());
    assert_fails_with(
        send(&mut vm, &intruder.keypair, ix),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );
}

#[test]
fn a_program_without_an_upgrade_authority_cannot_be_initialized() {
    let mut vm = fresh_vm();
    let payer = provision_deterministic_payer(&mut vm).expect("payer");
    let program_id = load_axis_core_program(&mut vm).expect("load").program_id;
    let ix = init_config_ix(&program_id, &payer.pubkey());
    assert_fails_with(
        send(&mut vm, &payer.keypair, ix),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );
}

#[test]
fn a_substitute_program_data_account_is_rejected() {
    let (mut vm, payer, program_id) = setup();
    let ix = init_config_ix_at(
        &program_id,
        &address_of(&payer),
        &config_address(&program_id),
        &Address::new_from_array([6u8; 32]),
    );
    assert_fails_with(
        send(&mut vm, &payer, ix),
        AxisCoreError::InvalidProgramAccount,
    );
}

/// Every off-curve bump derives a valid PDA. Accepting a non-canonical one
/// would allow a second `protocol_config`.
#[test]
fn a_non_canonical_config_address_is_rejected() {
    let (mut vm, payer, program_id) = setup();
    let (_, canonical) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &program_id);
    let other = (0..canonical)
        .rev()
        .find_map(|b| {
            Address::create_program_address(&[PROTOCOL_CONFIG_SEED, &[b]], &program_id).ok()
        })
        .expect("some lower bump is off the curve");

    let ix = init_config_ix_at(
        &program_id,
        &address_of(&payer),
        &other,
        &program_data_address(&program_id),
    );
    assert_fails_with(send(&mut vm, &payer, ix), AxisCoreError::InvalidPda);
}

/// Anyone can send lamports to the config address before it exists; the
/// cheapest transfer the runtime accepts is the rent minimum for an empty
/// account. System `CreateAccount` refuses a funded address, which would block
/// the singleton forever if the program relied on it alone. Both a top-up and
/// an over-funded address are covered.
#[test]
fn a_pre_funded_config_address_still_initializes() {
    let cheapest = fresh_vm().minimum_balance_for_rent_exemption(0);
    for lamports in [cheapest, 10_000_000_000] {
        let (mut vm, payer, program_id) = setup();
        let config = config_address(&program_id);
        vm.airdrop(&config, lamports).expect("pre-fund the PDA");

        send(
            &mut vm,
            &payer,
            init_config_ix(&program_id, &address_of(&payer)),
        )
        .unwrap_or_else(|e| panic!("init with {lamports} lamports pre-funded: {e:?}"));
        assert_created_by_program(&vm, &config, &program_id, ProtocolConfig::LEN);
    }
}

fn setup_with_config() -> (LiteSVM, Keypair, Address) {
    let (mut vm, payer, program_id) = setup();
    send(
        &mut vm,
        &payer,
        init_config_ix(&program_id, &address_of(&payer)),
    )
    .expect("config");
    vm.expire_blockhash();
    (vm, payer, program_id)
}

#[test]
fn create_market_creates_the_account_and_writes_the_inline_asset_table() {
    let (mut vm, payer, program_id) = setup_with_config();
    let creator = address_of(&payer);
    let dtf_mint = Address::new_from_array(DTF_MINT);
    let (market, bump) = market_address(&program_id, &dtf_mint);
    assert!(
        vm.get_account(&market).is_none(),
        "nothing exists before CreateMarket"
    );

    let entries = three_assets();
    send(
        &mut vm,
        &payer,
        create_market_ix(&program_id, &creator, &entries),
    )
    .expect("create market");

    assert_created_by_program(&vm, &market, &program_id, DTFMarket::LEN);
    let state = DTFMarket::unpack(&vm.get_account(&market).unwrap().data).expect("decode");
    assert_eq!(state.creator, creator);
    assert_eq!(state.dtf_mint, dtf_mint);
    assert_eq!(
        state.treasury.to_bytes(),
        TREASURY,
        "treasury snapshotted from config"
    );
    assert_eq!(state.status, MarketStatus::Created);
    assert_eq!(state.bump, bump);

    let live: Vec<_> = state.assets().collect();
    assert_eq!(live.len(), 3);
    for (a, e) in live.iter().zip(&entries) {
        assert_eq!(a.asset_mint.to_bytes(), e.mint);
        assert_eq!(a.reserve_vault.to_bytes(), e.vault);
        assert_eq!(a.weight_bps, e.weight);
    }
    state.validate_composition().expect("composition");
}

#[test]
fn a_dtf_mint_gets_at_most_one_market() {
    let (mut vm, payer, program_id) = setup_with_config();
    let ix = create_market_ix(&program_id, &address_of(&payer), &three_assets());
    send(&mut vm, &payer, ix.clone()).expect("first market");
    vm.expire_blockhash();
    assert_fails_with(
        send(&mut vm, &payer, ix),
        AxisCoreError::AccountAlreadyInitialized,
    );
}

#[test]
fn create_market_rejects_a_four_asset_composition() {
    let (mut vm, payer, program_id) = setup_with_config();
    let entries: Vec<Entry> = (0..4u8)
        .map(|i| Entry {
            mint: [11 + i; 32],
            vault: [21 + i; 32],
            weight: 2_500,
        })
        .collect();
    let ix = create_market_ix(&program_id, &address_of(&payer), &entries);
    assert_fails_with(send(&mut vm, &payer, ix), AxisCoreError::TooManyAssets);
}

/// A config the caller controls would let them choose the treasury their
/// market's fees go to.
#[test]
fn create_market_rejects_a_config_account_it_does_not_own() {
    let (mut vm, payer, program_id) = setup_with_config();
    let fake = Address::new_from_array([7u8; 32]);
    let real = vm.get_account(&config_address(&program_id)).unwrap();
    vm.set_account(
        fake.clone(),
        Account {
            owner: Address::new_from_array([66u8; 32]),
            ..real
        },
    )
    .expect("look-alike owned by another program");

    let ix = create_market_ix_with_config(&program_id, &address_of(&payer), &fake, &three_assets());
    assert_fails_with(
        send(&mut vm, &payer, ix),
        AxisCoreError::InvalidAccountOwner,
    );
}

/// Once PDAs use the canonical bump, no second Axis-owned config can be
/// created. The address is still checked rather than assumed.
#[test]
fn create_market_rejects_an_axis_owned_config_away_from_the_singleton_address() {
    let (mut vm, payer, program_id) = setup_with_config();
    let elsewhere = Address::new_from_array([7u8; 32]);
    let real = vm.get_account(&config_address(&program_id)).unwrap();
    vm.set_account(elsewhere.clone(), real)
        .expect("copy at another address");

    let ix = create_market_ix_with_config(
        &program_id,
        &address_of(&payer),
        &elsewhere,
        &three_assets(),
    );
    assert_fails_with(send(&mut vm, &payer, ix), AxisCoreError::InvalidPda);
}
