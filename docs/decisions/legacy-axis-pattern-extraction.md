# Legacy Axis Pattern Extraction

## 1. Purpose

This is a research-only extraction from the legacy Axis repository. It is not a compatibility plan, an endorsement of the legacy design, or an instruction to port code.

The purpose is to identify on-chain safety patterns that may inform a separate Axis Core implementation, while explicitly rejecting the legacy basket-accounting model where it conflicts with Axis v1 invariants. Classifications describe ideas, not reusable source modules.

`H` in the inventory means the inspected repository state at commit `535c0b33ca3589b8e7a6d7ee8d57932d4719ee78`.

## 2. Source Snapshot

| Repository | Branch | Commit SHA | Inspected paths |
|---|---|---|---|
| `Axis-pizza/Axis_AMM` (`https://github.com/Axis-pizza/Axis_AMM.git`) | `main` | `535c0b33ca3589b8e7a6d7ee8d57932d4719ee78` | `contracts/axis-vault/src/**`; `contracts/ab-integration-tests/tests/axis_vault_{coverage,rebalance,setfee_setcap,sol_ix_scaffold,withdraw_sol_bound}.rs`; `contracts/ab-integration-tests/{mock-jupiter,src/helpers}/**`; limited venue search in `legacy/axis-g3m/tests/jupiter_cpi.rs` |

Observations are based on the current tree at `H`; a file's last historical edit may predate `H`. No claim below treats a legacy deployment, legacy specification, or frontend behavior as canonical for Axis Core.

## 3. New Axis Core Context

Axis Core is a new Token-2022 DTF program with this P0 lifecycle:

```text
USDC in
→ approved execution
→ reserve assets held by program-controlled vaults
→ Token-2022 DTF token minted
→ DTF burned on redeem
→ reserve assets unwound
→ USDC out
```

The accounting boundary is the observed token-account balance delta in Axis-controlled vaults. Target weights, quotes, and off-chain calculations may be inputs to validation or policy, but are never reserve/NAV truth. Creator and protocol fees must be kept distinct from reserve/NAV accounting. Each execution must be authorized by on-chain `ApprovedRoute` validation.

Local controlled-adapter tests can validate P0 containment properties. They are not evidence that a production venue works. Before mainnet, Axis Core requires production paths and tests for Orca Whirlpool and Raydium CPMM fallback. External DTF/USDC pools remain external liquidity.

## 4. Pattern Inventory

| Pattern | Source path | Commit SHA | What it does | Classification | Axis Core relevance | Risks / caveats |
|---|---|---|---|---|---|---|
| Deterministic state PDA with stored bump | `contracts/axis-vault/src/instructions/create_etf.rs`; `contracts/axis-vault/src/state/etf.rs` | H (`535c0b33…`) | Derives `[b"etf", authority, name]`, stores the canonical bump, and makes that PDA mint and vault authority. | Adapt | Use deterministic, domain-separated PDAs for Axis Core config, each DTF, and each reserve vault authority. Re-derive before any privileged mutation or PDA signature. | The seed scheme encodes creator/name and the old ETF layout; do not retain it. A stored bump is only safe when the PDA is re-derived from canonical state, not trusted by itself. |
| PDA re-derivation for privileged administration | `contracts/axis-vault/src/instructions/{set_paused,set_fee,set_cap}.rs`; `contracts/axis-vault/src/instructions/rebalance.rs` | H (`535c0b33…`) | Requires a signer, checks it against stored authority, and re-derives the ETF PDA from stored authority/name/bump before state mutation. | Adapt | Reuse the validation sequence for Axis Core governance and emergency controls: signer/role check, program ownership/type check, canonical PDA derivation, then mutation. | Legacy uses one creator authority and a name-derived seed. Axis Core needs an explicit governance/emergency-role model and a fresh account design. |
| Program-controlled reserve custody | `contracts/axis-vault/src/instructions/create_etf.rs`; `contracts/axis-vault/src/state/etf.rs` | H (`535c0b33…`) | Initializes one SPL token account per basket mint with the ETF PDA as token-account authority and stores its address. | Adapt | The core custody principle is directly relevant: every reserve mint needs a canonical Axis-controlled vault and an authority that only the program can sign for. | Legacy stores a fixed basket of up to five Legacy Token vaults. Axis Core must validate each vault's token program, mint, initialized state, and authority on every asset-moving path; address equality plus Token Program ownership is insufficient as a complete Token-2022 policy. |
| PDA mint authority and user-authorized burn | `contracts/axis-vault/src/instructions/{create_etf,deposit,withdraw}.rs` | H (`535c0b33…`) | The ETF state PDA signs `MintTo`; the user signs `Burn` from their ETF token account. | Adapt | DTF mint authority should be an Axis Core PDA; redemption should burn the user's Token-2022 DTF before releasing the corresponding value. | The legacy code relies materially on downstream SPL Token CPI checks and does not explicitly parse every user DTF account's mint/owner before CPI. Axis Core should make those checks explicit and reconcile expected versus observed supply/balance deltas. |
| Pause gate on asset-moving paths | `contracts/axis-vault/src/instructions/{deposit,withdraw,deposit_sol,withdraw_sol,rebalance}.rs`; `contracts/axis-vault/src/instructions/set_paused.rs` | H (`535c0b33…`) | A stored `paused` byte blocks deposit, withdraw, SOL routes, and rebalance; an authority-gated instruction changes it. | Adapt | A pause/emergency control is a useful P0 containment tool for mint, redeem, and adapters. | Define the pause matrix deliberately: whether a pause blocks new risk but permits bounded redemption is a new governance and insolvency-policy decision. Do not inherit the legacy all-or-nothing behavior by default. |
| Fee ceiling and mutation guard | `contracts/axis-vault/src/constants.rs`; `contracts/axis-vault/src/instructions/{create_etf,set_fee,set_cap}.rs` | H (`535c0b33…`) | Stores a per-ETF maximum fee, applies a program-wide ceiling, and authorizes changes only for the stored authority. It also has a monotonically non-decreasing supply cap. | Adapt | Static ceilings, explicit units, role checks, and independent protocol limits are useful guardrail concepts. | `tvl_cap` is denominated in the legacy internal `total_supply`, not USDC/NAV. New caps and fees need unambiguous USDC/value semantics, governance policy, and accounting separated from reserves. |
| Actual pre/post vault balance observation | `contracts/axis-vault/src/instructions/{deposit_sol,withdraw_sol,rebalance}.rs`; `contracts/axis-vault/src/jupiter.rs` | H (`535c0b33…`) | Reads token balances before and after CPI; DepositSol mints only after positive vault deltas, WithdrawSol bounds consumption, and Rebalance checks vault deltas. | Adapt | This is the most important reusable safety idea: mint/redeem accounting must be based on actual, correctly validated reserve-vault and USDC-vault deltas, with checked arithmetic and explicit bounds. | The legacy calculations turn those deltas into basket-share math and use `total_supply`/weights. Axis Core must instead define USDC-in, reserve-acquired, reserve-sold, USDC-out, and fee deltas independently of targets and quotes. |
| Post-CPI input-side drain bound | `contracts/axis-vault/src/instructions/withdraw_sol.rs`; `contracts/ab-integration-tests/tests/axis_vault_withdraw_sol_bound.rs` | H (`535c0b33…`) | Snapshots each vault, limits its observed loss to the user's pro-rata burn share after opaque Jupiter CPIs, and reverts on excess drain. | Adapt | For any adapter that receives a PDA signature, validate the maximum allowed source-vault decrease and required destination-vault/USDC increase from actual deltas. | The legacy bound is proportional basket redemption and is not an Axis Core formula. A post-CPI check does not replace route approval; transaction rollback contains an invalid CPI outcome, but the program must constrain what it authorizes up front. |
| Pinned CPI program ID | `contracts/axis-vault/src/jupiter.rs`; `contracts/axis-vault/src/instructions/{deposit_sol,withdraw_sol,rebalance}.rs` | H (`535c0b33…`) | Rejects a non-Jupiter program account before invoking CPI. | Adapt | Every Axis adapter should pin the expected venue program ID and verify the expected account role/mint relationships. | Program-ID equality alone is not route authorization. The legacy code accepts opaque caller-supplied route bytes and account lists, so it lacks the required on-chain `ApprovedRoute` validation. |
| Opaque Jupiter route bytes and caller-controlled CPI metas | `contracts/axis-vault/src/jupiter.rs`; `contracts/axis-vault/src/instructions/{deposit_sol,withdraw_sol,rebalance}.rs` | H (`535c0b33…`) | Builds CPI metas from caller-provided route accounts and passes opaque route bytes to a hard-coded Jupiter V6 program. | Reject | Do not use this as the Axis Core execution authorization model. `ApprovedRoute` must bind the allowed adapter/venue, pool, direction, reserve mint, USDC mint, canonical source/destination vaults, and bounded inputs/outputs on-chain. | The legacy program checks some resulting deltas, but it does not prove that a route is an approved venue/pool/path. It contains no Orca Whirlpool or Raydium CPMM adapter path. |
| Direct proportional basket deposit/withdraw | `contracts/axis-vault/src/instructions/{deposit,withdraw}.rs`; `contracts/axis-vault/src/state/etf.rs` | H (`535c0b33…`) | Accepts multiple user basket assets calculated from target weights, mints a share token, and later returns each reserve asset pro rata. | Reject | Axis Core must accept USDC for minting and return USDC on redemption after approved execution/unwind. | This is the central legacy product model, not a transportable safety pattern. It would violate the new lifecycle and complicate reserve truth. |
| Target-weight-based minting and NAV-deviation gate | `contracts/axis-vault/src/instructions/deposit.rs`; `contracts/axis-vault/src/constants.rs`; `contracts/axis-vault/src/state/etf.rs` | H (`535c0b33…`) | Derives basket input amounts from `weights_bps`, calculates per-vault mint candidates, and rejects a wide candidate spread. | Reject | Target weights may be strategy constraints, reporting values, or route-planning inputs only. | The code explicitly treats target composition as part of mint math. That is incompatible with the Axis Core rule that weights are not reserve/NAV accounting truth. |
| Program-maintained `total_supply` mirror | `contracts/axis-vault/src/state/etf.rs`; `contracts/axis-vault/src/instructions/{deposit,withdraw,deposit_sol,withdraw_sol,sweep_treasury}.rs` | H (`535c0b33…`) | Stores `EtfState.total_supply` and increments/decrements it from intended mint/burn amounts; legacy proportional payout and cap logic read that field. | Reject | Use actual Token-2022 mint/account balance observations or tightly reconciled canonical accounting; do not make an unverified duplicate counter the source of truth. | The legacy state does not compare its mirror with the mint account's actual supply after every lifecycle action. This creates a reconciliation surface and is not a substitute for actual reserve deltas. |
| Fee paid in DTF and treasury redemption sweep | `contracts/axis-vault/src/instructions/{deposit,withdraw,sweep_treasury}.rs` | H (`535c0b33…`) | Mints deposit fees as ETF tokens, transfers withdrawal fees as ETF tokens to treasury, then lets the treasury burn those tokens for a basket share. | Reject | Axis Core needs separate creator/protocol fee accounting and explicit fee destinations that do not obscure reserve/NAV accounting. | A treasury holding redeemable DTF creates a claim on reserves and combines fee handling with the share-supply/redemption model. Do not carry this fee rail into the USDC-in/USDC-out design. |
| Raw Legacy Token account parsing | `contracts/axis-vault/src/jupiter.rs`; `contracts/axis-vault/src/instructions/{deposit,withdraw,deposit_sol,withdraw_sol,sweep_treasury}.rs` | H (`535c0b33…`) | Reads token-account owner from bytes `32..64` and balance from `64..72`, while requiring the legacy SPL Token program ID. | Reject | Axis Core must use Token-2022-aware account parsing/validation and an explicit extension policy before trusting account data or moving tokens. | No Token-2022 code was found in `contracts/axis-vault/src/**`; these offsets and the hard-coded legacy program ID are not a portable Token-2022 validation pattern. Several paths also leave user-account mint/owner checks to downstream CPI rather than asserting them locally. |
| Inconsistent canonical PDA validation on signing paths | `contracts/axis-vault/src/instructions/{deposit,withdraw,deposit_sol,withdraw_sol,sweep_treasury}.rs`; contrast `set_paused.rs` | H (`535c0b33…`) | Asset-moving paths use caller-provided `name` with stored authority/bump to form signer seeds, but do not re-derive and compare the ETF state PDA as administration does. | Reject | Axis Core should centralize a single canonical state/PDA validation routine and invoke it before every PDA signature. | A wrong name normally makes the CPI signature fail, so this is not evidence of a demonstrated theft path. It is nevertheless weaker and less auditable than the explicit re-derivation already used elsewhere in the legacy program. |
| Lazy sidecar PDA that tolerates prefunding | `contracts/axis-vault/src/instructions/rebalance.rs`; `contracts/axis-vault/src/state/rebalance.rs` | H (`535c0b33…`) | For a deterministic, lazily initialized sidecar, the code handles a pre-funded system account by topping up, allocating, assigning under PDA seeds, then validating a discriminator/back-pointer. | Investigate | This is a useful anti-DoS pattern only if Axis Core later introduces optional/lazy PDAs. | Rebalance state, weight timelocks, and turnover windows are not P0 Core blockers. Do not introduce this state merely because the legacy program has it. |
| Controlled adversarial adapter fixture | `contracts/ab-integration-tests/mock-jupiter/src/lib.rs`; `contracts/ab-integration-tests/tests/axis_vault_withdraw_sol_bound.rs` | H (`535c0b33…`) | A deliberately drain-capable mock, loaded at the Jupiter program ID, proves that the parent program rejects an input delta above its allowed bound. | Adapt | Retain this test style for local P0 tests of Axis Core adapter containment and rollback behavior. | It is expressly not a venue integration. It cannot demonstrate Orca Whirlpool, Raydium CPMM, account-extension, pool-state, or production-routing readiness. |
| Fixture-dependent tests that return early | `contracts/ab-integration-tests/src/helpers/svm_setup.rs`; `contracts/ab-integration-tests/tests/axis_vault_{coverage,rebalance,setfee_setcap,withdraw_sol_bound}.rs` | H (`535c0b33…`) | `require_fixture!` prints `SKIP` and returns when a required `.so` is absent; many tests also synthesize raw account blobs at fixed offsets. | Reject | Axis Core test CI should fail when mandatory fixtures/adapters are unavailable and should create/parse real program accounts where possible. | A green local invocation can mean a test was skipped. The repository currently has `mock_jupiter.so`, but no local `axis_vault.so` fixture was present during this inspection. |
| Token-2022 support | No matching implementation in `contracts/axis-vault/src/**` (search at H) | H (`535c0b33…`) | The Axis Vault source depends on `pinocchio-token` and hard-codes the legacy SPL Token program; no Token-2022 instruction, parser, or extension handling was found. | Investigate | Axis Core must make a fresh Token-2022 decision: permitted extensions, transfer-hook behavior, fee handling, close/freeze/delegate policy, and test matrix. | There is no legacy Token-2022 pattern to adopt. Locking the extension policy and the exact Token-2022 validation library/API is a Core design prerequisite. |

## 5. Adopt / Adapt / Reject / Investigate Summary

### Adopt

None. The repository offers no complete pattern that can be reused unchanged in a USDC-in/USDC-out, Token-2022, approved-route system.

### Adapt

- Canonical PDA derivation and PDA-only custody authority.
- Signer/authority checks, program ownership/type checks, and pause controls.
- Bounded fee/configuration changes, with new units and governance semantics.
- Checked arithmetic plus actual pre/post balance-delta assertions.
- Adapter source-loss and destination-gain bounds after CPI.
- Controlled malicious-adapter tests for P0 containment only.

### Reject

- Direct multi-asset basket deposit/withdraw and proportional in-kind redemption.
- Target weights or frontend/backend quotes as NAV/reserve accounting truth.
- Opaque caller-supplied routes as authorization, even when the CPI program ID is pinned.
- Legacy Token raw-byte parsing and hard-coded legacy Token Program assumptions.
- Duplicated internal supply as the accounting source of truth.
- DTF-denominated treasury fees that are later redeemed from reserves.
- Fixture-optional tests as launch evidence.

### Investigate

- Token-2022 extension and account-validation policy.
- Whether any Axis Core P0 account needs lazy creation and therefore pre-funded-PDA handling.
- The separate governance design for config, execution approval, pause, and fee recipients.

## 6. Do-Not-Copy List

The following are explicit non-goals for Axis Core, with legacy evidence where applicable:

- Do not copy the direct basket deposit/withdraw model in `deposit.rs` and `withdraw.rs`.
- Do not make a user's direct deposit of multiple reserve assets the primary mint path.
- Do not reuse the `EtfState` fixed basket layout, its `weights_bps`, or its internal `total_supply` counter as an Axis Core layout.
- Do not use mock balances, test-created raw account blobs, target weights, or quotes as reserve truth.
- Do not let target weights drive DTF mint/redeem accounting; actual validated vault deltas do that.
- Do not use opaque Jupiter route bytes/account lists as an `ApprovedRoute` substitute.
- Do not mix PFDA, auction, ClearCorrection, or Axis-controlled JIT liquidity into P0 mint/redeem. Legacy rebalance/weight-governance sidecars are likewise not P0 blockers.
- Do not treat public DTF/USDC pools as Axis-native liquidity. They are external venues and must not affect Axis reserve/NAV truth.
- Do not port the Legacy Token-only byte offsets into Token-2022 handling.

## 7. Security / Validation Notes

The useful legacy controls are narrowly scoped: it checks privileged signers, program ownership for state, a state discriminator, selected vault addresses, legacy Token Program ownership, CPI program IDs, and many arithmetic operations through `checked_*`. The post-CPI balance checks in `WithdrawSol` and `Rebalance` correctly recognize that a PDA-signed external CPI is a high-risk boundary.

Axis Core should tighten these into one mandatory validation pipeline for every mint, redeem, and adapter call:

1. Re-derive every config/DTF/vault PDA and verify program ownership plus account discriminator/version.
2. Parse every Token-2022 mint/account through a Token-2022-aware validator; verify token program, initialized state, mint, account authority, extension policy, and expected non-aliasing.
3. Validate an on-chain `ApprovedRoute` that binds venue adapter, program ID, pool, direction, USDC mint, reserve mint, and the exact Axis source/destination vaults. Do not delegate this decision to opaque route bytes or a quote service.
4. Snapshot validated USDC and reserve account balances immediately before execution; after execution, derive accounting from the observed deltas and reject outcomes outside the instruction's bound.
5. Keep fee transfers and fee balances in separately specified accounts and exclude them from reserve/NAV accounting. Reconcile DTF mint/burn against actual Token-2022 mint supply rather than a legacy mirror.
6. Ensure the paused/emergency path prevents new unsafe execution and has an explicit, reviewed redemption policy.

Specific legacy weaknesses to avoid are: fixed-offset Legacy Token parsing; no Token-2022 support; inconsistent PDA re-derivation across state-changing/signing paths; an internal supply mirror not reconciled to the token mint; and fixture tests that can silently skip. No issue is reported here as an Axis Core vulnerability—the program is a legacy research source, not the new implementation.

## 8. Open Questions for Muse / ADP / Toby

### Muse

- Which governance roles are distinct for Core configuration, route approval, fee-recipient change, and emergency pause? Is a creator role needed at all after DTF creation?
- During an emergency pause, is redemption blocked, permitted only through already-approved unwind routes, or governed by a separate settlement procedure?
- Are protocol and creator fees taken in USDC before execution, in a separate fee vault after execution, or under another explicitly reconciled model?

### ADP

- What exact fields make an `ApprovedRoute` immutable and sufficient: adapter program, venue program, pool/config account, token mints, direction, canonical Axis vaults, maximum input, minimum output, expiry, and route version?
- How should a route bind Orca Whirlpool and Raydium CPMM fallback semantics without permitting arbitrary CPI account substitution?
- Which adapter outcome deltas are authoritative when a route has temporary/intermediate accounts, and what is the P0 policy for partial fills?

### Toby

- Which Token-2022 extensions are allowed on the DTF mint and USDC/reserve mints? Explicitly address transfer fees, transfer hooks, permanent delegates, freeze authority, confidential transfer, and close authority.
- Which library/API will parse Token-2022 account state and extensions on-chain, and what test vectors will prove rejection of unsupported extensions?
- What is the required production-test matrix for Orca Whirlpool and Raydium CPMM, including real pool state, canonical vault authority, failure modes, and balance-delta assertions?

## 9. Recommended Follow-up Issues

1. Define the Axis Core account graph and PDA namespace: global config, DTF config, DTF mint authority, per-reserve vault authority, reserve vault, USDC vault, fee vaults, and `ApprovedRoute`.
2. Specify and implement a Token-2022 account/mint validation module with a documented allowed-extension matrix and negative tests.
3. Specify `ApprovedRoute` serialization and validation rules that bind an adapter to a venue/pool/mints/direction/canonical vaults; prohibit arbitrary route accounts from becoming authorization.
4. Write mint and redeem accounting invariants entirely in observed USDC/reserve/DTF balance deltas, including rollback behavior, rounding, and reconciliation of actual Token-2022 supply.
5. Design separated creator/protocol fee rails and state which accounts are excluded from reserve and NAV calculations.
6. Implement a minimal pause/emergency-role design and document the redemption behavior while paused.
7. Build controlled malicious-adapter tests that prove source-vault loss and destination-vault gain bounds; make required fixtures fail CI instead of skipping.
8. Add production integration suites for Orca Whirlpool and Raydium CPMM fallback using real/mainnet-forked venue state. Treat these as launch gates, not extensions of mock-adapter coverage.
9. Add property/fuzz tests for PDA substitution, token-account mint/authority substitution, unsupported Token-2022 extensions, duplicate account aliasing, route substitution, fee/reserve separation, and conservation under failed CPIs.
