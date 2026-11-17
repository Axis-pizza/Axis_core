//! Axis Core V1 through its real instruction entry points, on the SBF
//! artifact, in LiteSVM.
//!
//! These tests only mean anything when `target/deploy/axis_core.so` exists, so
//! they fail loudly rather than skipping if it does not: a silently skipped
//! integration test is how a broken toolchain stays invisible.
//!
//! No test injects an Axis-owned state account. The program allocates its own
//! PDAs, and every DTF in these tests was issued by `InitialPurchase` or
//! `Purchase`. `set_account` appears only to model state that exists outside
//! the program: the loader's upgrade authority, lamports sent to an address in
//! advance, and look-alike or tampered token accounts offered to
//! `CreateMarket`.
//!
//! Every purchase and USDC redemption here swaps through the route stand-in
//! described in `axis_core_test_utils::fixture`: the legacy SPL Token program
//! approved as the market's swap program, and a co-signing maker. They prove
//! Core's accounting, bounds and rollback. They are not evidence about a
//! production Jupiter route. All amounts are fixtures.

use axis_core::constants::{MAX_ROUTE_LEG_ACCOUNTS, PROTOCOL_CONFIG_SEED};
use axis_core::error::AxisCoreError;
use axis_core::instructions::admin::{PAUSE_PURCHASE, PAUSE_USDC_REDEMPTION};
use axis_core::state::{DTFMarket, ProtocolConfig, TokenProgramKind};
use axis_core_test_utils::fixture::{
    address_of, config_address, program_data_address, purchase_legs, redemption_legs, to_pubkey,
    transfer_leg, Env, Leg, MarketFixture, MarketSpec, Trader, TxResult,
};
use axis_core_test_utils::{legacy_token_program_id, token_2022_program_id};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::AccountMeta;
use solana_keypair::Keypair;

fn assert_fails_with(result: TxResult, want: AxisCoreError) {
    let err = result.expect_err("transaction must fail");
    let code = format!("Custom({})", want as u32);
    assert!(
        format!("{:?}", err.err).contains(&code),
        "expected {want:?} ({code}), got {:?}\n{:#?}",
        err.err,
        err.meta.logs
    );
}

fn market_state(env: &Env, market: &MarketFixture) -> DTFMarket {
    DTFMarket::unpack(&env.vm.get_account(&market.market).unwrap().data).expect("decode market")
}

fn config_state(env: &Env) -> ProtocolConfig {
    ProtocolConfig::unpack(
        &env.vm
            .get_account(&config_address(&env.program_id))
            .unwrap()
            .data,
    )
    .expect("decode config")
}

fn tamper(env: &mut Env, address: &Address, change: impl FnOnce(&mut Account)) {
    let mut account = env.vm.get_account(address).expect("account to tamper with");
    change(&mut account);
    env.vm.set_account(*address, account).expect("set account");
}

/// A maker holding plenty of USDC and of every constituent.
fn maker(env: &mut Env, market: &MarketFixture) -> Trader {
    let maker = env.trader("maker", market);
    env.fund_usdc(&maker, 100_000_000);
    let amounts = vec![100_000_000; market.vaults.len()];
    env.fund_assets(market, &maker, &amounts);
    maker
}

fn buyer(env: &mut Env, label: &str, market: &MarketFixture, usdc: u64) -> Trader {
    let trader = env.trader(label, market);
    env.fund_usdc(&trader, usdc);
    trader
}

#[allow(clippy::too_many_arguments)]
fn purchase(
    env: &mut Env,
    market: &MarketFixture,
    initial: bool,
    buyer: &Trader,
    maker: &Trader,
    usdc: u64,
    deliveries: &[u64],
    max_usdc_in: u64,
    min_net_dtf_out: u64,
) -> TxResult {
    let legs = purchase_legs(market, buyer, maker, usdc, deliveries);
    let ix = env.purchase_ix(market, initial, buyer, max_usdc_in, min_net_dtf_out, &legs);
    env.send(&[ix], &[&buyer.keypair, &maker.keypair])
}

const ALICE_USDC: u64 = 1_000_000;
const FIRST_PURCHASE_USDC: u64 = 700_000;

/// The standard funded market: 2 A and 5 B approved per DTF, and Alice's
/// first purchase delivering 200,000 A and 500,000 B.
///
/// ```txt
/// gross 100,000 = lock 1,000 + treasury 300 + Alice 98,700
/// supply 100,000, reserves A = 200,000 and B = 500,000
/// ```
fn funded_market() -> (Env, MarketFixture, Trader, Trader) {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("funded"));
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        FIRST_PURCHASE_USDC,
        &[200_000, 500_000],
        FIRST_PURCHASE_USDC,
        98_700,
    )
    .expect("first purchase");
    (env, market, alice, maker)
}

// ---------------------------------------------------------------------------
// Protocol config
// ---------------------------------------------------------------------------

#[test]
fn the_upgrade_authority_creates_protocol_config_and_names_governance() {
    let mut env = Env::uninitialized();
    let config = config_address(&env.program_id);
    assert!(env.vm.get_account(&config).is_none());

    let deployer = env.deployer.insecure_clone();
    let ix = env.init_config_ix(&address_of(&deployer));
    env.send(&[ix], &[&deployer]).expect("init");

    let account = env.vm.get_account(&config).unwrap();
    assert_eq!(account.owner, env.program_id);
    assert_eq!(account.data.len(), ProtocolConfig::LEN);
    let state = config_state(&env);
    assert_eq!(state.protocol_authority, address_of(&env.governance));
    assert_eq!(state.emergency_operator, address_of(&env.operator));
    assert_eq!(state.protocol_treasury, address_of(&env.treasury));
    assert_eq!(state.usdc_mint, env.usdc_mint);
    assert_eq!(
        state.bump,
        Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &env.program_id).1
    );
}

#[test]
fn protocol_config_cannot_be_initialized_twice() {
    let mut env = Env::new();
    let deployer = env.deployer.insecure_clone();
    let ix = env.init_config_ix(&address_of(&deployer));
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::AccountAlreadyInitialized,
    );
}

/// Without this, whoever lands the first transaction after deployment owns
/// the protocol.
#[test]
fn only_the_upgrade_authority_can_initialize() {
    let mut env = Env::uninitialized();
    let intruder = env.funded_keypair("intruder");
    let ix = env.init_config_ix(&address_of(&intruder));
    assert_fails_with(
        env.send(&[ix], &[&intruder]),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );

    env.set_upgrade_authority(None);
    let deployer = env.deployer.insecure_clone();
    let ix = env.init_config_ix(&address_of(&deployer));
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );
}

#[test]
fn initialization_rejects_substitute_accounts_and_a_zero_governance() {
    let mut env = Env::uninitialized();
    let deployer = env.deployer.insecure_clone();
    let authority = address_of(&deployer);
    let config = config_address(&env.program_id);
    let program_data = program_data_address(&env.program_id);
    let governance = address_of(&env.governance);

    let ix = env.init_config_ix_at(
        &authority,
        &config,
        &Address::new_from_array([6u8; 32]),
        &governance,
    );
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::InvalidProgramAccount,
    );

    // Every off-curve bump derives a valid PDA. Accepting a non-canonical one
    // would allow a second `protocol_config`.
    let (_, canonical) = Address::find_program_address(&[PROTOCOL_CONFIG_SEED], &env.program_id);
    let other = (0..canonical)
        .rev()
        .find_map(|b| {
            Address::create_program_address(&[PROTOCOL_CONFIG_SEED, &[b]], &env.program_id).ok()
        })
        .expect("some lower bump is off the curve");
    let ix = env.init_config_ix_at(&authority, &other, &program_data, &governance);
    assert_fails_with(env.send(&[ix], &[&deployer]), AxisCoreError::InvalidPda);

    let ix = env.init_config_ix_at(&authority, &config, &program_data, &Address::default());
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::InvalidAuthority,
    );
}

/// Anyone can send lamports to the config address before it exists. System
/// `CreateAccount` refuses a funded address, which would block the singleton
/// forever if the program relied on it alone.
#[test]
fn a_pre_funded_config_address_still_initializes() {
    let cheapest = Env::uninitialized()
        .vm
        .minimum_balance_for_rent_exemption(0);
    for lamports in [cheapest, 10_000_000_000] {
        let mut env = Env::uninitialized();
        let config = config_address(&env.program_id);
        env.vm.airdrop(&config, lamports).expect("pre-fund the PDA");
        let deployer = env.deployer.insecure_clone();
        let ix = env.init_config_ix(&address_of(&deployer));
        env.send(&[ix], &[&deployer])
            .unwrap_or_else(|e| panic!("init with {lamports} lamports pre-funded: {e:?}"));
        assert_eq!(
            env.vm.get_account(&config).unwrap().data.len(),
            ProtocolConfig::LEN
        );
    }
}

// ---------------------------------------------------------------------------
// CreateMarket: governance approval and custody validation
// ---------------------------------------------------------------------------

#[test]
fn governance_creates_a_market_with_validated_custody() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("created"));

    let account = env.vm.get_account(&market.market).unwrap();
    assert_eq!(account.owner, env.program_id);
    assert_eq!(account.data.len(), DTFMarket::LEN);
    assert!(account.lamports >= env.vm.minimum_balance_for_rent_exemption(DTFMarket::LEN));

    let state = market_state(&env, &market);
    assert_eq!(state.dtf_mint, market.dtf_mint);
    assert_eq!(state.treasury, address_of(&env.treasury));
    assert_eq!(state.usdc_mint, env.usdc_mint);
    assert_eq!(state.swap_program, legacy_token_program_id());
    assert_eq!(state.liquidity_lock, market.liquidity_lock);
    assert_eq!(state.reference_supply, 1_000);
    assert_eq!(state.min_liquidity, 1_000);
    assert_eq!(state.dtf_token_program, TokenProgramKind::Token2022);
    assert!(!state.purchase_paused && !state.usdc_redemption_paused);
    let live: Vec<_> = state.assets().collect();
    assert_eq!(live.len(), 2);
    for (i, asset) in live.iter().enumerate() {
        assert_eq!(asset.asset_mint, market.asset_mints[i]);
        assert_eq!(asset.reserve_vault, market.vaults[i]);
        assert_eq!(asset.reference_amount, market.reference_amounts[i]);
        assert_eq!(asset.token_program, TokenProgramKind::LegacySplToken);
    }
    state.validate_composition().expect("composition");

    // Approval funds nothing: the first public purchaser does.
    assert_eq!(env.supply(&market.dtf_mint), 0);
    assert!(market.vaults.iter().all(|v| env.balance(v) == 0));
}

#[test]
fn only_governance_can_create_a_market() {
    let mut env = Env::new();
    let market = env.prepare_market(&MarketSpec::two_assets("unauthorized"));
    let outsider = env.funded_keypair("outsider");
    for signer in [
        env.operator.insecure_clone(),
        env.deployer.insecure_clone(),
        env.treasury.insecure_clone(),
        outsider,
    ] {
        let ix = env.create_market_ix(&market, &address_of(&signer));
        assert_fails_with(
            env.send(&[ix], &[&signer]),
            AxisCoreError::UnauthorizedProtocolAuthority,
        );
    }
    assert!(env.vm.get_account(&market.market).is_none());

    // Governance must sign; naming it is not enough.
    let governance = address_of(&env.governance);
    let mut ix = env.create_market_ix(&market, &governance);
    ix.accounts[0] = AccountMeta::new(to_pubkey(&governance), false);
    let deployer = env.deployer.insecure_clone();
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::AccountNotSigner,
    );
}

#[test]
fn a_dtf_mint_gets_at_most_one_market() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("once"));
    let governance = env.governance.insecure_clone();
    let ix = env.create_market_ix(&market, &address_of(&governance));
    assert_fails_with(
        env.send(&[ix], &[&governance]),
        AxisCoreError::AccountAlreadyInitialized,
    );
}

#[test]
fn create_market_rejects_a_config_it_cannot_trust() {
    let mut env = Env::new();
    let market = env.prepare_market(&MarketSpec::two_assets("config"));
    let governance = env.governance.insecure_clone();
    let real = env
        .vm
        .get_account(&config_address(&env.program_id))
        .unwrap();

    // A config the caller controls would let them name themselves governance.
    let foreign = Address::new_from_array([7u8; 32]);
    env.vm
        .set_account(
            foreign,
            Account {
                owner: Address::new_from_array([66u8; 32]),
                ..real.clone()
            },
        )
        .unwrap();
    let ix = env.create_market_ix_with_config(&market, &address_of(&governance), &foreign);
    assert_fails_with(
        env.send(&[ix], &[&governance]),
        AxisCoreError::InvalidAccountOwner,
    );

    let elsewhere = Address::new_from_array([8u8; 32]);
    env.vm.set_account(elsewhere, real).unwrap();
    let ix = env.create_market_ix_with_config(&market, &address_of(&governance), &elsewhere);
    assert_fails_with(env.send(&[ix], &[&governance]), AxisCoreError::InvalidPda);
}

fn expect_create_market_error(
    label: &'static str,
    change: impl FnOnce(&mut Env, &mut MarketFixture),
    want: AxisCoreError,
) {
    let mut env = Env::new();
    let mut market = env.prepare_market(&MarketSpec::two_assets(label));
    change(&mut env, &mut market);
    let governance = env.governance.insecure_clone();
    let ix = env.create_market_ix(&market, &address_of(&governance));
    assert_fails_with(env.send(&[ix], &[&governance]), want);
    assert!(
        env.vm.get_account(&market.market).is_none(),
        "{label}: no market may exist after a rejected CreateMarket"
    );
}

/// DTF may exist only through Core's own issuance.
#[test]
fn create_market_validates_the_dtf_mint() {
    expect_create_market_error(
        "mint-authority",
        |env, m| tamper(env, &m.dtf_mint, |a| a.data[4..36].fill(9)),
        AxisCoreError::InvalidMintAuthority,
    );
    expect_create_market_error(
        "no-mint-authority",
        |env, m| tamper(env, &m.dtf_mint, |a| a.data[0..4].fill(0)),
        AxisCoreError::InvalidMintAuthority,
    );
    expect_create_market_error(
        "freeze-authority",
        |env, m| tamper(env, &m.dtf_mint, |a| a.data[46] = 1),
        AxisCoreError::MintHasFreezeAuthority,
    );
    expect_create_market_error(
        "pre-minted",
        |env, m| tamper(env, &m.dtf_mint, |a| a.data[36] = 1),
        AxisCoreError::MintSupplyNotZero,
    );
    expect_create_market_error(
        "extension",
        |env, m| {
            tamper(env, &m.dtf_mint, |a| {
                a.data.resize(170, 0);
                a.data[165] = 1;
            })
        },
        AxisCoreError::UnsupportedTokenExtension,
    );
    expect_create_market_error(
        "not-a-mint",
        |env, m| tamper(env, &m.dtf_mint, |a| a.owner = Address::default()),
        AxisCoreError::InvalidTokenProgram,
    );
    expect_create_market_error(
        "uninitialized",
        |env, m| tamper(env, &m.dtf_mint, |a| a.data[45] = 0),
        AxisCoreError::InvalidMint,
    );
}

/// In-kind redemption is only an exit if the market PDA alone controls every
/// reserve.
#[test]
fn create_market_validates_every_reserve_vault() {
    // A vault some wallet owns, created with real instructions.
    expect_create_market_error(
        "vault-owner",
        |env, m| {
            let owner = address_of(&env.governance);
            m.vaults[1] = env.create_token_account(
                TokenProgramKind::LegacySplToken,
                &m.asset_mints[1],
                &owner,
            );
        },
        AxisCoreError::TokenAccountOwnerMismatch,
    );
    // A vault for another mint.
    expect_create_market_error(
        "vault-mint",
        |env, m| {
            let usdc = env.usdc_mint;
            m.vaults[0] =
                env.create_token_account(TokenProgramKind::LegacySplToken, &usdc, &m.market);
        },
        AxisCoreError::TokenAccountMintMismatch,
    );
    expect_create_market_error(
        "vault-delegate",
        |env, m| tamper(env, &m.vaults[0], |a| a.data[72] = 1),
        AxisCoreError::TokenAccountHasDelegate,
    );
    expect_create_market_error(
        "vault-close-authority",
        |env, m| tamper(env, &m.vaults[0], |a| a.data[129] = 1),
        AxisCoreError::TokenAccountHasCloseAuthority,
    );
    expect_create_market_error(
        "vault-frozen",
        |env, m| tamper(env, &m.vaults[0], |a| a.data[108] = 2),
        AxisCoreError::InvalidTokenAccount,
    );
    expect_create_market_error(
        "vault-wrong-program",
        |env, m| tamper(env, &m.vaults[0], |a| a.owner = token_2022_program_id()),
        AxisCoreError::InvalidTokenAccount,
    );
    expect_create_market_error(
        "asset-mint-extension",
        |env, m| tamper(env, &m.asset_mints[0], |a| a.data.resize(170, 0)),
        AxisCoreError::UnsupportedTokenExtension,
    );
    expect_create_market_error(
        "lock-owner",
        |env, m| {
            let owner = address_of(&env.governance);
            m.liquidity_lock = env.create_token_account(m.dtf_kind, &m.dtf_mint, &owner);
        },
        AxisCoreError::TokenAccountOwnerMismatch,
    );
}

#[test]
fn create_market_validates_the_composition() {
    expect_create_market_error(
        "duplicate",
        |_, m| {
            m.asset_mints[1] = m.asset_mints[0];
            m.vaults[1] = m.vaults[0];
        },
        AxisCoreError::DuplicateAsset,
    );
    expect_create_market_error(
        "usdc-constituent",
        |env, m| {
            let usdc = env.usdc_mint;
            m.asset_mints[1] = usdc;
            m.vaults[1] =
                env.create_token_account(TokenProgramKind::LegacySplToken, &usdc, &m.market);
        },
        AxisCoreError::ForbiddenAsset,
    );
    expect_create_market_error(
        "one-asset",
        |_, m| {
            m.asset_mints.truncate(1);
            m.vaults.truncate(1);
            m.reference_amounts.truncate(1);
        },
        AxisCoreError::TooFewAssets,
    );
    expect_create_market_error(
        "zero-reference",
        |_, m| m.reference_amounts[1] = 0,
        AxisCoreError::InvalidReferenceBasket,
    );
    expect_create_market_error(
        "zero-lock",
        |_, m| m.min_liquidity = 0,
        AxisCoreError::InvalidReferenceBasket,
    );
    // Axis Core as its own route would let a leg re-enter Purchase.
    expect_create_market_error(
        "self-route",
        |env, m| m.swap_program = env.program_id,
        AxisCoreError::InvalidProgramAccount,
    );

    let mut env = Env::new();
    let mut spec = MarketSpec::two_assets("four");
    spec.reference_amounts = vec![1_000; 4];
    let market = env.prepare_market(&spec);
    let governance = env.governance.insecure_clone();
    let ix = env.create_market_ix(&market, &address_of(&governance));
    assert_fails_with(
        env.send(&[ix], &[&governance]),
        AxisCoreError::TooManyAssets,
    );
}

// ---------------------------------------------------------------------------
// First purchase
// ---------------------------------------------------------------------------

#[test]
fn the_first_public_purchase_funds_the_approved_basket() {
    let (env, market, alice, maker) = funded_market();

    let b = env.balances(&market, &[&alice, &maker]);
    assert_eq!(b.supply, 100_000);
    assert_eq!(b.reserves, vec![200_000, 500_000]);
    assert_eq!(b.lock, 1_000, "minimum liquidity locked for good");
    assert_eq!(b.treasury_dtf, 300, "30 bps of the gross issuance");
    assert_eq!(b.traders[0].dtf, 98_700);
    assert_eq!(b.lock + b.treasury_dtf + b.traders[0].dtf, b.supply);
    assert_eq!(
        b.traders[0].usdc,
        ALICE_USDC - FIRST_PURCHASE_USDC,
        "unspent USDC never left the buyer"
    );
}

/// The first purchaser chooses the size. The allocation and the issuance rate
/// are governance's: delivering extra of one constituent buys nothing.
#[test]
fn the_first_purchaser_cannot_change_the_allocation_or_the_issuance() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("skewed"));
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);

    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        900_000,
        &[400_000, 500_000],
        900_000,
        0,
    )
    .expect("skewed first purchase");

    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.supply, 100_000, "B still limits issuance to 100,000");
    assert_eq!(b.traders[0].dtf, 98_700);
    assert_eq!(
        b.reserves,
        vec![400_000, 500_000],
        "the surplus A stays in the reserve"
    );
}

#[test]
fn a_first_purchase_must_issue_more_than_the_locked_minimum() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("tiny"));
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    let before = env.balances(&market, &[&alice, &maker]);

    // Gross 1,003 = lock 1,000 + fee 3, which leaves the buyer nothing.
    for deliveries in [[2_000, 5_000], [2_006, 5_015]] {
        assert_fails_with(
            purchase(
                &mut env,
                &market,
                true,
                &alice,
                &maker,
                10,
                &deliveries,
                10,
                0,
            ),
            AxisCoreError::SupplyBelowMinimumLiquidity,
        );
        assert_eq!(env.balances(&market, &[&alice, &maker]), before);
    }
    assert_fails_with(
        purchase(&mut env, &market, true, &alice, &maker, 10, &[1, 1], 10, 0),
        AxisCoreError::ZeroAmount,
    );

    // One unit more and the buyer holds exactly 1.
    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        10,
        &[2_008, 5_020],
        10,
        1,
    )
    .expect("smallest first purchase");
    let b = env.balances(&market, &[&alice]);
    assert_eq!((b.supply, b.lock, b.treasury_dtf), (1_004, 1_000, 3));
    assert_eq!(b.traders[0].dtf, 1);
}

#[test]
fn an_unfunded_market_only_accepts_the_initial_purchase() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("unfunded"));
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    let before = env.balances(&market, &[&alice, &maker]);

    assert_fails_with(
        purchase(
            &mut env,
            &market,
            false,
            &alice,
            &maker,
            700_000,
            &[200_000, 500_000],
            700_000,
            0,
        ),
        AxisCoreError::MarketNotSeeded,
    );
    let ix = env.redeem_in_kind_ix(&market, &alice, 1);
    assert_fails_with(
        env.send(&[ix], &[&alice.keypair]),
        AxisCoreError::MarketNotSeeded,
    );
    assert_eq!(env.balances(&market, &[&alice, &maker]), before);

    // The lock account in the instruction must be the market's own.
    let legs = purchase_legs(&market, &alice, &maker, 700_000, &[200_000, 500_000]);
    let mut ix = env.purchase_ix(&market, true, &alice, 700_000, 0, &legs);
    ix.accounts[8] = AccountMeta::new(to_pubkey(&alice.dtf), false);
    assert_fails_with(
        env.send(&[ix], &[&alice.keypair, &maker.keypair]),
        AxisCoreError::AccountMismatch,
    );
}

/// Tokens sent straight to a vault before launch are a gift to the reserves.
/// They do not count as the first purchaser's delivery.
#[test]
fn a_direct_transfer_before_launch_does_not_change_the_first_issuance() {
    let mut env = Env::new();
    let market = env.create_market(&MarketSpec::two_assets("donated"));
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    env.transfer_tokens(
        TokenProgramKind::LegacySplToken,
        &maker.assets[0],
        &market.vaults[0],
        &maker.keypair,
        77_777,
    )
    .expect("direct transfer");

    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        700_000,
        &[200_000, 500_000],
        700_000,
        98_700,
    )
    .expect("first purchase");
    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.supply, 100_000);
    assert_eq!(b.traders[0].dtf, 98_700);
    assert_eq!(b.reserves, vec![277_777, 500_000]);
}

/// Two buyers race for the first purchase. The loser's `InitialPurchase` runs
/// as an ordinary purchase against the winner's reserves, under the loser's
/// own signed bounds, and pays no second lock.
#[test]
fn the_loser_of_a_first_purchase_race_buys_as_an_ordinary_purchaser() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, ALICE_USDC);

    purchase(
        &mut env,
        &market,
        true,
        &bob,
        &maker,
        700_000,
        &[200_000, 500_000],
        700_000,
        99_700,
    )
    .expect("the second InitialPurchase");

    let b = env.balances(&market, &[&alice, &bob]);
    assert_eq!(b.supply, 200_000);
    assert_eq!(b.lock, 1_000, "locked once, by the first purchase only");
    assert_eq!(b.treasury_dtf, 600);
    assert_eq!(b.traders[1].dtf, 99_700);
}

// ---------------------------------------------------------------------------
// Additional purchase: the acceptance fixture
// ---------------------------------------------------------------------------

const BOB_USDC: u64 = 10_000;
const BOB_SPEND: u64 = 7_000;

fn acceptance_purchase(
    env: &mut Env,
    market: &MarketFixture,
    bob: &Trader,
    maker: &Trader,
    min_net_dtf_out: u64,
) -> TxResult {
    purchase(
        env,
        market,
        false,
        bob,
        maker,
        BOB_SPEND,
        &[2_100, 5_000],
        BOB_SPEND,
        min_net_dtf_out,
    )
}

/// Supply 100,000 with reserves A = 200,000 and B = 500,000. Actual reserve
/// increases of A = 2,100 and B = 5,000 issue 1,000 gross, limited by B: 3 to
/// the treasury and 997 to the buyer.
#[test]
fn an_additional_purchase_issues_against_actual_reserve_increases() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);
    assert_eq!(before.supply, 100_000);
    assert_eq!(before.reserves, vec![200_000, 500_000]);

    let meta = acceptance_purchase(&mut env, &market, &bob, &maker, 997).expect("purchase");
    println!(
        "additional purchase, 2 assets, route stand-in: {} CU",
        meta.compute_units_consumed
    );

    let after = env.balances(&market, &[&alice, &bob, &maker]);
    assert_eq!(after.supply, 101_000, "gross issuance of 1,000");
    assert_eq!(after.treasury_dtf - before.treasury_dtf, 3);
    assert_eq!(after.traders[1].dtf, 997);
    assert_eq!(
        after.reserves,
        vec![202_100, 505_000],
        "the 100 A beyond what B supports stays in the reserve"
    );
    assert_eq!(after.traders[1].usdc, BOB_USDC - BOB_SPEND);
    assert_eq!(after.lock, before.lock);
    assert_eq!(
        after.traders[0], before.traders[0],
        "other holders untouched"
    );
}

#[test]
fn a_purchase_below_the_signed_minimum_net_output_fails_atomically() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);

    assert_fails_with(
        acceptance_purchase(&mut env, &market, &bob, &maker, 998),
        AxisCoreError::SlippageExceeded,
    );

    // The swaps had already delivered to the reserves when the bound failed.
    assert_eq!(
        env.balances(&market, &[&alice, &bob, &maker]),
        before,
        "no token balance and no DTF supply may change"
    );
}

#[test]
fn a_purchase_above_the_signed_maximum_usdc_fails_atomically() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);

    assert_fails_with(
        purchase(
            &mut env,
            &market,
            false,
            &bob,
            &maker,
            BOB_SPEND + 1,
            &[2_100, 5_000],
            BOB_SPEND,
            0,
        ),
        AxisCoreError::MaxInputExceeded,
    );
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);

    // Spending exactly the maximum is allowed.
    purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[2_100, 5_000],
        BOB_SPEND,
        0,
    )
    .expect("at the maximum");
}

#[test]
fn a_failed_route_leg_reverts_the_legs_before_it() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);

    // The USDC payment and the A delivery succeed; the B delivery asks for
    // more than the maker holds.
    let result = purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[2_100, u64::MAX],
        BOB_SPEND,
        0,
    );
    assert!(result.is_err(), "the route must fail");
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);
}

/// Core never signs a route, so a route cannot move a reserve. A leg that
/// tries is refused by the token program and the purchase reverts.
#[test]
fn a_route_cannot_debit_a_reserve_vault() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);

    for thief in [bob.address(), market.market] {
        let mut legs = purchase_legs(&market, &bob, &maker, BOB_SPEND, &[2_100, 5_000]);
        let mut steal = transfer_leg(&market.vaults[0], &bob.assets[0], &thief, 1_000);
        // The market PDA cannot sign a transaction.
        steal.accounts[2].is_signer = thief == bob.address();
        legs.push(steal);
        let ix = env.purchase_ix(&market, false, &bob, BOB_SPEND, 0, &legs);
        let result = env.send(&[ix], &[&bob.keypair, &maker.keypair]);
        assert!(result.is_err(), "a reserve debit must fail");
        assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);
    }
}

#[test]
fn a_purchase_that_delivers_too_little_to_issue_anything_fails() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let before = env.balances(&market, &[&alice, &bob, &maker]);

    // 4 B supports floor(4 * 100,000 / 500,000) = 0 DTF.
    assert_fails_with(
        purchase(
            &mut env,
            &market,
            false,
            &bob,
            &maker,
            10,
            &[2_100, 4],
            10,
            0,
        ),
        AxisCoreError::ZeroAmount,
    );
    // No delivery at all to one constituent.
    assert_fails_with(
        purchase(
            &mut env,
            &market,
            false,
            &bob,
            &maker,
            10,
            &[2_100, 0],
            10,
            0,
        ),
        AxisCoreError::ZeroAmount,
    );
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);
}

#[test]
fn a_purchase_rejects_accounts_that_are_not_the_markets() {
    let (mut env, market, _alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let mallory = buyer(&mut env, "mallory", &market, BOB_USDC);
    let legs = purchase_legs(&market, &bob, &maker, BOB_SPEND, &[2_100, 5_000]);
    let base = env.purchase_ix(&market, false, &bob, BOB_SPEND, 0, &legs);
    let signers: [&Keypair; 2] = [&bob.keypair, &maker.keypair];

    // A vault that is not the market's, in a reserve slot.
    let mut ix = base.clone();
    ix.accounts[8] = AccountMeta::new(to_pubkey(&bob.assets[0]), false);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    // Reserve vaults out of table order.
    let mut ix = base.clone();
    ix.accounts.swap(8, 9);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    // A fee destination the treasury does not own.
    let mut ix = base.clone();
    ix.accounts[5] = AccountMeta::new(to_pubkey(&mallory.dtf), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::TokenAccountOwnerMismatch,
    );

    // Someone else's USDC account as the one the maximum is measured on.
    let mut ix = base.clone();
    ix.accounts[3] = AccountMeta::new(to_pubkey(&mallory.usdc), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::TokenAccountOwnerMismatch,
    );

    // A swap program governance did not approve.
    let mut ix = base.clone();
    ix.accounts[7] = AccountMeta::new_readonly(to_pubkey(&token_2022_program_id()), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::InvalidProgramAccount,
    );

    // Another DTF mint.
    let mut ix = base.clone();
    ix.accounts[2] = AccountMeta::new(to_pubkey(&market.asset_mints[0]), false);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    // A market account Axis Core does not own.
    let mut ix = base.clone();
    ix.accounts[1] = AccountMeta::new_readonly(to_pubkey(&bob.usdc), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::InvalidAccountOwner,
    );

    // The buyer named but not signing.
    let mut ix = base.clone();
    ix.accounts[0] = AccountMeta::new_readonly(to_pubkey(&bob.address()), false);
    ix.accounts.truncate(10);
    ix.data.truncate(17);
    ix.data.push(0);
    let deployer = env.deployer.insecure_clone();
    assert_fails_with(
        env.send(&[ix], &[&deployer]),
        AxisCoreError::AccountNotSigner,
    );

    env.send(&[base], &signers)
        .expect("the untouched instruction");
}

#[test]
fn route_encoding_is_checked_strictly() {
    let (mut env, market, _alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let legs = purchase_legs(&market, &bob, &maker, BOB_SPEND, &[2_100, 5_000]);
    let base = env.purchase_ix(&market, false, &bob, BOB_SPEND, 0, &legs);
    let signers: [&Keypair; 2] = [&bob.keypair, &maker.keypair];

    // A trailing account no leg consumes.
    let mut ix = base.clone();
    ix.accounts
        .push(AccountMeta::new_readonly(to_pubkey(&bob.usdc), false));
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::InvalidRouteData);

    // Trailing bytes after the last leg.
    let mut ix = base.clone();
    ix.data.push(0);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::InvalidRouteData);

    // A leg that claims more accounts than the instruction carries.
    let mut ix = base.clone();
    let last_vault = ix.accounts.len() - 2;
    ix.accounts.remove(last_vault);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::MissingAccount);

    // No route at all.
    let mut ix = base.clone();
    ix.data.truncate(17);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::InvalidRouteData);

    // More legs than the cap.
    let many: Vec<Leg> = (0..7)
        .map(|_| transfer_leg(&bob.usdc, &maker.usdc, &bob.address(), 1))
        .collect();
    let ix = env.purchase_ix(&market, false, &bob, BOB_SPEND, 0, &many);
    assert_fails_with(
        env.send(&[ix], &[&bob.keypair]),
        AxisCoreError::TooManyRouteLegs,
    );
}

/// A leg may carry up to `MAX_ROUTE_LEG_ACCOUNTS` accounts, the bound of the
/// stack frame that holds the CPI account array.
#[test]
fn a_route_leg_at_the_account_bound_executes_and_one_past_it_is_refused() {
    let (mut env, market, _alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let padded = |extra: usize| {
        let mut legs = purchase_legs(&market, &bob, &maker, BOB_SPEND, &[2_100, 5_000]);
        // The token program ignores accounts after a single-signer authority.
        let filler = AccountMeta::new_readonly(to_pubkey(&legacy_token_program_id()), false);
        legs[1]
            .accounts
            .extend(std::iter::repeat_n(filler, extra - 3));
        legs
    };

    let ix = env.purchase_ix(
        &market,
        false,
        &bob,
        BOB_SPEND,
        0,
        &padded(MAX_ROUTE_LEG_ACCOUNTS + 1),
    );
    assert_fails_with(
        env.send(&[ix], &[&bob.keypair, &maker.keypair]),
        AxisCoreError::TooManyRouteAccounts,
    );

    let ix = env.purchase_ix(
        &market,
        false,
        &bob,
        BOB_SPEND,
        997,
        &padded(MAX_ROUTE_LEG_ACCOUNTS),
    );
    env.send(&[ix], &[&bob.keypair, &maker.keypair])
        .expect("a leg at the bound");
    assert_eq!(env.balance(&bob.dtf), 997);
}

// ---------------------------------------------------------------------------
// In-kind redemption
// ---------------------------------------------------------------------------

#[test]
fn a_partial_in_kind_redemption_transfers_the_proportional_quantities() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);

    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    let meta = env.send(&[ix], &[&alice.keypair]).expect("redeem in kind");
    println!(
        "in-kind redemption, 2 assets: {} CU",
        meta.compute_units_consumed
    );

    // Fee 30 to the treasury; 9,970 burned; 9,970 / 100,000 of each reserve.
    let after = env.balances(&market, &[&alice, &maker]);
    assert_eq!(after.treasury_dtf - before.treasury_dtf, 30);
    assert_eq!(after.supply, 100_000 - 9_970);
    assert_eq!(after.traders[0].dtf, 98_700 - 10_000);
    assert_eq!(after.traders[0].assets, vec![19_940, 49_850]);
    assert_eq!(after.reserves, vec![200_000 - 19_940, 500_000 - 49_850]);
    assert_eq!(after.traders[0].usdc, before.traders[0].usdc);
    assert_eq!(after.lock, before.lock);
    assert_eq!(after.traders[1], before.traders[1]);
}

/// Every holder leaves, the treasury included. The locked minimum keeps
/// backing behind it, so the ratio stays defined and the market can be bought
/// again.
#[test]
fn full_in_kind_exits_leave_the_locked_minimum_backed() {
    let (mut env, market, alice, maker) = funded_market();

    let ix = env.redeem_in_kind_ix(&market, &alice, 98_700);
    env.send(&[ix], &[&alice.keypair]).expect("full exit");
    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.traders[0].dtf, 0);
    // Fee 296; 98,404 burned.
    assert_eq!(b.traders[0].assets, vec![196_808, 492_020]);
    assert_eq!(b.treasury_dtf, 596);
    assert_eq!(b.supply, 1_596);
    assert_eq!(b.reserves, vec![3_192, 7_980]);

    // The treasury redeems its fees in kind, paying the fee to itself.
    let treasury = Trader {
        keypair: env.treasury.insecure_clone(),
        usdc: alice.usdc,
        dtf: market.treasury_dtf,
        assets: {
            let owner = address_of(&env.treasury);
            market
                .asset_mints
                .iter()
                .map(|m| env.create_token_account(TokenProgramKind::LegacySplToken, m, &owner))
                .collect()
        },
    };
    let ix = env.redeem_in_kind_ix(&market, &treasury, 596);
    env.send(&[ix], &[&treasury.keypair])
        .expect("treasury exit");
    let b = env.balances(&market, &[&treasury]);
    assert_eq!(b.traders[0].assets, vec![1_190, 2_975]);
    assert_eq!((b.supply, b.lock, b.treasury_dtf), (1_001, 1_000, 1));
    assert_eq!(b.reserves, vec![2_002, 5_005]);

    // The lock cannot be redeemed by anyone: its owner is the market PDA,
    // which signs no DTF transfer or burn.
    let mut thief = Trader {
        keypair: alice.keypair.insecure_clone(),
        usdc: alice.usdc,
        dtf: market.liquidity_lock,
        assets: alice.assets.clone(),
    };
    let ix = env.redeem_in_kind_ix(&market, &thief, 1_000);
    assert!(env.send(&[ix], &[&thief.keypair]).is_err());
    thief.dtf = alice.dtf;

    // A new buyer follows the reserves that remain.
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[2_002, 5_005],
        BOB_SPEND,
        998,
    )
    .expect("purchase after every holder left");
    let b = env.balances(&market, &[&bob]);
    assert_eq!(b.supply, 2_002);
    assert_eq!(b.traders[0].dtf, 998);
}

#[test]
fn in_kind_redemption_validates_its_accounts_and_amount() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);
    let base = env.redeem_in_kind_ix(&market, &alice, 10_000);
    let signers: [&Keypair; 1] = [&alice.keypair];

    let ix = env.redeem_in_kind_ix(&market, &alice, 0);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::ZeroAmount);

    // More DTF than the holder has: the token program refuses the burn.
    let ix = env.redeem_in_kind_ix(&market, &alice, 98_701);
    assert!(env.send(&[ix], &signers).is_err());

    // A reserve vault as the destination.
    let mut ix = base.clone();
    ix.accounts[9] = ix.accounts[8].clone();
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    // A vault that is not the market's.
    let mut ix = base.clone();
    ix.accounts[7] = AccountMeta::new(to_pubkey(&maker.assets[0]), false);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    // A fee destination the treasury does not own.
    let mut ix = base.clone();
    ix.accounts[4] = AccountMeta::new(to_pubkey(&maker.dtf), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::TokenAccountOwnerMismatch,
    );

    // The wrong program in a token program slot the exit needs.
    let mut ix = base.clone();
    ix.accounts[6] = ix.accounts[5].clone();
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::InvalidTokenProgram,
    );

    // A missing destination.
    let mut ix = base.clone();
    ix.accounts.pop();
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::MissingAccount);

    // Someone else's DTF: the holder named must be the one who signs.
    let mallory = env.trader("mallory", &market);
    let mut ix = base.clone();
    ix.accounts[0] = AccountMeta::new_readonly(to_pubkey(&mallory.address()), true);
    assert!(env.send(&[ix], &[&mallory.keypair]).is_err());

    assert_eq!(env.balances(&market, &[&alice, &maker]), before);
}

/// A redemption too small to release one base unit of any constituent would
/// only destroy the holder's DTF, so it is refused.
#[test]
fn a_dust_redemption_that_would_release_nothing_is_refused() {
    let mut env = Env::new();
    let mut spec = MarketSpec::two_assets("dust");
    spec.reference_supply = 1_000_000;
    spec.reference_amounts = vec![1_000, 1_000];
    let market = env.create_market(&spec);
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        10,
        &[100_000, 100_000],
        10,
        0,
    )
    .expect("first purchase");
    assert_eq!(env.supply(&market.dtf_mint), 100_000_000);
    let before = env.balances(&market, &[&alice]);

    let ix = env.redeem_in_kind_ix(&market, &alice, 999);
    assert_fails_with(
        env.send(&[ix], &[&alice.keypair]),
        AxisCoreError::ZeroAmount,
    );
    assert_eq!(env.balances(&market, &[&alice]), before);

    // 1,003 DTF: fee 3, 1,000 burned, one base unit of each constituent.
    let ix = env.redeem_in_kind_ix(&market, &alice, 1_003);
    env.send(&[ix], &[&alice.keypair]).expect("smallest exit");
    assert_eq!(
        env.balances(&market, &[&alice]).traders[0].assets,
        vec![1, 1]
    );
}

/// A market whose DTF and constituents all use the legacy token program can
/// repeat it in both token program slots.
#[test]
fn an_all_legacy_market_needs_only_one_token_program() {
    let mut env = Env::new();
    let mut spec = MarketSpec::two_assets("legacy");
    spec.dtf_kind = TokenProgramKind::LegacySplToken;
    let mut market = env.create_market(&spec);
    assert_eq!(
        market_state(&env, &market).dtf_token_program,
        TokenProgramKind::LegacySplToken
    );
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        700_000,
        &[200_000, 500_000],
        700_000,
        98_700,
    )
    .expect("first purchase");

    market.token_program_slots = [legacy_token_program_id(); 2];
    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    env.send(&[ix], &[&alice.keypair]).expect("redeem in kind");
    assert_eq!(
        env.balances(&market, &[&alice]).traders[0].assets,
        vec![19_940, 49_850]
    );
}

// ---------------------------------------------------------------------------
// USDC redemption
// ---------------------------------------------------------------------------

fn redeem_usdc(
    env: &mut Env,
    market: &MarketFixture,
    holder: &Trader,
    maker: &Trader,
    dtf_in: u64,
    sold: &[u64],
    usdc: u64,
    min_net_usdc_out: u64,
) -> TxResult {
    let legs = redemption_legs(holder, maker, sold, usdc);
    let ix = env.redeem_usdc_ix(market, holder, dtf_in, min_net_usdc_out, &legs);
    env.send(&[ix], &[&holder.keypair, &maker.keypair])
}

#[test]
fn a_partial_usdc_redemption_pays_the_fee_in_dtf_and_delivers_net_usdc() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);

    let meta = redeem_usdc(
        &mut env,
        &market,
        &alice,
        &maker,
        10_000,
        &[19_940, 49_850],
        69_000,
        69_000,
    )
    .expect("redeem to USDC");
    println!(
        "USDC redemption, 2 assets, route stand-in: {} CU",
        meta.compute_units_consumed
    );

    let after = env.balances(&market, &[&alice, &maker]);
    assert_eq!(after.treasury_dtf - before.treasury_dtf, 30);
    assert_eq!(
        after.supply,
        100_000 - 9_970,
        "only the remainder is burned"
    );
    assert_eq!(after.traders[0].dtf, 98_700 - 10_000);
    assert_eq!(after.reserves, vec![200_000 - 19_940, 500_000 - 49_850]);
    assert_eq!(after.traders[0].usdc - before.traders[0].usdc, 69_000);
    assert_eq!(
        after.traders[0].assets,
        vec![0, 0],
        "everything released was sold"
    );
    assert_eq!(after.lock, before.lock);
}

#[test]
fn a_usdc_redemption_below_the_signed_minimum_net_usdc_fails_atomically() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);

    assert_fails_with(
        redeem_usdc(
            &mut env,
            &market,
            &alice,
            &maker,
            10_000,
            &[19_940, 49_850],
            69_000,
            69_001,
        ),
        AxisCoreError::SlippageExceeded,
    );

    // The fee, the burn, both releases and all three swaps had run.
    assert_eq!(
        env.balances(&market, &[&alice, &maker]),
        before,
        "no token balance and no DTF supply may change"
    );
}

#[test]
fn a_failed_redemption_swap_reverts_the_burn_the_fee_and_the_releases() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);

    // The route tries to sell more B than the redemption released.
    let result = redeem_usdc(
        &mut env,
        &market,
        &alice,
        &maker,
        10_000,
        &[19_940, 49_851],
        69_000,
        0,
    );
    assert!(result.is_err(), "the route must fail");
    assert_eq!(env.balances(&market, &[&alice, &maker]), before);

    // The maker cannot pay.
    let result = redeem_usdc(
        &mut env,
        &market,
        &alice,
        &maker,
        10_000,
        &[19_940, 49_850],
        u64::MAX,
        0,
    );
    assert!(result.is_err(), "the route must fail");
    assert_eq!(env.balances(&market, &[&alice, &maker]), before);
}

#[test]
fn a_full_usdc_redemption_exits_the_whole_position() {
    let (mut env, market, alice, maker) = funded_market();
    let usdc_before = env.balance(&alice.usdc);

    redeem_usdc(
        &mut env,
        &market,
        &alice,
        &maker,
        98_700,
        &[196_808, 492_020],
        680_000,
        680_000,
    )
    .expect("full exit to USDC");

    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.traders[0].dtf, 0);
    assert_eq!(b.traders[0].assets, vec![0, 0]);
    assert_eq!(b.traders[0].usdc - usdc_before, 680_000);
    assert_eq!(b.treasury_dtf, 596);
    assert_eq!(b.supply, 1_596);
    assert_eq!(b.reserves, vec![3_192, 7_980]);
}

/// The redemption route runs under the holder's signature only. It cannot
/// take more from a reserve than the redemption released.
#[test]
fn a_redemption_route_cannot_reach_the_remaining_reserves() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);

    let mut legs = redemption_legs(&alice, &maker, &[19_940, 49_850], 69_000);
    legs.push(transfer_leg(
        &market.vaults[0],
        &alice.assets[0],
        &alice.address(),
        1,
    ));
    let ix = env.redeem_usdc_ix(&market, &alice, 10_000, 0, &legs);
    let result = env.send(&[ix], &[&alice.keypair, &maker.keypair]);
    assert!(result.is_err(), "a reserve debit must fail");
    assert_eq!(env.balances(&market, &[&alice, &maker]), before);
}

#[test]
fn a_usdc_redemption_rejects_accounts_that_are_not_the_markets() {
    let (mut env, market, alice, maker) = funded_market();
    let before = env.balances(&market, &[&alice, &maker]);
    let legs = redemption_legs(&alice, &maker, &[19_940, 49_850], 69_000);
    let base = env.redeem_usdc_ix(&market, &alice, 10_000, 0, &legs);
    let signers: [&Keypair; 2] = [&alice.keypair, &maker.keypair];

    // The minimum must be measured on the holder's own USDC account.
    let mut ix = base.clone();
    ix.accounts[5] = AccountMeta::new(to_pubkey(&maker.usdc), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::TokenAccountOwnerMismatch,
    );

    // An account that is not USDC at all.
    let mut ix = base.clone();
    ix.accounts[5] = AccountMeta::new(to_pubkey(&alice.assets[0]), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::TokenAccountMintMismatch,
    );

    let mut ix = base.clone();
    ix.accounts[8] = AccountMeta::new_readonly(to_pubkey(&token_2022_program_id()), false);
    assert_fails_with(
        env.send(&[ix], &signers),
        AxisCoreError::InvalidProgramAccount,
    );

    let mut ix = base.clone();
    ix.accounts[9] = AccountMeta::new(to_pubkey(&maker.assets[0]), false);
    assert_fails_with(env.send(&[ix], &signers), AxisCoreError::AccountMismatch);

    assert_eq!(env.balances(&market, &[&alice, &maker]), before);
}

// ---------------------------------------------------------------------------
// Three constituents
// ---------------------------------------------------------------------------

#[test]
fn a_three_asset_market_purchases_and_redeems_on_both_paths() {
    let mut env = Env::new();
    let mut spec = MarketSpec::two_assets("three");
    spec.reference_amounts = vec![2_000, 5_000, 300];
    let market = env.create_market(&spec);
    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);

    purchase(
        &mut env,
        &market,
        true,
        &alice,
        &maker,
        700_000,
        &[200_000, 500_000, 30_000],
        700_000,
        98_700,
    )
    .expect("first purchase");

    // C limits: floor(290 * 100,000 / 30,000) = 966.
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let meta = purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[2_100, 5_000, 290],
        BOB_SPEND,
        964,
    )
    .expect("additional purchase");
    println!(
        "additional purchase, 3 assets, route stand-in: {} CU",
        meta.compute_units_consumed
    );
    let b = env.balances(&market, &[&alice, &bob]);
    assert_eq!(b.supply, 100_966);
    assert_eq!(b.treasury_dtf, 300 + 2);
    assert_eq!(b.traders[1].dtf, 964);
    assert_eq!(b.reserves, vec![202_100, 505_000, 30_290]);

    // Bob exits in kind: all 964, fee 2, 962 burned.
    let ix = env.redeem_in_kind_ix(&market, &bob, 964);
    env.send(&[ix], &[&bob.keypair]).expect("in kind");
    let release = |reserve: u64| (reserve as u128 * 962 / 100_966) as u64;
    let b = env.balances(&market, &[&bob]);
    assert_eq!(
        b.traders[0].assets,
        vec![release(202_100), release(505_000), release(30_290)]
    );
    assert_eq!(b.supply, 100_966 - 962);

    // Alice exits part of her position to USDC.
    let supply = b.supply;
    let reserves = b.reserves.clone();
    let sold: Vec<u64> = reserves
        .iter()
        .map(|r| (*r as u128 * 9_970 / supply as u128) as u64)
        .collect();
    let usdc_before = env.balance(&alice.usdc);
    let meta = redeem_usdc(
        &mut env, &market, &alice, &maker, 10_000, &sold, 70_000, 70_000,
    )
    .expect("to USDC");
    println!(
        "USDC redemption, 3 assets, route stand-in: {} CU",
        meta.compute_units_consumed
    );
    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.traders[0].usdc - usdc_before, 70_000);
    assert_eq!(b.traders[0].assets, vec![0, 0, 0]);
    for i in 0..3 {
        assert_eq!(b.reserves[i], reserves[i] - sold[i]);
    }
}

// ---------------------------------------------------------------------------
// Token-2022 constituents and direct transfers
// ---------------------------------------------------------------------------

/// Constituents under Token-2022, in reserve vaults that carry the
/// `ImmutableOwner` extension the associated token program adds. The route
/// stand-in here is the Token-2022 program, so legs can only deliver
/// constituents; the buyer's USDC does not move.
#[test]
fn token_2022_constituents_are_purchased_and_redeemed_in_kind() {
    let mut env = Env::new();
    let mut spec = MarketSpec::two_assets("token-2022-assets");
    spec.asset_kind = TokenProgramKind::Token2022;
    spec.immutable_owner_vaults = true;
    spec.swap_program = token_2022_program_id();
    let market = env.create_market(&spec);
    for asset in market_state(&env, &market).assets() {
        assert_eq!(asset.token_program, TokenProgramKind::Token2022);
    }
    assert_eq!(
        env.vm.get_account(&market.vaults[0]).unwrap().data.len(),
        170
    );

    let maker = maker(&mut env, &market);
    let alice = buyer(&mut env, "alice", &market, ALICE_USDC);
    let deliver = |deliveries: [u64; 2]| -> Vec<Leg> {
        (0..2)
            .map(|i| {
                transfer_leg(
                    &maker.assets[i],
                    &market.vaults[i],
                    &maker.address(),
                    deliveries[i],
                )
            })
            .collect()
    };

    let ix = env.purchase_ix(
        &market,
        true,
        &alice,
        0,
        98_700,
        &deliver([200_000, 500_000]),
    );
    env.send(&[ix], &[&alice.keypair, &maker.keypair])
        .expect("first purchase");
    let ix = env.purchase_ix(&market, false, &alice, 0, 997, &deliver([2_100, 5_000]));
    env.send(&[ix], &[&alice.keypair, &maker.keypair])
        .expect("additional purchase");
    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.supply, 101_000);
    assert_eq!(b.reserves, vec![202_100, 505_000]);
    assert_eq!(b.traders[0].dtf, 98_700 + 997);
    assert_eq!(b.traders[0].usdc, ALICE_USDC, "max_usdc_in of zero held");

    // Fee 30; 9,970 of 101,000 of each reserve.
    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    env.send(&[ix], &[&alice.keypair]).expect("redeem in kind");
    let after = env.balances(&market, &[&alice]);
    assert_eq!(after.traders[0].assets, vec![19_949, 49_850]);
    assert_eq!(after.treasury_dtf - b.treasury_dtf, 30);
    assert_eq!(after.supply, 101_000 - 9_970);
}

/// A Token-2022 custody account may carry `ImmutableOwner` and nothing else.
#[test]
fn a_token_2022_vault_with_any_other_extension_is_refused() {
    let token_2022 = |m: &mut MarketSpec| {
        m.asset_kind = TokenProgramKind::Token2022;
        m.immutable_owner_vaults = true;
        m.swap_program = token_2022_program_id();
    };
    for (label, change) in [
        // Another extension type in place of ImmutableOwner.
        (
            "other-extension",
            Box::new(|a: &mut Account| a.data[166] = 9) as Box<dyn Fn(&mut Account)>,
        ),
        // A second extension after it.
        (
            "second-extension",
            Box::new(|a: &mut Account| a.data.resize(178, 0)),
        ),
    ] {
        let mut env = Env::new();
        let mut spec = MarketSpec::two_assets(label);
        token_2022(&mut spec);
        let market = env.prepare_market(&spec);
        tamper(&mut env, &market.vaults[0], |a| change(a));
        let governance = env.governance.insecure_clone();
        let ix = env.create_market_ix(&market, &address_of(&governance));
        assert_fails_with(
            env.send(&[ix], &[&governance]),
            AxisCoreError::UnsupportedTokenExtension,
        );
    }
}

/// Tokens sent straight to a reserve after launch issue no DTF. They raise
/// what every existing DTF redeems for, the locked minimum included.
#[test]
fn a_direct_transfer_after_launch_is_a_gift_to_every_holder() {
    let (mut env, market, alice, maker) = funded_market();
    env.transfer_tokens(
        TokenProgramKind::LegacySplToken,
        &maker.assets[0],
        &market.vaults[0],
        &maker.keypair,
        100_000,
    )
    .expect("direct transfer");
    let b = env.balances(&market, &[&alice]);
    assert_eq!(b.supply, 100_000, "no DTF issued");
    assert_eq!(b.reserves, vec![300_000, 500_000]);

    // A later buyer follows the new composition: 3 A and 5 B per DTF.
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[3_000, 5_000],
        BOB_SPEND,
        997,
    )
    .expect("purchase at the new ratio");
    assert_eq!(env.balance(&bob.dtf), 997);

    // Alice's 9,970 burned DTF now release 9,970 / 101,000 of 303,000 A.
    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    env.send(&[ix], &[&alice.keypair]).expect("redeem in kind");
    assert_eq!(
        env.balances(&market, &[&alice]).traders[0].assets,
        vec![29_910, 49_850]
    );
}

// ---------------------------------------------------------------------------
// Pause, resume and roles
// ---------------------------------------------------------------------------

fn send_admin(env: &mut Env, signer: &Keypair, ix: solana_instruction::Instruction) -> TxResult {
    env.send(&[ix], &[signer])
}

#[test]
fn purchases_and_usdc_redemption_pause_independently_and_in_kind_stays_open() {
    let (mut env, market, alice, maker) = funded_market();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);
    let operator = env.operator.insecure_clone();
    let governance = env.governance.insecure_clone();

    // The emergency operator pauses purchases alone.
    let ix = env.pause_ix(&address_of(&operator), &market, PAUSE_PURCHASE);
    send_admin(&mut env, &operator, ix).expect("pause purchases");
    let state = market_state(&env, &market);
    assert!(state.purchase_paused && !state.usdc_redemption_paused);

    let before = env.balances(&market, &[&alice, &bob, &maker]);
    assert_fails_with(
        acceptance_purchase(&mut env, &market, &bob, &maker, 0),
        AxisCoreError::PurchasePaused,
    );
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);
    redeem_usdc(
        &mut env,
        &market,
        &alice,
        &maker,
        10_000,
        &[19_940, 49_850],
        69_000,
        69_000,
    )
    .expect("USDC redemption is not paused");

    // Governance resumes purchases; the operator pauses USDC redemption.
    let ix = env.resume_ix(&address_of(&governance), &market, PAUSE_PURCHASE);
    send_admin(&mut env, &governance, ix).expect("resume purchases");
    let ix = env.pause_ix(&address_of(&operator), &market, PAUSE_USDC_REDEMPTION);
    send_admin(&mut env, &operator, ix).expect("pause USDC redemption");
    let state = market_state(&env, &market);
    assert!(!state.purchase_paused && state.usdc_redemption_paused);

    let before = env.balances(&market, &[&alice, &bob, &maker]);
    assert_fails_with(
        redeem_usdc(&mut env, &market, &alice, &maker, 1_000, &[1, 1], 1, 0),
        AxisCoreError::UsdcRedemptionPaused,
    );
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);
    purchase(
        &mut env,
        &market,
        false,
        &bob,
        &maker,
        BOB_SPEND,
        &[1_801, 4_502],
        BOB_SPEND,
        1,
    )
    .expect("purchases are not paused");

    // With both paused, the in-kind exit is unchanged.
    let ix = env.pause_ix(
        &address_of(&operator),
        &market,
        PAUSE_PURCHASE | PAUSE_USDC_REDEMPTION,
    );
    send_admin(&mut env, &operator, ix).expect("pause both");
    let state = market_state(&env, &market);
    assert!(state.purchase_paused && state.usdc_redemption_paused);

    let b = env.balances(&market, &[&alice]);
    let burned = 10_000 - 30;
    let expected: Vec<u64> = b
        .reserves
        .iter()
        .map(|r| (*r as u128 * burned as u128 / b.supply as u128) as u64)
        .collect();
    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    env.send(&[ix], &[&alice.keypair])
        .expect("in-kind redemption during both pauses");
    let after = env.balances(&market, &[&alice]);
    assert_eq!(after.traders[0].assets, expected);
    assert_eq!(after.treasury_dtf - b.treasury_dtf, 30);
    assert_eq!(after.supply, b.supply - burned);

    // Governance resumes both at once.
    let ix = env.resume_ix(
        &address_of(&governance),
        &market,
        PAUSE_PURCHASE | PAUSE_USDC_REDEMPTION,
    );
    send_admin(&mut env, &governance, ix).expect("resume both");
    let state = market_state(&env, &market);
    assert!(!state.purchase_paused && !state.usdc_redemption_paused);
}

#[test]
fn only_the_operator_or_governance_pauses_and_only_governance_resumes() {
    let (mut env, market, _alice, _maker) = funded_market();
    let operator = env.operator.insecure_clone();
    let governance = env.governance.insecure_clone();
    let outsider = env.funded_keypair("outsider");
    let both = PAUSE_PURCHASE | PAUSE_USDC_REDEMPTION;

    for signer in [
        &outsider,
        &env.deployer.insecure_clone(),
        &env.treasury.insecure_clone(),
    ] {
        let ix = env.pause_ix(&address_of(signer), &market, both);
        assert_fails_with(
            send_admin(&mut env, signer, ix),
            AxisCoreError::UnauthorizedPauseAuthority,
        );
    }
    let state = market_state(&env, &market);
    assert!(!state.purchase_paused && !state.usdc_redemption_paused);

    // Naming the operator without its signature is not enough.
    let mut ix = env.pause_ix(&address_of(&operator), &market, both);
    ix.accounts[0].is_signer = false;
    assert_fails_with(
        send_admin(&mut env, &outsider, ix),
        AxisCoreError::AccountNotSigner,
    );

    for flags in [0u8, 4, 0xff] {
        let ix = env.pause_ix(&address_of(&operator), &market, flags);
        assert_fails_with(
            send_admin(&mut env, &operator, ix),
            AxisCoreError::InvalidPauseFlags,
        );
    }

    // Governance can pause too, and pausing twice is not an error.
    let ix = env.pause_ix(&address_of(&governance), &market, PAUSE_PURCHASE);
    send_admin(&mut env, &governance, ix).expect("governance pauses");
    let ix = env.pause_ix(&address_of(&operator), &market, both);
    send_admin(&mut env, &operator, ix).expect("operator pauses");

    // The emergency operator cannot resume. Neither can anyone else.
    for signer in [&operator, &outsider, &env.deployer.insecure_clone()] {
        let ix = env.resume_ix(&address_of(signer), &market, both);
        assert_fails_with(
            send_admin(&mut env, signer, ix),
            AxisCoreError::UnauthorizedProtocolAuthority,
        );
    }
    let state = market_state(&env, &market);
    assert!(state.purchase_paused && state.usdc_redemption_paused);

    let ix = env.resume_ix(&address_of(&governance), &market, PAUSE_USDC_REDEMPTION);
    send_admin(&mut env, &governance, ix).expect("governance resumes one");
    let state = market_state(&env, &market);
    assert!(state.purchase_paused && !state.usdc_redemption_paused);
}

#[test]
fn a_pause_applies_to_one_dtf_only() {
    let mut env = Env::new();
    let first = env.create_market(&MarketSpec::two_assets("first"));
    let second = env.create_market(&MarketSpec::two_assets("second"));
    let operator = env.operator.insecure_clone();

    let ix = env.pause_ix(
        &address_of(&operator),
        &first,
        PAUSE_PURCHASE | PAUSE_USDC_REDEMPTION,
    );
    send_admin(&mut env, &operator, ix).expect("pause the first");

    let state = market_state(&env, &first);
    assert!(state.purchase_paused && state.usdc_redemption_paused);
    let state = market_state(&env, &second);
    assert!(!state.purchase_paused && !state.usdc_redemption_paused);

    let maker = maker(&mut env, &second);
    let alice = buyer(&mut env, "alice", &second, ALICE_USDC);
    purchase(
        &mut env,
        &second,
        true,
        &alice,
        &maker,
        700_000,
        &[200_000, 500_000],
        700_000,
        98_700,
    )
    .expect("the other DTF still sells");
}

#[test]
fn only_governance_changes_governance_and_the_emergency_operator() {
    let (mut env, market, _alice, _maker) = funded_market();
    let governance = env.governance.insecure_clone();
    let operator = env.operator.insecure_clone();
    let new_governance = env.funded_keypair("new-governance");
    let new_operator = env.funded_keypair("new-operator");
    let config_before = config_state(&env);

    for signer in [&operator, &new_governance, &env.deployer.insecure_clone()] {
        let ix = env.update_authorities_ix(
            &address_of(signer),
            &address_of(signer),
            &address_of(signer),
        );
        assert_fails_with(
            send_admin(&mut env, signer, ix),
            AxisCoreError::UnauthorizedProtocolAuthority,
        );
    }
    let ix = env.update_authorities_ix(
        &address_of(&governance),
        &Address::default(),
        &address_of(&new_operator),
    );
    assert_fails_with(
        send_admin(&mut env, &governance, ix),
        AxisCoreError::InvalidAuthority,
    );
    assert_eq!(config_state(&env), config_before);

    // Replace the operator, keep governance.
    let ix = env.update_authorities_ix(
        &address_of(&governance),
        &address_of(&governance),
        &address_of(&new_operator),
    );
    send_admin(&mut env, &governance, ix).expect("replace the operator");
    let ix = env.pause_ix(&address_of(&operator), &market, PAUSE_PURCHASE);
    assert_fails_with(
        send_admin(&mut env, &operator, ix),
        AxisCoreError::UnauthorizedPauseAuthority,
    );
    let ix = env.pause_ix(&address_of(&new_operator), &market, PAUSE_PURCHASE);
    send_admin(&mut env, &new_operator, ix).expect("the new operator pauses");

    // Hand governance over.
    let ix = env.update_authorities_ix(
        &address_of(&governance),
        &address_of(&new_governance),
        &address_of(&new_operator),
    );
    send_admin(&mut env, &governance, ix).expect("hand governance over");
    let ix = env.resume_ix(&address_of(&governance), &market, PAUSE_PURCHASE);
    assert_fails_with(
        send_admin(&mut env, &governance, ix),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );
    let ix = env.resume_ix(&address_of(&new_governance), &market, PAUSE_PURCHASE);
    send_admin(&mut env, &new_governance, ix).expect("the new governance resumes");

    let config = config_state(&env);
    assert_eq!(config.protocol_authority, address_of(&new_governance));
    assert_eq!(config.emergency_operator, address_of(&new_operator));
    assert_eq!(config.protocol_treasury, config_before.protocol_treasury);
    assert_eq!(config.usdc_mint, config_before.usdc_mint);
}

/// Until governance approves a swap program, a market cannot be bought or
/// redeemed to USDC. In-kind redemption does not depend on one.
#[test]
fn a_market_without_an_approved_swap_program_only_redeems_in_kind() {
    let (mut env, market, alice, maker) = funded_market();
    let governance = env.governance.insecure_clone();
    let operator = env.operator.insecure_clone();
    let bob = buyer(&mut env, "bob", &market, BOB_USDC);

    let ix = env.set_swap_program_ix(&address_of(&operator), &market, &Address::default());
    assert_fails_with(
        send_admin(&mut env, &operator, ix),
        AxisCoreError::UnauthorizedProtocolAuthority,
    );
    let program_id = env.program_id;
    let ix = env.set_swap_program_ix(&address_of(&governance), &market, &program_id);
    assert_fails_with(
        send_admin(&mut env, &governance, ix),
        AxisCoreError::InvalidProgramAccount,
    );

    let ix = env.set_swap_program_ix(&address_of(&governance), &market, &Address::default());
    send_admin(&mut env, &governance, ix).expect("clear the swap program");

    let before = env.balances(&market, &[&alice, &bob, &maker]);
    assert_fails_with(
        acceptance_purchase(&mut env, &market, &bob, &maker, 0),
        AxisCoreError::SwapProgramNotConfigured,
    );
    assert_fails_with(
        redeem_usdc(
            &mut env,
            &market,
            &alice,
            &maker,
            10_000,
            &[19_940, 49_850],
            69_000,
            0,
        ),
        AxisCoreError::SwapProgramNotConfigured,
    );
    assert_eq!(env.balances(&market, &[&alice, &bob, &maker]), before);

    let ix = env.redeem_in_kind_ix(&market, &alice, 10_000);
    env.send(&[ix], &[&alice.keypair])
        .expect("in-kind redemption needs no swap program");

    let ix = env.set_swap_program_ix(
        &address_of(&governance),
        &market,
        &legacy_token_program_id(),
    );
    send_admin(&mut env, &governance, ix).expect("approve the swap program");
    acceptance_purchase(&mut env, &market, &bob, &maker, 1).expect("purchases reopen");
}

#[test]
fn an_unknown_instruction_tag_is_rejected() {
    let mut env = Env::new();
    let deployer = env.deployer.insecure_clone();
    for data in [vec![], vec![10u8], vec![255u8]] {
        let ix = solana_instruction::Instruction {
            program_id: to_pubkey(&env.program_id),
            accounts: vec![],
            data,
        };
        assert_fails_with(
            env.send(&[ix], &[&deployer]),
            AxisCoreError::InvalidInstruction,
        );
    }
}
