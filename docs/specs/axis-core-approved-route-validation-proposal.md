# Axis Core ApprovedRoute Validation Proposal (P0-ROUTE-06)

> Status: proposal for review. This document defines the semantic on-chain
> ApprovedRoute model and validation boundary. It does not define account
> serialization, account size, PDA seeds, an ABI, Rust handlers, adapter execution,
> or production venue account encoding.

## 1. Decision requested

Approve the following semantic boundary for later account, ABI, handler, adapter, and
test work:

1. An `ApprovedRoute` is Axis Core-owned policy state created, revised, enabled, and
   disabled only by the configured route authority.
2. Runtime mint and redeem execution must bind the supplied accounts to one current,
   enabled, on-chain `ApprovedRoute` before any venue CPI.
3. The route identity should provisionally use a bounded hybrid of:
   `input_mint + output_mint + direction + venue_adapter + venue_program +
   pool/account_set_hash`.
4. Market binding may be added when policy requires approval to be market-specific,
   but is not part of the default candidate until the tradeoffs in section 6 are
   reviewed.
5. Route-builder output and quotes are advisory. Actual program-observed token balance
   deltas determine execution results, and actual program-controlled reserve balances
   determine reserve accounting.

Approval does **not** finalize the candidate identity, serialization, account size, PDA
derivation, revision encoding, expiry encoding, production account-set encoding,
Token-2022 extension policy, or adapter ABI.

## 2. Scope and non-goals

### 2.1 Scope

This proposal covers:

- ownership, authority, lifecycle, and invalidation of ApprovedRoute policy;
- route identity and granularity alternatives;
- semantic fields needed to bind an execution attempt;
- the exact class and order of checks required before controlled adapter execution;
- controlled-adapter constraints for deterministic local and integration tests;
- post-CPI balance-delta invariants for mint and redeem;
- rejection conditions and table-driven review cases; and
- production venue validation work that remains a mainnet launch gate.

### 2.2 Non-goals

This proposal does not:

- implement Rust handlers, route validation, or CPI execution;
- implement Orca Whirlpool, Raydium CPMM, PumpSwap, or any production integration;
- implement the route-builder service or quote pricing;
- define a generalized routing aggregator, split routing, or arbitrary route graph;
- define or create a public DTF/USDC pool;
- make a controlled adapter evidence of production readiness;
- finalize account serialization, account size, discriminators, ABI, account order, or
  PDA seeds;
- finalize production venue account encoding or account-set canonicalization;
- finalize Token-2022 adoption or extension policy;
- redefine mint/redeem, NAV, fee, fixed-point, rounding, or dust accounting owned by
  their respective proposals; or
- add Auction Program, ClearCorrection, or Axis-controlled JIT liquidity scope.

## 3. Canonical references and dependencies

Canonical requirements:

- `Axis-pizza/Axis_docs/requirements/03-mint-requirements.md`
  (`MINT-008` through `MINT-018`)
- `Axis-pizza/Axis_docs/requirements/04-redeem-requirements.md`
  (`REDEEM-006` through `REDEEM-018`)
- `Axis-pizza/Axis_docs/requirements/05-swap-cpi-execution-requirements.md`
  (`EXEC-001` through `EXEC-020`)
- `Axis-pizza/Axis_docs/requirements/07-execution-policy-risk-controls.md`
  (`POLICY-002`, `POLICY-006`, `POLICY-008`, `POLICY-008b`, `POLICY-010`
  through `POLICY-012`)
- `Axis-pizza/Axis_docs/requirements/09-admin-safety-requirements.md`
  (`ADMIN-001` through `ADMIN-007`)
- `Axis-pizza/Axis_docs/requirements/16-route-builder-backend-api-requirements.md`
  (sections 5, 7, 9 through 11, 15, 21, and 22)
- `Axis-pizza/Axis_docs/requirements/19-axis-core-implementation-rfc.md`
  (Invariants 3 through 8 and 12; P0 and mainnet launch-gate scope; open questions
  3 through 5)

Proposal dependencies available on their review branches:

- PR #57 account model proposal,
  `docs/specs/axis-core-account-model-proposal.md`: defines ApprovedRoute as Axis
  Core-owned enforced policy, identifies `route_registry_authority`, and leaves route
  granularity and validation layout unresolved.
- P0 instruction surface proposal,
  `docs/specs/axis-core-p0-instruction-surface-proposal.md`: defines semantic
  `upsert_approved_route` and `set_approved_route_enabled` administration, signer
  separation, CPI boundaries, and actual-delta invariants.

`docs/context/axis-core-implementation-brief.md` is listed as an input dependency. If it
is unavailable in this branch or referenced proposal branches, no additional decision
is inferred from it.

Dependency rule: if this proposal conflicts with canonical Axis requirements, the
requirements win. If PR #57 or the instruction proposal changes during review, this
proposal must be reconciled before implementation.

## 4. Truth model

| Layer | Role | Authority over protocol accounting |
|---|---|---|
| Route builder, quote source, app, or backend plan | Discovers routes, assembles accounts, proposes minimums, and reports estimates/freshness | None; advisory only |
| On-chain `ApprovedRoute` | Enforced policy describing which bounded execution identity may be attempted | Authoritative for execution permission, not for amounts received |
| Program-observed token balance deltas | Measures what the approved execution actually moved | Execution truth for input spent, output received, fees moved, and DTF supply movement |
| Program-controlled reserve-vault balances | Custodied backing after successful atomic execution | Reserve accounting truth |

Consequences:

- A backend route plan cannot create approval, retarget an approval, or replace the
  on-chain route account.
- A matching ApprovedRoute permits an attempt; it does not attest that a quote, price,
  or output amount is correct.
- A successful quote match cannot compensate for a wrong route or invalid account.
- A quote mismatch is not independently fatal when the route, policy, actual deltas,
  and minimum-output checks all pass.
- Target weights, expected outputs, and route metadata cannot substitute for observed
  reserve balances or observed settlement output.

## 5. ApprovedRoute ownership and authority

| Role | Allowed authority | Explicit non-authority |
|---|---|---|
| Axis Core program | Owns the ApprovedRoute account and enforces its state | Does not trust an externally owned look-alike account |
| Route authority (`route_registry_authority`) | Creates a route boundary, creates a new revision, disables it, and re-enables it after required review | Cannot determine accounting amounts, spend custody, or bypass current policy |
| Admin/config authority (`authority`) | Configures protocol-level authority references under the account model | Does not approve or mutate routes unless it is also explicitly configured as route authority |
| Pause authority | Pauses protocol/market execution through its separate safety controls | Does not silently retarget route identity; direct route-disable power is unresolved |
| Asset policy authority | Changes independent creation/mint/redeem flags and limits | Does not approve venues or route account sets |
| User | Authorizes the exact source spend or DTF burn and supplies minimum output | Cannot approve, revise, enable, or bypass a route |
| Backend/app/quote provider | References current routes and assembles an advisory plan | Has no on-chain route, custody, policy, or accounting authority |
| Venue or adapter | Performs only the bounded invocation selected by Axis Core | Has no route-administration or Axis accounting authority |

Equal public keys configured for multiple roles do not merge their semantic powers. Each
instruction must check the authority required for that instruction.

Route disable must remain an immediate safety action for the route authority. Protocol
or market pause may independently prevent use of all routes. Whether the pause authority
also receives direct disable-only route authority is unresolved; no implementation
should infer it. Re-enable requires current route-authority authorization and
revalidation of the route's current identity and revision.

Material identity changes must create a new identity or revision rather than silently
retargeting an existing approval. Historical executions remain attributable to the
revision that was current when they executed.

## 6. Route identity and granularity comparison

| Model | Candidate identity | Advantages | Risks / limitations |
|---|---|---|---|
| Market-scoped | `market + direction + route` | Strongest per-market isolation; permits market-specific risk review | Duplicates identical routes across markets; route rotation and indexing scale with market count |
| Asset-pair-scoped | `input_mint + output_mint + direction` | Simple, reusable across markets; matches mint/redeem legs | Too broad by itself: does not bind adapter, venue program, pool, or required venue accounts |
| Venue-scoped | `venue_adapter + venue_program` | Simple venue enable/disable boundary | Far too broad for custody safety; approval could unintentionally cover arbitrary pools, mints, or accounts |
| Pool-scoped | `venue_program + pool` | Strong pool identity; useful for direct routes | Does not alone bind direction, mints, token programs, adapter semantics, or auxiliary account set |
| Bounded hybrid | mints + direction + adapter + venue program + pool/account-set commitment | Reusable across markets while binding all execution-critical identities; supports deterministic rejection | Requires adapter-specific account-set canonicalization and revision rules; more administration than coarse scopes |

### 6.1 Recommended candidate

The recommended review candidate is:

```txt
input_mint
+ output_mint
+ direction
+ venue_adapter_program_id
+ venue_program_id
+ pool_or_account_set_hash
```

This is a proposal, not a final implementation decision. It closes the dangerous gaps
in pair-only, venue-only, and pool-only approvals without forcing per-market duplication.

Optional `market` binding should be available when:

- the approved accounts include market-specific custody or settlement identities;
- an asset policy requires market-specific approval; or
- risk review intentionally limits a route to one DTF market.

When market binding is absent, runtime validation must still prove that every reserve,
fee, DTF, user, and settlement account belongs to the supplied market and operation.
Route reuse never weakens market-account validation.

`pool_or_account_set_hash` means a commitment to a canonical, adapter-defined bounded
identity set. It must not mean an unchecked hash supplied by the backend. Exact
canonicalization, ordering, writable flags, and venue-specific fields remain deferred.

## 7. ApprovedRoute fields and semantic model

The following are semantic requirements, not a serialization or layout:

| Semantic field | Required meaning |
|---|---|
| Route identity/hash | Stable commitment to all immutable identity components approved for comparison |
| Route version/revision | Identifies the reviewed policy revision and supports stale-plan/stale-write rejection |
| Enabled/disabled status | Current execution gate; disabled routes reject before CPI |
| Input mint | Exact mint allowed as adapter input |
| Output mint | Exact mint allowed as adapter output |
| Direction | Exact semantic operation direction, at minimum mint composition or redeem unwind |
| Venue adapter program ID | Exact controlled dispatch boundary Axis Core may invoke |
| Venue program ID | Exact downstream venue program the adapter is allowed to target |
| Pool/account-set commitment | Exact pool or bounded adapter-defined venue identity set |
| Allowed token programs | Token program owner allowed for each bound mint/account role; does not settle Token-2022 extension policy |
| Validity/invalidation rule | Optional expiry slot, validity range, explicit invalidation revision, or another review-approved freshness rule |
| Maximum route complexity | Bound on adapter calls/hops/accounts; P0 candidate is one direct route leg per asset with no split |
| Optional market binding | Exact `DTFMarket` when review requires market-scoped approval |
| Created/updated authority | Route authority responsible for the approved revision, for attribution and audit |

Useful administrative metadata such as creation slot, update slot, or predecessor
revision may be considered, but cannot replace the identity and freshness checks above.

The following are deliberately not decided here:

- serialized field order, widths, enum representations, reserved bytes, and account
  size;
- whether identity is stored as components, a hash, or both;
- PDA seed components or whether revisions use separate accounts;
- exact expiry representation;
- exact token-program and Token-2022 extension encoding; and
- exact Orca, Raydium, or future PumpSwap account-set representation.

## 8. Pre-CPI validation sequence

Axis Core must complete these checks in order before controlled adapter or venue
execution. An earlier failure prevents later CPI and leaves all token balances, supply,
fee counters, and program state unchanged.

1. **Protocol and market state.** Validate Axis Core ownership and identity of
   `ProtocolConfig` and `DTFMarket`; reject global pause; validate that the supplied
   market and DTF mint/reserve namespace match; require mint-active state for mint or
   an explicitly safe active/exit-only state for redeem.
2. **Asset flags and policy.** Validate every affected asset config and immutable mint
   binding; require `mint_enabled` or `redeem_enabled` as applicable; enforce net
   allocation/trade, price-impact, pricing, direct-route, and complexity policy owned
   by the instruction/policy proposals.
3. **Route account owner, derivation, and status.** Require an Axis Core-owned route
   account at the approved identity/PDA candidate, reject a missing or substituted
   account, and require `enabled`.
4. **Route version/revision freshness.** Match the instruction's expected revision to
   current state and enforce expiry, validity range, and invalidation rules. A fresh
   quote cannot revive a stale route.
5. **Route direction.** Require mint composition for `USDC -> reserve asset` or redeem
   unwind for `reserve asset -> USDC`; reject reverse or unsupported direction.
6. **Input/output mints.** Match instruction accounts, token accounts, and route fields
   to the exact expected input/output mints. Also validate the market DTF mint
   independently.
7. **Token program ownership.** Validate each mint and token account owner against its
   bound allowed token program and current asset policy. Reject mixed or substituted
   token programs. Token-2022 extension acceptance remains a separate launch decision.
8. **Adapter program ID.** Match the executable adapter program to the route's exact
   adapter ID. The adapter must be an approved controlled-test or production profile
   for the current cluster and policy.
9. **Venue program ID.** Match the adapter's permitted downstream target to the exact
   executable venue program in the route; reject arbitrary program invocation.
10. **Expected pool/account set.** Canonicalize the supplied adapter-specific identities
    under the approved profile and match the route commitment. Reject missing, extra,
    duplicated, reordered-when-significant, wrong-pool, wrong-vault, or wrong auxiliary
    identities.
11. **Signer and authority boundaries.** Require the user authority for the user source
    spend/DTF burn; validate Axis PDA authorities for reserve, settlement, fee, and DTF
    mint roles. Route/admin/backend/venue keys are not execution spend authorities, and
    the route authority need not sign ordinary mint/redeem execution.
12. **Writable account restrictions.** Permit writes only to operation-required user,
    reserve, fee, settlement, DTF mint/supply, and exact approved venue roles. Config,
    policy, pricing, and route accounts are read-only during execution. Reject duplicate
    incompatible roles, unexpected writable accounts, and any fee-vault/reserve-vault
    alias before CPI.
13. **Minimum output.** Require nonzero `min_out_i` for every mint leg and nonzero
    aggregate `min_usdc_out` for redeem, except a separately isolated test-only rule if
    ever approved. Minimums remain user instruction constraints, not route approval.
14. **Advisory quote/backend data.** Parse only data required by the approved ABI and
    bounds. Do not use estimated output, route availability, quote source, or plan ID as
    approval or accounting truth. Enforce any instruction-level quote freshness policy
    separately from ApprovedRoute freshness.
15. **Pre-execution snapshots.** Snapshot all user source/destination, fee custody,
    settlement, reserve vault, and DTF supply balances needed for post-CPI checks. Only
    after all preceding checks and snapshots pass may Axis Core dispatch the adapter.

Checks that are independent of CPI should fail before CPI. Post-CPI validation remains
mandatory even when every pre-CPI identity check succeeds.

## 9. Controlled adapter profile

A controlled adapter used in local or integration tests must:

- accept only the exact input and output mints bound by the ApprovedRoute;
- accept only a fixed, bounded venue/account set committed by the route;
- invoke no arbitrary program and expose no caller-selected downstream program;
- execute no split, graph, arbitrary multi-hop, or unbounded remaining-account route;
- have no authority to determine reserve values, DTF output, fee accrual, or user
  settlement accounting;
- move real test token balances so Axis Core can observe pre/post deltas;
- be subject to the same minimum-output, alias, signer, writable-account, atomicity, and
  post-CPI delta checks as a production adapter; and
- be clearly identified as a test profile that cannot be enabled as a production
  venue merely because its tests pass.

A controlled adapter proves the Axis accounting and rejection boundary in a
deterministic environment. It does **not** prove production venue account validation,
liquidity behavior, compute/account limits, mainnet compatibility, or launch readiness.

## 10. Production venue requirements

Production venue support is deferred launch-gate work. Mainnet readiness requires:

- Orca Whirlpool validation as the first production venue candidate;
- Raydium CPMM validation as the fallback production venue candidate;
- PumpSwap only later if separately reviewed and approved;
- mainnet-fork or cloned-account validation for each approved production path;
- preference for direct `USDC <-> asset` routes;
- explicit approval and rejection tests for venue program, pool, vault, oracle,
  observation/tick, authority, and other venue accounts;
- adapter-specific canonical account constraints and writable/signer restrictions;
- real token movement plus post-CPI actual-delta and minimum-output checks;
- compute, account-count, transaction-size, and failure/revert measurements; and
- documented venue-specific risks.

At least Orca Whirlpool and Raydium CPMM paths must satisfy the production requirements
before mainnet launch. A quote SDK, account assembler, controlled adapter, or successful
public pool trade is insufficient.

This work concerns underlying mint/redeem execution. It does not create or approve a
public DTF/USDC pool or establish Axis-native secondary-liquidity/LVR protection.

## 11. Mint post-CPI actual delta checks

Let:

- `G` be the authorized gross user USDC input;
- `F` be the canonical mint fee computed from `G`;
- `N = G - F` be net USDC available for composition;
- `R_i` be the observed positive increase in reserve vault `i`;
- `M` be the later-approved DTF output derived from actual added reserve value.

After CPI and before commit, Axis Core must observe and enforce:

1. The user USDC source decreases by exactly `G`; an allowance or quote is not proof of
   actual intake.
2. Separate USDC fee custody increases by exactly `F`, with creator/protocol accrual
   consistent with the canonical fee proposal.
3. Only `N` is available to composition execution. `F` is not adapter input, reserve
   value, or refundable execution budget.
4. Every expected reserve vault has `R_i = post_i - pre_i > 0` and
   `R_i >= min_out_i`; no unapproved reserve or wrong-mint account receives backing.
5. Fee custody does not alias any reserve vault, settlement account, or user
   destination in a way that double-counts movement.
6. Actual added reserve value is computed later under the approved pricing,
   decimals, fixed-point, and rounding rules from observed `R_i`, never from `G`, `N`,
   or quoted output.
7. DTF supply increase and user DTF destination increase both equal `M`; `M` is derived
   from actual added reserve value under the later-approved pre-trade NAV/supply
   formula.
8. Any unexpected direction, unaccounted token movement, supply/destination mismatch,
   fee mismatch, or incomplete leg rejects the entire mint atomically.

A quoted output may differ from `R_i`. The mint may still succeed only when every route,
policy, pricing, actual-delta, fee, and minimum-output check passes.

## 12. Redeem post-CPI actual delta checks

Let:

- `D` be the authorized DTF amount in;
- `P_i` be the maximum pro-rata reserve input computed from pre-redeem supply and
  reserve balance under later-approved rounding rules;
- `S_i` be the observed reserve vault decrease used by execution, where
  `0 <= S_i <= P_i`;
- `A` be actual observed USDC output.

After CPI and before commit, Axis Core must observe and enforce:

1. The user DTF source decreases by exactly `D`.
2. DTF total supply decreases by exactly `D`; burn/source movements must agree.
3. Each reserve vault input decreases only in the expected direction and by no more
   than its computed `P_i`. The executed route must account for every observed `S_i`.
4. The validated USDC settlement account increases by actual venue output before
   payout, and the user USDC destination increases by exactly the resulting `A`.
   Implementations that combine settlement and destination must preserve an equivalent
   non-double-counted observation.
5. `A >= min_usdc_out`; a quoted amount cannot satisfy this check.
6. Fee-vault balance, `accrued_creator_fee_usdc`, and
   `accrued_protocol_fee_usdc` remain unchanged because launch
   `redeem_fee_bps = 0`.
7. No fee vault is used as reserve input or settlement output, and no reserve decrease
   exceeds the user's pro-rata entitlement.
8. Any unexpected movement, missing output, supply mismatch, pro-rata breach, fee
   mutation, or settlement/destination mismatch rejects the entire redeem atomically.

A quoted output may differ from `A`. The redeem may still succeed only when every route,
policy, actual-delta, pro-rata, and minimum-output check passes.

## 13. Rejection conditions

Axis Core must reject:

- a wrong or substituted `DTFMarket`, including a failed optional route-market binding;
- wrong input mint, output mint, DTF mint, or reserve mint;
- wrong or reversed execution direction;
- wrong adapter program ID or an adapter not permitted in the current environment;
- wrong venue program ID or arbitrary downstream invocation;
- wrong pool, venue vault, auxiliary identity, or bounded account set;
- wrong token program, mint owner, token-account owner, or unapproved token policy;
- missing, inactive, disabled, expired, invalidated, or stale route/revision;
- an unauthorized route create/update/disable/re-enable authority;
- a backend route without a matching current on-chain ApprovedRoute;
- a quote mismatch that causes actual minimum-output, price, policy, or delta checks to
  fail;
- fee-vault/reserve-vault aliasing or another incompatible duplicate account role;
- a missing/zero required `min_out` or `min_usdc_out`;
- an unexpected writable or signer account;
- unsupported route complexity, split routing, arbitrary graph, or unapproved hop; and
- any actual balance-delta, DTF supply, fee, settlement, reserve, or pro-rata mismatch.

All failures are atomic. No failed route attempt may leave partial reserve movement,
fee accrual, DTF mint/burn, or user settlement.

## 14. Table-driven validation cases

### 14.1 Successful balance-delta cases

`Δ` is post-balance minus pre-balance. Exact valuation, fixed-point, and rounding
formulas remain owned by later accounting review.

Each row isolates one approved route leg inside an otherwise-valid atomic mint or
redeem. A multi-asset market applies the same per-leg reserve checks to every expected
asset; accounts outside the market/operation remain unchanged.

| Case | Expected USDC deltas | Expected reserve-asset deltas | Expected DTF deltas | Result |
|---|---|---|---|---|
| One mint route: `USDC -> asset_i` | User source `Δ=-G`; fee custody `Δ=+F`; only `N=G-F` enters composition | Bound reserve `i`: `Δ=+R_i`, `R_i>0`, `R_i>=min_out_i`; accounts outside the operation `Δ=0` | User destination `Δ=+M`; total supply `Δ=+M`, where `M` derives from all actual reserve deltas under the later-approved formula | Success if all policy, price, route, and delta checks pass |
| One redeem route: `asset_i -> USDC` | Validated settlement observes aggregate venue output `A`; user destination `Δ=+A`; fee custody `Δ=0`; `A>=min_usdc_out` | Bound reserve `i`: `Δ=-S_i`, `0<=S_i<=P_i`; accounts outside the operation `Δ=0` | User source `Δ=-D`; total supply `Δ=-D` | Success if all route, pro-rata, policy, and delta checks pass |

### 14.2 Validation and failure cases

For each rejected case, expected final deltas for user tokens, fee custody, reserves,
settlement, DTF supply, and fee counters are all zero because the transaction reverts.

| Case | Test setup | Expected decision | CPI allowed? |
|---|---|---|---:|
| Wrong market | Supply a substituted market, market-owned vault, DTF mint, or fail an explicit route-market binding | Reject `STATE`/`ROUTE` | No |
| Wrong mint | Change input, output, reserve, USDC, or DTF mint | Reject `TOKEN`/`ROUTE` | No |
| Wrong direction | Use redeem route for mint or reverse the pair | Reject `ROUTE` | No |
| Wrong adapter/program | Change adapter ID or venue program ID | Reject `ROUTE`/`OWNER` | No |
| Wrong account set | Change pool/vault/auxiliary account, omit one, or add an unapproved identity | Reject `ROUTE` | No |
| Inactive route | Set `enabled=false` | Reject `ROUTE`/`STATE` | No |
| Expired route | Exceed route validity or use invalidated revision | Reject `ROUTE`/`STATE` | No |
| Unauthorized route authority | Sign administration with user, backend, admin-only, or wrong route key | Reject `AUTH` | Not applicable |
| Unapproved route | Omit route or provide a backend-only/external route | Reject `ROUTE` | No |
| Backend quote mismatch, safe actual | Quote differs, but route/policy pass and actual output meets minimum | Accept; account only from actual deltas | Yes |
| Backend quote mismatch, unsafe actual | Quote differs and actual output violates a minimum or policy | Reject `SLIPPAGE`/`POLICY`/`DELTA` | CPI may run; transaction reverts |
| Insufficient actual output | Output delta is nonpositive, incomplete, or below required execution result | Reject `DELTA`/`SLIPPAGE` | CPI may run; transaction reverts |
| Minimum-output failure | Actual mint leg `< min_out_i` or redeem output `< min_usdc_out` | Reject `SLIPPAGE` | CPI may run; transaction reverts |
| Actual delta mismatch | Wrong direction/amount, DTF source/supply mismatch, unexpected account movement, or settlement mismatch | Reject `DELTA` | CPI may run; transaction reverts |
| Fee vault/reserve vault alias | Supply the same account for fee and reserve roles | Reject `FEE`/`OWNER` | No |
| Success: one mint route | Use current enabled route and deltas from section 14.1 | Accept | Yes |
| Success: one redeem route | Use current enabled route and deltas from section 14.1 | Accept | Yes |

## 15. Unresolved review questions

1. Is the bounded hybrid identity approved, and when is market binding mandatory?
2. Which identity changes create a new route account versus a new revision?
3. What canonical ordering and fields define each adapter's
   `pool_or_account_set_hash`, including writable flags and optional accounts?
4. Is freshness represented by expiry slot, validity interval, an invalidation epoch,
   revision comparison, or a reviewed combination?
5. May the pause authority directly disable routes, or only stop protocol/market use
   while the route authority owns route state?
6. Must all P0 routes be one direct adapter call, or is any explicitly bounded
   multi-hop profile needed before production?
7. Which token programs are accepted per role, and what Token-2022 extensions are
   rejected?
8. Must complete route readiness exist at market creation, activation, or both?
9. What controlled-adapter identity and ABI prove P0 accounting without resembling a
   production approval?
10. What exact Orca Whirlpool and Raydium CPMM identities and constraints enter their
    production account sets?
11. How are route authority rotation, re-enable, and material route revision protected
    by multisig, timelock, or dual authorization?
12. What event schema records route revision, disable/re-enable, and execution identity
    without making off-chain indexing protocol truth?

Implementation must not infer permissive defaults for unresolved questions.
