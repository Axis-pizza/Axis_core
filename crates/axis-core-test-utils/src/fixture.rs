//! Market fixtures for the Axis Core invocation tests.
//!
//! Everything here sets up state the way a deployment would: real SPL Token
//! and Token-2022 mints and accounts, and Axis Core's own instructions for
//! every Axis-owned account. Amounts passed in are test fixtures only; none
//! is a production parameter.
//!
//! ## The route stand-in
//!
//! Purchases and USDC redemptions run their swaps through the market's
//! approved swap program. These fixtures approve the legacy SPL Token program
//! in that role, and a "swap" is a pair of plain token transfers against a
//! `maker` who co-signs the transaction: the user pays the maker, the maker
//! pays the reserve vaults (or the user, on a redemption). That exercises
//! Core's real CPI path, balance measurement and rollback, but it is not
//! Jupiter: account counts, compute use and route behaviour for a production
//! route are not shown by any test that uses it.

use axis_core::constants::{MARKET_SEED, PROTOCOL_CONFIG_SEED, SYSTEM_PROGRAM_ID};
use axis_core::instructions::AxisInstruction;
use axis_core::state::TokenProgramKind;
use litesvm::types::{FailedTransactionMetadata, TransactionMetadata};
use litesvm::LiteSVM;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_message::Message;
use solana_program_pack::Pack;
use solana_transaction::Transaction;
use spl_token_2022_interface::{
    instruction as token_instruction,
    state::{Account as TokenAccount, Mint},
};

use crate::{
    deterministic_keypair, legacy_token_program_id, load_axis_core_program, token_2022_program_id,
    token_account_balance, token_mint_supply,
};

pub type TxResult = Result<TransactionMetadata, FailedTransactionMetadata>;

const SIGNER_LAMPORTS: u64 = 10_000_000_000;

pub fn to_pubkey(a: &Address) -> solana_pubkey::Pubkey {
    solana_pubkey::Pubkey::from(a.to_bytes())
}

pub fn address_of(k: &Keypair) -> Address {
    Address::try_from(k.pubkey().as_ref()).unwrap()
}

pub fn token_program_address(kind: TokenProgramKind) -> Address {
    match kind {
        TokenProgramKind::LegacySplToken => legacy_token_program_id(),
        TokenProgramKind::Token2022 => token_2022_program_id(),
    }
}

pub fn program_data_address(program_id: &Address) -> Address {
    Address::find_program_address(
        &[program_id.as_ref()],
        &axis_core::constants::BPF_LOADER_UPGRADEABLE_ID,
    )
    .0
}

pub fn config_address(program_id: &Address) -> Address {
    Address::find_program_address(&[PROTOCOL_CONFIG_SEED], program_id).0
}

pub fn market_address(program_id: &Address, dtf_mint: &Address) -> (Address, u8) {
    Address::find_program_address(&[MARKET_SEED, dtf_mint.as_ref()], program_id)
}

fn meta(address: &Address, writable: bool, signer: bool) -> AccountMeta {
    if writable {
        AccountMeta::new(to_pubkey(address), signer)
    } else {
        AccountMeta::new_readonly(to_pubkey(address), signer)
    }
}

/// One route leg: an instruction for the market's swap program.
#[derive(Clone, Debug)]
pub struct Leg {
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

/// A plain token `Transfer` as a route leg. `authority` must sign the
/// transaction.
pub fn transfer_leg(from: &Address, to: &Address, authority: &Address, amount: u64) -> Leg {
    let mut data = vec![3u8];
    data.extend_from_slice(&amount.to_le_bytes());
    Leg {
        accounts: vec![
            meta(from, true, false),
            meta(to, true, false),
            meta(authority, false, true),
        ],
        data,
    }
}

pub fn encode_legs(legs: &[Leg]) -> (Vec<u8>, Vec<AccountMeta>) {
    let mut data = vec![legs.len() as u8];
    let mut accounts = Vec::new();
    for leg in legs {
        data.push(leg.accounts.len() as u8);
        data.extend_from_slice(&(leg.data.len() as u16).to_le_bytes());
        data.extend_from_slice(&leg.data);
        accounts.extend(leg.accounts.iter().cloned());
    }
    (data, accounts)
}

pub struct MarketSpec {
    pub label: &'static str,
    pub dtf_kind: TokenProgramKind,
    pub reference_supply: u64,
    pub reference_amounts: Vec<u64>,
    pub min_liquidity: u64,
    pub swap_program: Address,
}

impl MarketSpec {
    /// Two constituents at 2 A and 5 B per DTF, a Token-2022 DTF, and the
    /// legacy token program as the route stand-in.
    pub fn two_assets(label: &'static str) -> Self {
        Self {
            label,
            dtf_kind: TokenProgramKind::Token2022,
            reference_supply: 1_000,
            reference_amounts: vec![2_000, 5_000],
            min_liquidity: 1_000,
            swap_program: legacy_token_program_id(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketFixture {
    pub market: Address,
    pub dtf_mint: Address,
    pub dtf_kind: TokenProgramKind,
    pub liquidity_lock: Address,
    pub treasury_dtf: Address,
    pub asset_mints: Vec<Address>,
    pub vaults: Vec<Address>,
    pub swap_program: Address,
    pub reference_supply: u64,
    pub reference_amounts: Vec<u64>,
    pub min_liquidity: u64,
    /// The `token_program` and `token_2022_program` slots of the redemption
    /// instructions.
    pub token_program_slots: [Address; 2],
}

/// A wallet with a USDC account, a DTF account and one account per
/// constituent. Buyers, holders and the route's maker are all this.
#[derive(Debug)]
pub struct Trader {
    pub keypair: Keypair,
    pub usdc: Address,
    pub dtf: Address,
    pub assets: Vec<Address>,
}

impl Trader {
    pub fn address(&self) -> Address {
        address_of(&self.keypair)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraderBalances {
    pub usdc: u64,
    pub dtf: u64,
    pub assets: Vec<u64>,
}

/// Every balance a purchase or redemption can touch. Two equal snapshots
/// around a failed instruction are the proof that it committed nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Balances {
    pub supply: u64,
    pub reserves: Vec<u64>,
    pub lock: u64,
    pub treasury_dtf: u64,
    pub traders: Vec<TraderBalances>,
}

pub struct Env {
    pub vm: LiteSVM,
    pub program_id: Address,
    /// The program's upgrade authority. Pays for fixture setup.
    pub deployer: Keypair,
    pub governance: Keypair,
    pub operator: Keypair,
    /// Owner of the accounts that receive the fee.
    pub treasury: Keypair,
    pub usdc_mint: Address,
    /// Mint authority of the fixture USDC and constituent mints.
    pub token_authority: Keypair,
    counter: u32,
}

impl Env {
    /// A loaded program whose upgrade authority is `deployer`, with no
    /// protocol config yet.
    pub fn uninitialized() -> Self {
        let mut vm = LiteSVM::new();
        let program_id = load_axis_core_program(&mut vm)
            .expect("target/deploy/axis_core.so must exist; build it with scripts/build-sbf.sh")
            .program_id;
        let mut env = Self {
            vm,
            program_id,
            deployer: deterministic_keypair("deployer"),
            governance: deterministic_keypair("governance"),
            operator: deterministic_keypair("emergency-operator"),
            treasury: deterministic_keypair("treasury"),
            usdc_mint: Address::default(),
            token_authority: deterministic_keypair("token-authority"),
            counter: 0,
        };
        for signer in [
            address_of(&env.deployer),
            address_of(&env.governance),
            address_of(&env.operator),
            address_of(&env.treasury),
            address_of(&env.token_authority),
        ] {
            env.vm.airdrop(&signer, SIGNER_LAMPORTS).expect("airdrop");
        }
        env.set_upgrade_authority(Some(address_of(&env.deployer)));
        env.usdc_mint = env.create_mint(TokenProgramKind::LegacySplToken, None, None);
        env
    }

    /// A loaded program with its protocol config initialized.
    pub fn new() -> Self {
        let mut env = Self::uninitialized();
        let ix = env.init_config_ix(&address_of(&env.deployer));
        let deployer = env.deployer.insecure_clone();
        env.send(&[ix], &[&deployer]).expect("initialize config");
        env
    }

    /// LiteSVM deploys with no upgrade authority, which is what a finalized
    /// program looks like. A real deployment has one until it is finalized.
    pub fn set_upgrade_authority(&mut self, authority: Option<Address>) {
        let address = program_data_address(&self.program_id);
        let mut account = self.vm.get_account(&address).expect("program data account");
        match authority {
            Some(authority) => {
                account.data[12] = 1;
                account.data[13..45].copy_from_slice(authority.as_ref());
            }
            None => account.data[12] = 0,
        }
        self.vm
            .set_account(address, account)
            .expect("set upgrade authority");
    }

    /// Sends one transaction. The first signer pays the fee.
    pub fn send(&mut self, instructions: &[Instruction], signers: &[&Keypair]) -> TxResult {
        self.vm.expire_blockhash();
        let message = Message::new(instructions, Some(&signers[0].pubkey()));
        let tx = Transaction::new(signers, message, self.vm.latest_blockhash());
        self.vm.send_transaction(tx)
    }

    pub fn funded_keypair(&mut self, label: &str) -> Keypair {
        let keypair = deterministic_keypair(label);
        self.vm
            .airdrop(&address_of(&keypair), SIGNER_LAMPORTS)
            .expect("airdrop");
        keypair
    }

    fn next_keypair(&mut self, what: &str) -> Keypair {
        self.counter += 1;
        deterministic_keypair(format!("fixture:{what}:{}", self.counter))
    }

    pub fn init_config_ix(&self, upgrade_authority: &Address) -> Instruction {
        self.init_config_ix_at(
            upgrade_authority,
            &config_address(&self.program_id),
            &program_data_address(&self.program_id),
            &address_of(&self.governance),
        )
    }

    pub fn init_config_ix_at(
        &self,
        upgrade_authority: &Address,
        config: &Address,
        program_data: &Address,
        governance: &Address,
    ) -> Instruction {
        let mut data = vec![AxisInstruction::InitializeProtocolConfig as u8];
        data.extend_from_slice(governance.as_ref());
        data.extend_from_slice(self.usdc_mint.as_ref());
        data.extend_from_slice(self.treasury.pubkey().as_ref());
        data.extend_from_slice(self.operator.pubkey().as_ref());
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts: vec![
                meta(upgrade_authority, true, true),
                meta(config, true, false),
                meta(&SYSTEM_PROGRAM_ID, false, false),
                meta(program_data, false, false),
            ],
            data,
        }
    }

    /// A mint under `kind`'s token program. The mint authority defaults to
    /// the fixture token authority.
    pub fn create_mint(
        &mut self,
        kind: TokenProgramKind,
        mint_authority: Option<Address>,
        freeze_authority: Option<Address>,
    ) -> Address {
        let mint = self.next_keypair("mint");
        let program = token_program_address(kind);
        let authority = mint_authority.unwrap_or_else(|| address_of(&self.token_authority));
        let deployer = self.deployer.insecure_clone();
        let create = solana_system_interface::instruction::create_account(
            &deployer.pubkey(),
            &mint.pubkey(),
            self.vm.minimum_balance_for_rent_exemption(Mint::LEN),
            Mint::LEN as u64,
            &program,
        );
        let init = token_instruction::initialize_mint2(
            &program,
            &mint.pubkey(),
            &authority,
            freeze_authority.as_ref(),
            6,
        )
        .expect("initialize_mint2");
        self.send(&[create, init], &[&deployer, &mint])
            .expect("create mint");
        address_of(&mint)
    }

    /// A bare 165-byte token account for `mint`, owned by `owner`.
    pub fn create_token_account(
        &mut self,
        kind: TokenProgramKind,
        mint: &Address,
        owner: &Address,
    ) -> Address {
        let account = self.next_keypair("token-account");
        let program = token_program_address(kind);
        let deployer = self.deployer.insecure_clone();
        let create = solana_system_interface::instruction::create_account(
            &deployer.pubkey(),
            &account.pubkey(),
            self.vm
                .minimum_balance_for_rent_exemption(TokenAccount::LEN),
            TokenAccount::LEN as u64,
            &program,
        );
        let init = token_instruction::initialize_account3(&program, &account.pubkey(), mint, owner)
            .expect("initialize_account3");
        self.send(&[create, init], &[&deployer, &account])
            .expect("create token account");
        address_of(&account)
    }

    /// Mints fixture USDC or a constituent. Not usable for a DTF: only the
    /// market PDA can mint those.
    pub fn mint_tokens(
        &mut self,
        kind: TokenProgramKind,
        mint: &Address,
        to: &Address,
        amount: u64,
    ) {
        let authority = self.token_authority.insecure_clone();
        let ix = token_instruction::mint_to(
            &token_program_address(kind),
            mint,
            to,
            &authority.pubkey(),
            &[],
            amount,
        )
        .expect("mint_to");
        self.send(&[ix], &[&authority]).expect("mint tokens");
    }

    /// A plain token transfer signed by `authority`, outside any Axis
    /// instruction.
    pub fn transfer_tokens(
        &mut self,
        kind: TokenProgramKind,
        from: &Address,
        to: &Address,
        authority: &Keypair,
        amount: u64,
    ) -> TxResult {
        let leg = transfer_leg(from, to, &address_of(authority), amount);
        let ix = Instruction {
            program_id: to_pubkey(&token_program_address(kind)),
            accounts: leg.accounts,
            data: leg.data,
        };
        self.send(&[ix], &[authority])
    }

    pub fn balance(&self, token_account: &Address) -> u64 {
        token_account_balance(&self.vm, token_account).expect("token account")
    }

    pub fn supply(&self, mint: &Address) -> u64 {
        token_mint_supply(&self.vm, mint).expect("mint")
    }

    /// Creates every token account a market needs, without calling
    /// `CreateMarket`: the DTF mint with the market PDA as its mint
    /// authority, a legacy mint and a PDA-owned vault per constituent, the
    /// PDA-owned liquidity lock, and the treasury's DTF account.
    pub fn prepare_market(&mut self, spec: &MarketSpec) -> MarketFixture {
        let dtf_keypair = deterministic_keypair(format!("dtf-mint:{}", spec.label));
        let dtf_mint = address_of(&dtf_keypair);
        let (market, _) = market_address(&self.program_id, &dtf_mint);

        let program = token_program_address(spec.dtf_kind);
        let deployer = self.deployer.insecure_clone();
        let create = solana_system_interface::instruction::create_account(
            &deployer.pubkey(),
            &dtf_keypair.pubkey(),
            self.vm.minimum_balance_for_rent_exemption(Mint::LEN),
            Mint::LEN as u64,
            &program,
        );
        let init =
            token_instruction::initialize_mint2(&program, &dtf_keypair.pubkey(), &market, None, 6)
                .expect("initialize_mint2");
        self.send(&[create, init], &[&deployer, &dtf_keypair])
            .expect("create DTF mint");

        let liquidity_lock = self.create_token_account(spec.dtf_kind, &dtf_mint, &market);
        let treasury = address_of(&self.treasury);
        let treasury_dtf = self.create_token_account(spec.dtf_kind, &dtf_mint, &treasury);

        let mut asset_mints = Vec::new();
        let mut vaults = Vec::new();
        for _ in &spec.reference_amounts {
            let mint = self.create_mint(TokenProgramKind::LegacySplToken, None, None);
            vaults.push(self.create_token_account(
                TokenProgramKind::LegacySplToken,
                &mint,
                &market,
            ));
            asset_mints.push(mint);
        }

        MarketFixture {
            market,
            dtf_mint,
            dtf_kind: spec.dtf_kind,
            liquidity_lock,
            treasury_dtf,
            asset_mints,
            vaults,
            swap_program: spec.swap_program,
            reference_supply: spec.reference_supply,
            reference_amounts: spec.reference_amounts.clone(),
            min_liquidity: spec.min_liquidity,
            token_program_slots: [legacy_token_program_id(), token_2022_program_id()],
        }
    }

    pub fn create_market_ix(&self, market: &MarketFixture, authority: &Address) -> Instruction {
        self.create_market_ix_with_config(market, authority, &config_address(&self.program_id))
    }

    pub fn create_market_ix_with_config(
        &self,
        market: &MarketFixture,
        authority: &Address,
        config: &Address,
    ) -> Instruction {
        let mut data = vec![AxisInstruction::CreateMarket as u8];
        data.extend_from_slice(market.swap_program.as_ref());
        data.extend_from_slice(&market.reference_supply.to_le_bytes());
        data.extend_from_slice(&market.min_liquidity.to_le_bytes());
        data.push(market.reference_amounts.len() as u8);
        for amount in &market.reference_amounts {
            data.extend_from_slice(&amount.to_le_bytes());
        }
        let mut accounts = vec![
            meta(authority, true, true),
            meta(config, false, false),
            meta(&market.market, true, false),
            meta(&market.dtf_mint, false, false),
            meta(&market.liquidity_lock, false, false),
            meta(&SYSTEM_PROGRAM_ID, false, false),
        ];
        accounts.extend(market.vaults.iter().map(|v| meta(v, false, false)));
        accounts.extend(market.asset_mints.iter().map(|m| meta(m, false, false)));
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts,
            data,
        }
    }

    /// Governance creates the market.
    pub fn create_market(&mut self, spec: &MarketSpec) -> MarketFixture {
        let market = self.prepare_market(spec);
        let governance = self.governance.insecure_clone();
        let ix = self.create_market_ix(&market, &address_of(&governance));
        self.send(&[ix], &[&governance]).expect("create market");
        market
    }

    /// A funded wallet with empty accounts for USDC, the DTF and every
    /// constituent of `market`.
    pub fn trader(&mut self, label: &str, market: &MarketFixture) -> Trader {
        let keypair = self.funded_keypair(label);
        let owner = address_of(&keypair);
        let usdc_mint = self.usdc_mint;
        let usdc = self.create_token_account(TokenProgramKind::LegacySplToken, &usdc_mint, &owner);
        let dtf = self.create_token_account(market.dtf_kind, &market.dtf_mint, &owner);
        let assets = market
            .asset_mints
            .iter()
            .map(|mint| self.create_token_account(TokenProgramKind::LegacySplToken, mint, &owner))
            .collect();
        Trader {
            keypair,
            usdc,
            dtf,
            assets,
        }
    }

    pub fn fund_usdc(&mut self, trader: &Trader, amount: u64) {
        let usdc_mint = self.usdc_mint;
        self.mint_tokens(
            TokenProgramKind::LegacySplToken,
            &usdc_mint,
            &trader.usdc,
            amount,
        );
    }

    pub fn fund_assets(&mut self, market: &MarketFixture, trader: &Trader, amounts: &[u64]) {
        for ((mint, account), amount) in market.asset_mints.iter().zip(&trader.assets).zip(amounts)
        {
            self.mint_tokens(TokenProgramKind::LegacySplToken, mint, account, *amount);
        }
    }

    pub fn balances(&self, market: &MarketFixture, traders: &[&Trader]) -> Balances {
        Balances {
            supply: self.supply(&market.dtf_mint),
            reserves: market.vaults.iter().map(|v| self.balance(v)).collect(),
            lock: self.balance(&market.liquidity_lock),
            treasury_dtf: self.balance(&market.treasury_dtf),
            traders: traders
                .iter()
                .map(|t| TraderBalances {
                    usdc: self.balance(&t.usdc),
                    dtf: self.balance(&t.dtf),
                    assets: t.assets.iter().map(|a| self.balance(a)).collect(),
                })
                .collect(),
        }
    }

    /// `Purchase`, or `InitialPurchase` when `initial`.
    pub fn purchase_ix(
        &self,
        market: &MarketFixture,
        initial: bool,
        buyer: &Trader,
        max_usdc_in: u64,
        min_net_dtf_out: u64,
        legs: &[Leg],
    ) -> Instruction {
        let tag = if initial {
            AxisInstruction::InitialPurchase
        } else {
            AxisInstruction::Purchase
        };
        let (route_data, route_accounts) = encode_legs(legs);
        let mut data = vec![tag as u8];
        data.extend_from_slice(&max_usdc_in.to_le_bytes());
        data.extend_from_slice(&min_net_dtf_out.to_le_bytes());
        data.extend_from_slice(&route_data);

        let mut accounts = vec![
            meta(&buyer.address(), false, true),
            meta(&market.market, false, false),
            meta(&market.dtf_mint, true, false),
            meta(&buyer.usdc, true, false),
            meta(&buyer.dtf, true, false),
            meta(&market.treasury_dtf, true, false),
            meta(&token_program_address(market.dtf_kind), false, false),
            meta(&market.swap_program, false, false),
        ];
        if initial {
            accounts.push(meta(&market.liquidity_lock, true, false));
        }
        accounts.extend(market.vaults.iter().map(|v| meta(v, true, false)));
        accounts.extend(route_accounts);
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts,
            data,
        }
    }

    pub fn redeem_in_kind_ix(
        &self,
        market: &MarketFixture,
        holder: &Trader,
        dtf_in: u64,
    ) -> Instruction {
        let mut data = vec![AxisInstruction::RedeemInKind as u8];
        data.extend_from_slice(&dtf_in.to_le_bytes());
        let mut accounts = vec![
            meta(&holder.address(), false, true),
            meta(&market.market, false, false),
            meta(&market.dtf_mint, true, false),
            meta(&holder.dtf, true, false),
            meta(&market.treasury_dtf, true, false),
            meta(&market.token_program_slots[0], false, false),
            meta(&market.token_program_slots[1], false, false),
        ];
        accounts.extend(market.vaults.iter().map(|v| meta(v, true, false)));
        accounts.extend(holder.assets.iter().map(|a| meta(a, true, false)));
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts,
            data,
        }
    }

    pub fn redeem_usdc_ix(
        &self,
        market: &MarketFixture,
        holder: &Trader,
        dtf_in: u64,
        min_net_usdc_out: u64,
        legs: &[Leg],
    ) -> Instruction {
        let (route_data, route_accounts) = encode_legs(legs);
        let mut data = vec![AxisInstruction::RedeemUsdc as u8];
        data.extend_from_slice(&dtf_in.to_le_bytes());
        data.extend_from_slice(&min_net_usdc_out.to_le_bytes());
        data.extend_from_slice(&route_data);
        let mut accounts = vec![
            meta(&holder.address(), false, true),
            meta(&market.market, false, false),
            meta(&market.dtf_mint, true, false),
            meta(&holder.dtf, true, false),
            meta(&market.treasury_dtf, true, false),
            meta(&holder.usdc, true, false),
            meta(&market.token_program_slots[0], false, false),
            meta(&market.token_program_slots[1], false, false),
            meta(&market.swap_program, false, false),
        ];
        accounts.extend(market.vaults.iter().map(|v| meta(v, true, false)));
        accounts.extend(holder.assets.iter().map(|a| meta(a, true, false)));
        accounts.extend(route_accounts);
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts,
            data,
        }
    }

    fn market_admin_ix(
        &self,
        tag: AxisInstruction,
        signer: &Address,
        market: &Address,
        payload: &[u8],
    ) -> Instruction {
        let mut data = vec![tag as u8];
        data.extend_from_slice(payload);
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts: vec![
                meta(signer, false, true),
                meta(&config_address(&self.program_id), false, false),
                meta(market, true, false),
            ],
            data,
        }
    }

    pub fn pause_ix(&self, signer: &Address, market: &MarketFixture, flags: u8) -> Instruction {
        self.market_admin_ix(AxisInstruction::Pause, signer, &market.market, &[flags])
    }

    pub fn resume_ix(&self, signer: &Address, market: &MarketFixture, flags: u8) -> Instruction {
        self.market_admin_ix(AxisInstruction::Resume, signer, &market.market, &[flags])
    }

    pub fn set_swap_program_ix(
        &self,
        signer: &Address,
        market: &MarketFixture,
        swap_program: &Address,
    ) -> Instruction {
        self.market_admin_ix(
            AxisInstruction::SetSwapProgram,
            signer,
            &market.market,
            swap_program.as_ref(),
        )
    }

    pub fn update_authorities_ix(
        &self,
        signer: &Address,
        new_governance: &Address,
        new_operator: &Address,
    ) -> Instruction {
        let mut data = vec![AxisInstruction::UpdateAuthorities as u8];
        data.extend_from_slice(new_governance.as_ref());
        data.extend_from_slice(new_operator.as_ref());
        Instruction {
            program_id: to_pubkey(&self.program_id),
            accounts: vec![
                meta(signer, false, true),
                meta(&config_address(&self.program_id), true, false),
            ],
            data,
        }
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

/// The purchase route stand-in: the buyer pays `usdc` to the maker, and the
/// maker delivers `deliveries[i]` of each constituent to its reserve vault.
pub fn purchase_legs(
    market: &MarketFixture,
    buyer: &Trader,
    maker: &Trader,
    usdc: u64,
    deliveries: &[u64],
) -> Vec<Leg> {
    let mut legs = vec![transfer_leg(
        &buyer.usdc,
        &maker.usdc,
        &buyer.address(),
        usdc,
    )];
    for (i, amount) in deliveries.iter().enumerate() {
        legs.push(transfer_leg(
            &maker.assets[i],
            &market.vaults[i],
            &maker.address(),
            *amount,
        ));
    }
    legs
}

/// The redemption route stand-in: the holder sells `sold[i]` of each released
/// constituent to the maker, and the maker pays `usdc` to the holder.
pub fn redemption_legs(holder: &Trader, maker: &Trader, sold: &[u64], usdc: u64) -> Vec<Leg> {
    let mut legs: Vec<Leg> = sold
        .iter()
        .enumerate()
        .map(|(i, amount)| {
            transfer_leg(
                &holder.assets[i],
                &maker.assets[i],
                &holder.address(),
                *amount,
            )
        })
        .collect();
    legs.push(transfer_leg(
        &maker.usdc,
        &holder.usdc,
        &maker.address(),
        usdc,
    ));
    legs
}
