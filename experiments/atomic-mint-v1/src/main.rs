use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use litesvm::LiteSVM;
use serde::Deserialize;
use solana_account::Account;
use solana_address::Address as RuntimeAddress;
use solana_address_lookup_table_interface::state::AddressLookupTable;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_hash::Hash;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_message::{v0, AddressLookupTableAccount, Message, VersionedMessage};
use solana_pubkey::Pubkey;
use solana_transaction::{versioned::VersionedTransaction, Transaction};

const PACKET_LIMIT: usize = 1_232;
const ACCOUNT_LOCK_LIMIT: usize = 64;
const AXIS_AMM_FIXTURE: &str = "test/fixtures/jupiter/sol-usdc-100m.json";
const AXIS_AMM_ALT_DIR: &str = "test/fixtures/jupiter/accounts";
const JUPITER_USER_PLACEHOLDER: &str = "11111111111111111111111111111112";
const DIRECT_ROUTE_ID: [u8; 32] = [201; 32];
const HYP_MINT_ID: [u8; 32] = [202; 32];
const DIRECT_ROUTE_ARTIFACT: &str = "target/deploy/axis_direct_route_exp.so";
const HYP_MINT_ARTIFACT: &str = "target/deploy/axis_hyp_mint_exp.so";

#[derive(Clone, Copy)]
enum Operation {
    Mint,
    Redeem,
    RedeemInKindSponsored,
}

#[derive(Clone, Copy)]
enum AccountModel {
    DeliveryInline,
    NavExternal,
}

struct Args {
    route_accounts: usize,
    route_data_bytes: usize,
    axis_amm: PathBuf,
    axis_amm_was_explicit: bool,
}

struct Scenario {
    name: String,
    operation: Operation,
    model: AccountModel,
    asset_count: usize,
    route_account_count: usize,
    route_data_bytes: usize,
}

struct Measurement {
    scenario: String,
    format: &'static str,
    resolved_locks: usize,
    static_keys: usize,
    looked_up_keys: usize,
    packet_bytes: usize,
}

struct KeySource(u8);

impl KeySource {
    fn next(&mut self) -> Pubkey {
        self.0 = self
            .0
            .checked_add(1)
            .expect("experiment key space exhausted");
        Pubkey::new_from_array([self.0; 32])
    }
}

#[derive(Deserialize)]
struct JupiterFixture {
    swap: JupiterSwap,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JupiterSwap {
    compute_budget_instructions: Vec<SerializedInstruction>,
    swap_instruction: SerializedInstruction,
    address_lookup_table_addresses: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedInstruction {
    program_id: String,
    accounts: Vec<SerializedAccountMeta>,
    data: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerializedAccountMeta {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

#[derive(Deserialize)]
struct AccountDump {
    account: DumpedAccount,
}

#[derive(Deserialize)]
struct DumpedAccount {
    data: Vec<String>,
}

struct CapturedRoute {
    compute_budget: Vec<Instruction>,
    swap: Instruction,
    lookup_tables: Vec<AddressLookupTableAccount>,
    route_meta_count: usize,
    route_data_bytes: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;

    println!("HYP-MINT-011 — transaction-envelope experiment");
    println!("Limits: {ACCOUNT_LOCK_LIMIT} resolved account locks, {PACKET_LIMIT} signed bytes");
    println!();

    let fixture_path = args.axis_amm.join(AXIS_AMM_FIXTURE);
    if fixture_path.exists() {
        let captured = load_axis_amm_capture(&args.axis_amm)?;
        run_captured_route(&captured)?;
    } else if args.axis_amm_was_explicit {
        return Err(format!("Axis_AMM fixture not found: {}", fixture_path.display()).into());
    } else {
        println!("Axis_AMM captured-route section: SKIPPED");
        println!("  expected sibling checkout at {}", args.axis_amm.display());
        println!("  pass --axis-amm /path/to/Axis_AMM to enable it");
        println!();
    }

    run_controlled_sbf_experiment()?;
    run_synthetic_matrix(args.route_accounts, args.route_data_bytes)?;

    println!();
    println!("Interpretation boundary:");
    println!("- FIT proves only that a signed transaction envelope can be serialized.");
    println!(
        "- Axis_AMM's split fallback leaves a recoverable intermediate state; it is not atomic."
    );
    println!("- PR #10 still governs N=3: 61-67 locks, with 4/11 clean samples over 64.");
    println!("- Jupiter CPI execution/CU still needs a real fork or production-venue replay.");
    println!("- An ALT account itself is packet metadata, not an extra execution lock.");
    Ok(())
}

struct ControlledCase {
    name: &'static str,
    spend: u64,
    delivery: u64,
    expected_error: Option<u32>,
}

struct ControlledResult {
    name: &'static str,
    compute_units: u64,
    locks: usize,
    packet_bytes: usize,
    outcome: &'static str,
}

fn run_controlled_sbf_experiment() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let route_artifact = root.join(DIRECT_ROUTE_ARTIFACT);
    let mint_artifact = root.join(HYP_MINT_ARTIFACT);
    if !route_artifact.exists() || !mint_artifact.exists() {
        println!("Controlled SBF/LiteSVM section: SKIPPED");
        println!("  run ./experiments/atomic-mint-v1/build-programs.sh first");
        println!();
        return Ok(());
    }

    println!("Controlled SBF/LiteSVM atomic Mint CPI");
    println!("required delivery = ceil(5,000 reserve * 100 DTF / 1,000 supply) = 500");
    println!(
        "{:<31} {:>9} {:>7} {:>7} {:>19}",
        "case", "CU", "locks", "bytes", "outcome"
    );
    for case in [
        ControlledCase {
            name: "exact delivery",
            spend: 900,
            delivery: 500,
            expected_error: None,
        },
        ControlledCase {
            name: "under-delivery rollback",
            spend: 900,
            delivery: 499,
            expected_error: Some(2),
        },
        ControlledCase {
            name: "max-input rollback",
            spend: 1_001,
            delivery: 500,
            expected_error: Some(3),
        },
    ] {
        let result = execute_controlled_case(case, &route_artifact, &mint_artifact)?;
        println!(
            "{:<31} {:>9} {:>7} {:>7} {:>19}",
            result.name, result.compute_units, result.locks, result.packet_bytes, result.outcome
        );
    }
    println!();
    Ok(())
}

fn execute_controlled_case(
    case: ControlledCase,
    route_artifact: &Path,
    mint_artifact: &Path,
) -> Result<ControlledResult, Box<dyn Error>> {
    const USDC_PRE: u64 = 10_000;
    const SUPPLY_PRE: u64 = 1_000;
    const RESERVE_PRE: u64 = 5_000;
    const DTF_PRE: u64 = 50;
    const DTF_OUT: u64 = 100;
    const MAX_USDC_IN: u64 = 1_000;

    let mut vm = LiteSVM::new();
    let payer = deterministic_signer(240);
    let payer_runtime = RuntimeAddress::new_from_array(payer.pubkey().to_bytes());
    vm.airdrop(&payer_runtime, 10_000_000_000)
        .map_err(|error| format!("failed to fund controlled signer: {error:?}"))?;
    vm.add_program_from_file(
        RuntimeAddress::new_from_array(DIRECT_ROUTE_ID),
        route_artifact,
    )
    .map_err(|error| format!("failed to load direct-route SBF: {error}"))?;
    vm.add_program_from_file(RuntimeAddress::new_from_array(HYP_MINT_ID), mint_artifact)
        .map_err(|error| format!("failed to load HYP Mint SBF: {error}"))?;

    let user_usdc = Pubkey::new_from_array([211; 32]);
    let market = Pubkey::new_from_array([212; 32]);
    let reserve = Pubkey::new_from_array([213; 32]);
    let user_dtf = Pubkey::new_from_array([214; 32]);
    set_balance_account(&mut vm, user_usdc, DIRECT_ROUTE_ID, USDC_PRE)?;
    set_balance_account(&mut vm, reserve, DIRECT_ROUTE_ID, RESERVE_PRE)?;
    set_balance_account(&mut vm, market, HYP_MINT_ID, SUPPLY_PRE)?;
    set_balance_account(&mut vm, user_dtf, HYP_MINT_ID, DTF_PRE)?;

    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&DTF_OUT.to_le_bytes());
    data.extend_from_slice(&MAX_USDC_IN.to_le_bytes());
    data.extend_from_slice(&case.spend.to_le_bytes());
    data.extend_from_slice(&case.delivery.to_le_bytes());
    let ix = Instruction {
        program_id: Pubkey::new_from_array(HYP_MINT_ID),
        accounts: vec![
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new(user_usdc, false),
            AccountMeta::new(market, false),
            AccountMeta::new(reserve, false),
            AccountMeta::new(user_dtf, false),
            AccountMeta::new_readonly(Pubkey::new_from_array(DIRECT_ROUTE_ID), false),
        ],
        data,
    };
    let instructions = [
        ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        ix,
    ];
    let message =
        Message::new_with_blockhash(&instructions, Some(&payer.pubkey()), &vm.latest_blockhash());
    let tx = Transaction::new(&[&payer], message, vm.latest_blockhash());
    let packet_bytes = bincode::serialize(&tx)?.len();
    let locks = resolved_lock_count(&instructions, payer.pubkey());

    let simulation = vm.simulate_transaction(tx.clone());
    let (compute_units, outcome) = match case.expected_error {
        None => {
            let simulated = simulation.map_err(|error| {
                format!("successful controlled case failed simulation: {error:?}")
            })?;
            vm.send_transaction(tx)
                .map_err(|error| format!("successful controlled case failed: {error:?}"))?;
            assert_eq!(read_vm_balance(&vm, user_usdc)?, USDC_PRE - case.spend);
            assert_eq!(read_vm_balance(&vm, reserve)?, RESERVE_PRE + case.delivery);
            assert_eq!(read_vm_balance(&vm, market)?, SUPPLY_PRE + DTF_OUT);
            assert_eq!(read_vm_balance(&vm, user_dtf)?, DTF_PRE + DTF_OUT);
            (simulated.meta.compute_units_consumed, "committed")
        }
        Some(expected_error) => {
            let failed = simulation.expect_err("controlled failure must fail simulation");
            let detail = format!("{:?}", failed.err);
            if !detail.contains(&format!("Custom({expected_error})")) {
                return Err(format!(
                    "{} returned wrong error: expected Custom({expected_error}), got {detail}",
                    case.name
                )
                .into());
            }
            vm.send_transaction(tx)
                .expect_err("controlled failure must fail execution");
            assert_eq!(read_vm_balance(&vm, user_usdc)?, USDC_PRE);
            assert_eq!(read_vm_balance(&vm, reserve)?, RESERVE_PRE);
            assert_eq!(read_vm_balance(&vm, market)?, SUPPLY_PRE);
            assert_eq!(read_vm_balance(&vm, user_dtf)?, DTF_PRE);
            (failed.meta.compute_units_consumed, "rolled back")
        }
    };

    Ok(ControlledResult {
        name: case.name,
        compute_units,
        locks,
        packet_bytes,
        outcome,
    })
}

fn set_balance_account(
    vm: &mut LiteSVM,
    address: Pubkey,
    owner: [u8; 32],
    value: u64,
) -> Result<(), Box<dyn Error>> {
    vm.set_account(
        RuntimeAddress::new_from_array(address.to_bytes()),
        Account {
            lamports: 10_000_000,
            data: value.to_le_bytes().to_vec(),
            owner: RuntimeAddress::new_from_array(owner),
            executable: false,
            rent_epoch: 0,
        },
    )
    .map_err(|error| format!("failed to inject controlled account: {error}"))?;
    Ok(())
}

fn read_vm_balance(vm: &LiteSVM, address: Pubkey) -> Result<u64, Box<dyn Error>> {
    let account = vm
        .get_account(&RuntimeAddress::new_from_array(address.to_bytes()))
        .ok_or_else(|| format!("controlled account disappeared: {address}"))?;
    let bytes: [u8; 8] = account
        .data
        .get(..8)
        .ok_or("controlled account data is truncated")?
        .try_into()?;
    Ok(u64::from_le_bytes(bytes))
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut route_accounts = 40;
    let mut route_data_bytes = 256;
    let mut axis_amm = default_axis_amm_path();
    let mut axis_amm_was_explicit = false;
    let mut cli = std::env::args().skip(1);

    while let Some(flag) = cli.next() {
        let raw = cli
            .next()
            .ok_or_else(|| format!("missing value after {flag}"))?;
        match flag.as_str() {
            "--route-accounts" => route_accounts = raw.parse()?,
            "--route-data-bytes" => route_data_bytes = raw.parse()?,
            "--axis-amm" => {
                axis_amm = PathBuf::from(raw);
                axis_amm_was_explicit = true;
            }
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }

    if route_accounts > 50 {
        return Err("route accounts must be <= 50 so synthetic keys stay unambiguous".into());
    }
    if route_data_bytes > 900 {
        return Err("route data bytes must be <= 900".into());
    }
    Ok(Args {
        route_accounts,
        route_data_bytes,
        axis_amm,
        axis_amm_was_explicit,
    })
}

fn default_axis_amm_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("experiment must live under Axis_core/experiments")
        .join("Axis_AMM")
}

fn run_captured_route(captured: &CapturedRoute) -> Result<(), Box<dyn Error>> {
    let payer = deterministic_signer(240);
    let blockhash = Hash::new_from_array([7; 32]);
    let core_verify = captured_core_mint_instruction(&payer, &captured.swap, false);
    let core_cpi = captured_core_mint_instruction(&payer, &captured.swap, true);

    println!("Axis_AMM recorded Jupiter route (SOL -> USDC, 100M lamports)");
    println!(
        "source shape: {} metas, {} route-data bytes, {} ALT(s)",
        captured.route_meta_count,
        captured.route_data_bytes,
        captured.lookup_tables.len()
    );
    print_header();

    let mut swap_only = captured.compute_budget.clone();
    swap_only.push(captured.swap.clone());
    print_measurements(measure_instructions(
        "axis-amm-split-tx0-swap",
        &swap_only,
        &captured.lookup_tables,
        &payer,
        &[&payer],
        blockhash,
    )?);

    let mut bundled = captured.compute_budget.clone();
    bundled.push(captured.swap.clone());
    bundled.push(core_verify.clone());
    print_measurements(measure_instructions(
        "hyp-mint-top-level-atomic-n1",
        &bundled,
        &captured.lookup_tables,
        &payer,
        &[&payer],
        blockhash,
    )?);

    let mut cpi = captured.compute_budget.clone();
    cpi.push(core_cpi);
    print_measurements(measure_instructions(
        "hyp-mint-core-cpi-atomic-n1",
        &cpi,
        &captured.lookup_tables,
        &payer,
        &[&payer],
        blockhash,
    )?);

    let split_mint = [
        ComputeBudgetInstruction::set_compute_unit_limit(200_000),
        core_verify,
    ];
    print_measurements(measure_instructions(
        "axis-amm-split-tx1-mint",
        &split_mint,
        &[],
        &payer,
        &[&payer],
        blockhash,
    )?);
    println!();
    Ok(())
}

fn captured_core_mint_instruction(
    payer: &Keypair,
    route: &Instruction,
    include_cpi_route: bool,
) -> Instruction {
    let mut keys = KeySource(90);
    let core_program = keys.next();
    let reserve = route
        .accounts
        .iter()
        .find(|meta| meta.is_writable && !meta.is_signer)
        .map(|meta| meta.pubkey)
        .unwrap_or_else(|| keys.next());
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .expect("valid token program id");

    let mut accounts = vec![
        AccountMeta::new_readonly(payer.pubkey(), true),
        AccountMeta::new(keys.next(), false),
        AccountMeta::new(keys.next(), false),
        AccountMeta::new(keys.next(), false),
        AccountMeta::new(keys.next(), false),
        AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(token_program, false),
    ];
    let mut data = vec![2];
    data.extend_from_slice(&1_000_000u64.to_le_bytes());
    data.extend_from_slice(&100_000_000u64.to_le_bytes());

    if include_cpi_route {
        accounts.push(AccountMeta::new_readonly(route.program_id, false));
        accounts.extend(route.accounts.iter().cloned());
        data.extend_from_slice(&(route.data.len() as u16).to_le_bytes());
        data.extend_from_slice(&route.data);
    }

    Instruction {
        program_id: core_program,
        accounts,
        data,
    }
}

fn load_axis_amm_capture(root: &Path) -> Result<CapturedRoute, Box<dyn Error>> {
    let fixture: JupiterFixture =
        serde_json::from_str(&fs::read_to_string(root.join(AXIS_AMM_FIXTURE))?)?;
    let placeholder = Pubkey::from_str(JUPITER_USER_PLACEHOLDER)?;
    let payer = deterministic_signer(240);

    let decode_ix = |raw: &SerializedInstruction| -> Result<Instruction, Box<dyn Error>> {
        let accounts = raw
            .accounts
            .iter()
            .map(|meta| {
                let parsed = Pubkey::from_str(&meta.pubkey)?;
                let pubkey = if parsed == placeholder {
                    payer.pubkey()
                } else {
                    parsed
                };
                Ok(if meta.is_writable {
                    AccountMeta::new(pubkey, meta.is_signer)
                } else {
                    AccountMeta::new_readonly(pubkey, meta.is_signer)
                })
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
        Ok(Instruction {
            program_id: Pubkey::from_str(&raw.program_id)?,
            accounts,
            data: BASE64.decode(&raw.data)?,
        })
    };

    let compute_budget = fixture
        .swap
        .compute_budget_instructions
        .iter()
        .map(decode_ix)
        .collect::<Result<Vec<_>, _>>()?;
    let swap = decode_ix(&fixture.swap.swap_instruction)?;
    let route_meta_count = swap.accounts.len();
    let route_data_bytes = swap.data.len();

    let lookup_tables = fixture
        .swap
        .address_lookup_table_addresses
        .iter()
        .map(|address| load_lookup_table(root, address))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CapturedRoute {
        compute_budget,
        swap,
        lookup_tables,
        route_meta_count,
        route_data_bytes,
    })
}

fn load_lookup_table(
    root: &Path,
    address: &str,
) -> Result<AddressLookupTableAccount, Box<dyn Error>> {
    let dump_path = root.join(AXIS_AMM_ALT_DIR).join(format!("{address}.json"));
    let dump: AccountDump = serde_json::from_str(&fs::read_to_string(&dump_path)?)?;
    let encoded = dump
        .account
        .data
        .first()
        .ok_or_else(|| format!("ALT dump has no account data: {}", dump_path.display()))?;
    let bytes = BASE64.decode(encoded)?;
    let table = AddressLookupTable::deserialize(&bytes)
        .map_err(|error| format!("invalid ALT dump {}: {error:?}", dump_path.display()))?;
    Ok(AddressLookupTableAccount {
        key: Pubkey::from_str(address)?,
        addresses: table.addresses.into_owned(),
    })
}

fn run_synthetic_matrix(
    route_accounts: usize,
    route_data_bytes: usize,
) -> Result<(), Box<dyn Error>> {
    let mut scenarios = Vec::new();
    for asset_count in [2, 3] {
        for (name, model) in [
            ("delivery-inline", AccountModel::DeliveryInline),
            ("nav-external", AccountModel::NavExternal),
        ] {
            scenarios.push(Scenario {
                name: format!("mint-{name}-n{asset_count}"),
                operation: Operation::Mint,
                model,
                asset_count,
                route_account_count: route_accounts,
                route_data_bytes,
            });
            scenarios.push(Scenario {
                name: format!("redeem-{name}-n{asset_count}"),
                operation: Operation::Redeem,
                model,
                asset_count,
                route_account_count: route_accounts,
                route_data_bytes,
            });
        }

        scenarios.push(Scenario {
            name: format!("redeem-in-kind-sponsored-inline-n{asset_count}"),
            operation: Operation::RedeemInKindSponsored,
            model: AccountModel::DeliveryInline,
            asset_count,
            route_account_count: 0,
            route_data_bytes: 0,
        });
    }

    println!("N=2/3 controlled projection (synthetic; not production-route evidence)");
    println!("route controls: {route_accounts} accounts, {route_data_bytes} data bytes");
    print_header();
    for scenario in scenarios {
        for measurement in measure_synthetic(&scenario)? {
            print_measurement(&measurement);
        }
    }
    Ok(())
}

fn print_header() {
    println!(
        "{:<47} {:<7} {:>5} {:>6} {:>6} {:>7} {:>7}",
        "scenario", "format", "locks", "static", "lookup", "bytes", "verdict"
    );
}

fn print_measurements(measurements: [Measurement; 2]) {
    for measurement in measurements {
        print_measurement(&measurement);
    }
}

fn print_measurement(measurement: &Measurement) {
    let fits = measurement.resolved_locks <= ACCOUNT_LOCK_LIMIT
        && measurement.packet_bytes <= PACKET_LIMIT;
    println!(
        "{:<47} {:<7} {:>5} {:>6} {:>6} {:>7} {:>7}",
        measurement.scenario,
        measurement.format,
        measurement.resolved_locks,
        measurement.static_keys,
        measurement.looked_up_keys,
        measurement.packet_bytes,
        if fits { "FIT" } else { "OVER" }
    );
}

fn measure_synthetic(scenario: &Scenario) -> Result<[Measurement; 2], Box<dyn Error>> {
    let payer = deterministic_signer(240);
    let user = deterministic_signer(241);
    let blockhash = Hash::new_from_array([7; 32]);
    let (instructions, lookup_candidates) = build_synthetic_instructions(scenario, &payer, &user);
    let lookup = AddressLookupTableAccount {
        key: Pubkey::new_from_array([239; 32]),
        addresses: lookup_candidates,
    };
    let signer_refs: Vec<&Keypair> = match scenario.operation {
        Operation::RedeemInKindSponsored => vec![&payer, &user],
        Operation::Mint | Operation::Redeem => vec![&payer],
    };
    measure_instructions(
        &scenario.name,
        &instructions,
        &[lookup],
        &payer,
        &signer_refs,
        blockhash,
    )
}

fn measure_instructions(
    name: &str,
    instructions: &[Instruction],
    lookup_tables: &[AddressLookupTableAccount],
    payer: &Keypair,
    signers: &[&Keypair],
    blockhash: Hash,
) -> Result<[Measurement; 2], Box<dyn Error>> {
    let resolved_locks = resolved_lock_count(instructions, payer.pubkey());
    let legacy_message =
        Message::new_with_blockhash(instructions, Some(&payer.pubkey()), &blockhash);
    let legacy_static = legacy_message.account_keys.len();
    let legacy_tx = Transaction::new(signers, legacy_message, blockhash);
    let legacy_bytes = bincode::serialize(&legacy_tx)?.len();

    let v0_message =
        v0::Message::try_compile(&payer.pubkey(), instructions, lookup_tables, blockhash)?;
    let v0_static = v0_message.account_keys.len();
    let v0_looked_up = v0_message
        .address_table_lookups
        .iter()
        .map(|item| item.writable_indexes.len() + item.readonly_indexes.len())
        .sum();
    let v0_tx = VersionedTransaction::try_new(VersionedMessage::V0(v0_message), signers)?;
    let v0_bytes = bincode::serialize(&v0_tx)?.len();

    Ok([
        Measurement {
            scenario: name.to_owned(),
            format: "legacy",
            resolved_locks,
            static_keys: legacy_static,
            looked_up_keys: 0,
            packet_bytes: legacy_bytes,
        },
        Measurement {
            scenario: name.to_owned(),
            format: "v0+ALT",
            resolved_locks,
            static_keys: v0_static,
            looked_up_keys: v0_looked_up,
            packet_bytes: v0_bytes,
        },
    ])
}

fn build_synthetic_instructions(
    scenario: &Scenario,
    payer: &Keypair,
    user: &Keypair,
) -> (Vec<Instruction>, Vec<Pubkey>) {
    let mut keys = KeySource(0);
    let axis_core = keys.next();
    let token_legacy = keys.next();
    let token_2022 = keys.next();
    let aggregator = keys.next();
    let associated_token = keys.next();
    let system_program = keys.next();

    let mut accounts = Vec::new();
    let mut lookup_candidates = Vec::new();
    let mut push = |meta: AccountMeta| {
        if !meta.is_signer {
            lookup_candidates.push(meta.pubkey);
        }
        accounts.push(meta);
    };

    match scenario.operation {
        Operation::Mint | Operation::Redeem => {
            push(AccountMeta::new_readonly(payer.pubkey(), true));
            push(AccountMeta::new(keys.next(), false));
            push(AccountMeta::new(keys.next(), false));
            push(AccountMeta::new(keys.next(), false));
            push(AccountMeta::new(keys.next(), false));
            push(AccountMeta::new(keys.next(), false));
            for _ in 0..scenario.asset_count {
                push(AccountMeta::new(keys.next(), false));
            }
            push(AccountMeta::new_readonly(token_legacy, false));
            push(AccountMeta::new_readonly(token_2022, false));
            push(AccountMeta::new_readonly(aggregator, false));

            if matches!(scenario.model, AccountModel::NavExternal) {
                push(AccountMeta::new_readonly(keys.next(), false));
                for _ in 0..scenario.asset_count {
                    push(AccountMeta::new_readonly(keys.next(), false));
                    push(AccountMeta::new_readonly(keys.next(), false));
                }
            }
            for i in 0..scenario.route_account_count {
                let key = keys.next();
                if i % 3 == 0 {
                    push(AccountMeta::new(key, false));
                } else {
                    push(AccountMeta::new_readonly(key, false));
                }
            }
        }
        Operation::RedeemInKindSponsored => {
            push(AccountMeta::new_readonly(user.pubkey(), true));
            push(AccountMeta::new_readonly(payer.pubkey(), true));
            push(AccountMeta::new(keys.next(), false));
            push(AccountMeta::new_readonly(keys.next(), false));
            push(AccountMeta::new(keys.next(), false));
            for _ in 0..scenario.asset_count {
                push(AccountMeta::new(keys.next(), false));
                push(AccountMeta::new_readonly(keys.next(), false));
                push(AccountMeta::new(keys.next(), false));
            }
            push(AccountMeta::new_readonly(token_legacy, false));
            push(AccountMeta::new_readonly(token_2022, false));
            push(AccountMeta::new_readonly(associated_token, false));
            push(AccountMeta::new_readonly(system_program, false));
        }
    }

    lookup_candidates.sort_unstable();
    lookup_candidates.dedup();
    lookup_candidates.retain(|key| *key != axis_core);

    let tag = match scenario.operation {
        Operation::Mint => 2,
        Operation::Redeem => 3,
        Operation::RedeemInKindSponsored => 4,
    };
    let mut data = vec![tag];
    data.extend_from_slice(&1_000_000u64.to_le_bytes());
    data.extend_from_slice(&100_000u64.to_le_bytes());
    data.extend(std::iter::repeat_n(0xA5, scenario.route_data_bytes));
    let instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1),
        Instruction {
            program_id: axis_core,
            accounts,
            data,
        },
    ];
    (instructions, lookup_candidates)
}

fn resolved_lock_count(instructions: &[Instruction], payer: Pubkey) -> usize {
    let mut accounts = HashSet::from([payer]);
    for instruction in instructions {
        accounts.insert(instruction.program_id);
        accounts.extend(instruction.accounts.iter().map(|meta| meta.pubkey));
    }
    accounts.len()
}

fn deterministic_signer(seed: u8) -> Keypair {
    Keypair::new_from_array([seed; 32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_inline_core_base_is_eleven_plus_n() {
        let payer = deterministic_signer(7);
        let user = deterministic_signer(8);

        for asset_count in [2, 3] {
            let scenario = Scenario {
                name: format!("mint-delivery-inline-n{asset_count}"),
                operation: Operation::Mint,
                model: AccountModel::DeliveryInline,
                asset_count,
                route_account_count: 0,
                route_data_bytes: 0,
            };

            let (instructions, _) = build_synthetic_instructions(&scenario, &payer, &user);
            assert_eq!(
                resolved_lock_count(&instructions, payer.pubkey()),
                11 + asset_count
            );
        }
    }

    #[test]
    fn nav_external_reversal_costs_one_plus_two_n_accounts() {
        let payer = deterministic_signer(7);
        let user = deterministic_signer(8);

        for asset_count in [2, 3] {
            let delivery_inline = Scenario {
                name: format!("mint-delivery-inline-n{asset_count}"),
                operation: Operation::Mint,
                model: AccountModel::DeliveryInline,
                asset_count,
                route_account_count: 0,
                route_data_bytes: 0,
            };
            let nav_external = Scenario {
                name: format!("mint-nav-external-n{asset_count}"),
                operation: Operation::Mint,
                model: AccountModel::NavExternal,
                asset_count,
                route_account_count: 0,
                route_data_bytes: 0,
            };

            let (delivery_instructions, _) =
                build_synthetic_instructions(&delivery_inline, &payer, &user);
            let (nav_instructions, _) = build_synthetic_instructions(&nav_external, &payer, &user);
            let delivery_locks = resolved_lock_count(&delivery_instructions, payer.pubkey());
            let nav_locks = resolved_lock_count(&nav_instructions, payer.pubkey());

            assert_eq!(nav_locks - delivery_locks, 1 + 2 * asset_count);
        }
    }

    #[test]
    fn controlled_projection_measures_the_signed_wire_transaction() {
        let scenario = Scenario {
            name: "mint-delivery-inline-n3".to_owned(),
            operation: Operation::Mint,
            model: AccountModel::DeliveryInline,
            asset_count: 3,
            route_account_count: 40,
            route_data_bytes: 256,
        };

        let [legacy, v0] = measure_synthetic(&scenario).expect("projection must compile");
        assert!(legacy.packet_bytes > PACKET_LIMIT);
        assert!(v0.packet_bytes <= PACKET_LIMIT);
        assert_eq!(v0.resolved_locks, 54);
        assert!(v0.looked_up_keys > 0);
    }

    #[test]
    fn sponsored_redeem_in_kind_really_carries_two_signatures() {
        let scenario = Scenario {
            name: "redeem-in-kind-sponsored-inline-n3".to_owned(),
            operation: Operation::RedeemInKindSponsored,
            model: AccountModel::DeliveryInline,
            asset_count: 3,
            route_account_count: 0,
            route_data_bytes: 0,
        };

        let [legacy, v0] = measure_synthetic(&scenario).expect("projection must compile");
        assert!(legacy.packet_bytes <= PACKET_LIMIT);
        assert!(v0.packet_bytes <= PACKET_LIMIT);
        assert_eq!(v0.resolved_locks, 20);
    }

    #[test]
    fn local_axis_amm_fixture_decodes_to_the_recorded_shape() {
        let root = default_axis_amm_path();
        if !root.join(AXIS_AMM_FIXTURE).exists() {
            return;
        }

        let captured = load_axis_amm_capture(&root).expect("Axis_AMM fixture must decode");
        assert_eq!(captured.route_meta_count, 21);
        assert_eq!(captured.route_data_bytes, 36);
        assert_eq!(captured.lookup_tables.len(), 1);
        assert_eq!(captured.lookup_tables[0].addresses.len(), 250);
    }

    #[cfg(feature = "sbf-integration")]
    #[test]
    fn controlled_sbf_path_commits_and_rolls_back_atomically() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let route = root.join(DIRECT_ROUTE_ARTIFACT);
        let mint = root.join(HYP_MINT_ARTIFACT);
        assert!(
            route.exists() && mint.exists(),
            "experiment SBF artifacts must exist; run build-programs.sh first"
        );

        let success = execute_controlled_case(
            ControlledCase {
                name: "success",
                spend: 900,
                delivery: 500,
                expected_error: None,
            },
            &route,
            &mint,
        )
        .expect("successful CPI case");
        assert_eq!(success.outcome, "committed");

        let under = execute_controlled_case(
            ControlledCase {
                name: "under-delivery",
                spend: 900,
                delivery: 499,
                expected_error: Some(2),
            },
            &route,
            &mint,
        )
        .expect("under-delivery case");
        assert_eq!(under.outcome, "rolled back");

        let over = execute_controlled_case(
            ControlledCase {
                name: "max-input",
                spend: 1_001,
                delivery: 500,
                expected_error: Some(3),
            },
            &route,
            &mint,
        )
        .expect("max-input case");
        assert_eq!(over.outcome, "rolled back");
    }
}
