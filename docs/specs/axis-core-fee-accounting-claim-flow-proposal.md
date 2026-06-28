# Axis Core Fee Accounting and Claim Flow Proposal

## Status

- Issue: P0-FEE-07 / #8
- Status: proposal for review
- Scope: documentation only
- Preferred direction: per-market USDC fee custody plus market-level creator and protocol accrual counters

This document defines the semantic accounting and authorization model for Axis v1 fees. It does not finalize an account layout, instruction ABI, PDA seed, serialization format, handler sequence, token-program integration, or account size.

PR #57, PR #58, and PR #59 are open dependencies. Names used here describe logical roles only. This proposal must be reconciled with their approved account and instruction boundaries before implementation.

## 1. Goals

This proposal must ensure that:

- creator and protocol mint fees have separate, on-chain entitlements;
- fee custody can never be treated as DTF reserve custody or NAV backing;
- only the authorized creator or protocol recipient can claim its entitlement;
- a claim cannot withdraw reserve assets;
- mint fee calculation and allocation conserve the user's gross USDC input;
- successful claims cannot be replayed to withdraw the same accrued amount twice;
- failed mints and failed claims leave fee accounting unchanged; and
- backend, indexer, app, quote, or database balances are never protocol truth.

## 2. Canonical v1 requirements

The following decisions are inputs to this proposal, not commercial-policy changes:

- v1 charges fees on the USDC side of mint;
- `redeem_fee_bps = 0`;
- creator and protocol fees accrue only on successful mint;
- fees are claim-based;
- fees are not immediately transferred to final recipients during mint;
- creators cannot customize fee bps per market;
- each market snapshots its protocol-derived fee configuration at creation;
- the market fee configuration is immutable after creation;
- fee custody is separate from reserve custody;
- fee balances and fee accruals are excluded from reserve backing and NAV; and
- backend, indexer, app, database, quote, and UI balances are never protocol truth.

The canonical launch configuration is referenced as:

```text
mint_fee_bps = 100
redeem_fee_bps = 0
creator_share_bps = 4000
protocol_share_bps = 6000
max_mint_fee_bps = 300
max_redeem_fee_bps = 0
```

These values come from the canonical fee requirements. This proposal does not redefine them or authorize market creators to override them.

## 3. Terminology and units

- **Market**: the logical DTF market whose mints generated the fees.
- **Creator fee owner**: the market's immutable creator fee recipient role, which beneficially owns the creator accrual.
- **Protocol fee owner**: the protocol treasury role, which beneficially owns the protocol accrual.
- **Recipient authority**: the signer authorized to claim for a fee owner.
- **Recipient token account**: the validated USDC account that receives a successful claim.
- **FeeVault**: the preferred per-market USDC custody boundary for unclaimed fees.
- **Accrued fee**: an outstanding, unclaimed USDC liability recorded in Axis Core-owned state.
- **Reserve vault**: program-controlled custody of assets that back DTF supply.
- **Claim bound**: a positive caller-provided upper limit on one claim.

All amounts in protocol calculations are non-negative integers denominated in the configured USDC mint's smallest unit. Human-readable USDC values in examples assume six decimals. Basis points use a denominator of `10_000`.

Terms such as `FeeVault`, `accrued_creator_fee_usdc`, and `accrued_protocol_fee_usdc` are semantic names. Their final field placement and representation remain unresolved.

## 4. Ownership and recipient model

### 4.1 Fee ownership

Fee ownership is an on-chain entitlement, not possession of the custody account.

- The creator fee owner owns only the market's creator accrued counter.
- The protocol fee owner owns only the market's protocol accrued counter.
- Neither owner owns or may claim the other counter.
- Neither owner owns any reserve asset by virtue of its fee entitlement.
- Axis Core controls FeeVault custody until an authorized claim succeeds.
- An unallocated FeeVault surplus is not owned through either accrued counter.

The two outstanding counters are the authoritative liabilities:

```text
creator_liability = accrued_creator_fee_usdc
protocol_liability = accrued_protocol_fee_usdc
total_fee_liability = creator_liability + protocol_liability
```

Possession of a backend credential, app session, indexer record, market metadata record, or database balance grants no claim right.

### 4.2 Creator fee recipient

At market creation, the market must bind:

- an immutable market creator identity; and
- an immutable creator fee recipient authority for v1.

The creator and creator fee recipient authority may be the same public key, but they are distinct semantic roles. A creator cannot change fee rates or fee shares by selecting a recipient.

Preferred v1 authorization:

1. the creator fee recipient authority must sign the claim;
2. the signer must equal the authority bound to the market;
3. the destination must be a valid USDC token account for that authority; and
4. the claim may debit only that market's creator accrued counter and FeeVault.

The final account model may store an authority, an exact token destination, or both. Regardless of representation, implementation must preserve an immutable recipient identity, require explicit creator authorization, and prevent redirection to an arbitrary claimant-controlled account. Recipient rotation or recovery is not part of v1 unless a later governance proposal defines it.

### 4.3 Protocol treasury and protocol fee recipient

The protocol fee owner is the protocol treasury role, not the market creator and not an arbitrary protocol administrator.

Preferred v1 authorization:

1. the protocol treasury recipient authority must sign the claim;
2. the signer must match the authorized on-chain protocol treasury role;
3. the destination must be the configured protocol USDC recipient or a validated USDC token account owned by that recipient authority; and
4. the claim may debit only the selected market's protocol accrued counter and FeeVault.

Protocol authority, pause authority, or another administrative signer does not gain claim authority merely by holding an administrative role. Whether the protocol recipient is snapshotted per market or resolved from current protocol configuration is an implementation question; either choice must define rotation effects on already-accrued fees before implementation.

### 4.4 No delegated or permissionless claim in v1

This proposal does not authorize a relayer, keeper, creator delegate, protocol admin, or permissionless caller to initiate a claim. Such a model could be safe if the destination were completely fixed, but it changes authorization and operational semantics and therefore requires a later explicit decision.

## 5. Preferred custody and accrual state

### 5.1 Per-market USDC FeeVault

Each market should have one logically unique FeeVault that:

- accepts only the configured USDC mint;
- is controlled by Axis Core;
- is bound to exactly one market;
- cannot alias a reserve vault, mint execution account, recipient account, or another market's FeeVault;
- receives the total mint fee only as part of a successful mint transaction; and
- can be debited only by a valid creator or protocol fee claim, or by a separately specified recovery flow.

This proposal prefers per-market custody because it limits cross-market accounting errors and makes each market's liabilities directly reconcilable against one custody balance.

### 5.2 Market-level accrual counters

The market must expose two logical outstanding counters:

```text
accrued_creator_fee_usdc
accrued_protocol_fee_usdc
```

The counters:

- are denominated in USDC smallest units;
- represent outstanding, unclaimed amounts;
- increase only when a mint succeeds;
- decrease only when the corresponding claim succeeds;
- never increase during redeem;
- never become negative or wrap;
- are protocol truth only when read from validated Axis Core-owned state; and
- are not derived from FeeVault balance, an event stream, or an off-chain ledger.

Optional lifetime accrued and lifetime claimed totals may improve auditability, but they are not required for claim derivation and do not replace the outstanding counters.

### 5.3 Reconciliation invariant

For a market with FeeVault balance `V`, creator liability `C`, and protocol liability `P`:

```text
L = C + P
V >= L
surplus = V - L
```

`C + P` must use checked arithmetic. `V < L` is a custody deficit and invalid accounting condition.

An unsolicited transfer can make `V > L`. This surplus:

- does not increase either claimable amount;
- is not reserve backing;
- is excluded from NAV;
- cannot be claimed through creator or protocol fee claims; and
- remains quarantined pending a separately reviewed recovery policy.

Claims must not use `min(counter, vault_balance)` to hide a deficit. A deficit must fail the claim without changing either recipient's entitlement.

## 6. Fee calculation and rounding

For gross user USDC input `G`, market mint fee bps `M`, creator share bps `S_c`, and basis-point denominator `B = 10_000`:

```text
mint_fee_usdc = floor(G * M / B)
net_usdc_for_composition = G - mint_fee_usdc

creator_fee_usdc = floor(mint_fee_usdc * S_c / B)
protocol_fee_usdc = mint_fee_usdc - creator_fee_usdc
```

The market configuration must satisfy:

```text
M <= max_mint_fee_bps
redeem_fee_bps = 0
creator_share_bps + protocol_share_bps = 10_000
```

All division rounds down toward zero. Inputs are non-negative, so this is floor division.

The protocol share is calculated as the residual instead of a second independent floor. Therefore:

```text
creator_fee_usdc + protocol_fee_usdc = mint_fee_usdc
mint_fee_usdc + net_usdc_for_composition = G
```

This direction never rounds the total mint fee above the exact rational fee and therefore avoids overcharging the user. Any split remainder is assigned to the protocol side, preserving exact conservation of the already-collected fee.

Implementation must use checked multiplication or a sufficiently wide intermediate representation. The integer widths are intentionally not selected here.

## 7. Mint-flow conservation

### 7.1 Successful mint

The USDC-side conservation model is:

```text
G = F + N
F = C_delta + P_delta
```

where:

- `G` is gross user USDC input;
- `F` is the mint fee transferred into the market FeeVault;
- `N` is net USDC available for reserve composition;
- `C_delta` is the creator counter increase; and
- `P_delta` is the protocol counter increase.

The mint must establish the following atomic post-state:

```text
FeeVault_post - FeeVault_pre = F
creator_counter_post - creator_counter_pre = C_delta
protocol_counter_post - protocol_counter_pre = P_delta
C_delta + P_delta = F
```

Only `N` may enter route execution for reserve composition. The value that backs newly minted DTF is not `G` or `N`; it is the actual reserve value added after execution, derived from validated reserve balance deltas and approved pricing under the separate mint/NAV specification.

Thus:

```text
minted_dtf is based on actual_added_value_usdc
actual_added_value_usdc excludes F
```

Venue spread, slippage, price impact, and quote differences affect execution results but are not creator or protocol fees.

### 7.2 Atomicity

Fee custody and counter increases are provisional until the complete mint succeeds. If any fee calculation, transfer, CPI, route, pricing, minimum-output, reserve-delta, DTF mint, or postcondition check fails, transaction rollback must restore:

- the user's USDC balance;
- the FeeVault balance;
- both accrued counters;
- reserve balances; and
- DTF supply and user DTF balance.

A failed mint never accrues fees, even if fee calculation or an earlier token movement executed before the failure within the transaction.

### 7.3 Redeem

For v1:

```text
redeem_fee_bps = 0
redeem_fee_usdc = 0
user_usdc_out = actual_usdc_received
```

Redeem:

- does not increase either fee counter;
- does not deposit a redeem fee into FeeVault;
- does not use FeeVault as an output or reserve source;
- does not decrease either accrued counter; and
- does not classify execution spread, slippage, or price impact as an Axis fee.

## 8. Reserve and NAV exclusion

Fee and reserve accounting are disjoint domains.

Required rules:

- a FeeVault can never be registered or passed as a market reserve vault;
- a reserve vault can never be passed as fee custody or a claim source;
- fee claim logic has no authority to debit a reserve vault;
- FeeVault balance is excluded from every reserve-value sum;
- creator and protocol accrued counters are excluded from reserve value and NAV;
- FeeVault surplus is also excluded from reserve value and NAV;
- fee amounts are excluded from `actual_added_value_usdc`;
- DTF issuance cannot be based on gross USDC input or fee custody; and
- claims cannot change reserve balances, reserve ownership, DTF supply, or NAV.

For a reserve set `R` and separate FeeVault balance `V`:

```text
reserve_value_usdc = value(actual balances in R)
NAV inputs do not include V, C, P, or FeeVault surplus
```

The program must validate these boundaries from on-chain account ownership, mint, market binding, and custody identity. Account labels supplied by a client are not sufficient.

## 9. Claimable amount and bounded claims

### 9.1 Claimable amount derivation

For role `r`:

```text
creator_claimable = accrued_creator_fee_usdc
protocol_claimable = accrued_protocol_fee_usdc
```

The claimable amount comes only from the validated on-chain counter for the selected market and role. It is not:

- the FeeVault balance;
- a percentage recomputed from historical mint events;
- a backend or indexer aggregate;
- a recipient token-account balance; or
- the sum of parsed transaction logs.

### 9.2 Bounded claim semantics

At the semantic level, a claim supplies a positive `max_claim_amount`. The final ABI encoding and instruction names are deferred.

For the authorized role:

```text
max_claim_amount > 0
claim_amount = min(max_claim_amount, role_claimable)
```

This makes the caller's amount an upper bound. It prevents a claim from withdrawing newly accrued fees above the amount the signer reviewed while allowing the transaction to succeed if another valid claim reduced the outstanding counter before execution.

Before transfer, the program must also require:

```text
FeeVault_balance >= creator_claimable + protocol_claimable
FeeVault_balance >= claim_amount
```

The first check protects both owners from a custody deficit. A claimant cannot drain the remaining collateral merely because the vault can cover that claimant's individual amount.

On creator success:

```text
accrued_creator_fee_usdc_post =
    accrued_creator_fee_usdc_pre - claim_amount
```

On protocol success:

```text
accrued_protocol_fee_usdc_post =
    accrued_protocol_fee_usdc_pre - claim_amount
```

Exactly `claim_amount` is transferred from FeeVault to the validated recipient token account. The other role's counter is unchanged.

For total liability `L = C + P` and claimed amount `A`:

```text
FeeVault_post = FeeVault_pre - A
selected_counter_post = selected_counter_pre - A
total_liability_post = total_liability_pre - A
recipient_balance_post = recipient_balance_pre + A
surplus_post = surplus_pre
```

This conserves custody plus payout and cannot change the unallocated surplus.

### 9.3 Zero-claim behavior

If the selected role's accrued counter is zero, the claim fails with a specific no-fees-claimable class of error. It must not:

- transfer zero tokens;
- change either counter;
- change FeeVault or reserve balances; or
- emit a successful claim event.

A zero `max_claim_amount` is invalid and fails without state changes.

This rejection policy makes accidental repeated submissions observable while preserving economic idempotency.

## 10. Claim authorization and validation

Validation must establish all of the following before a claim can succeed:

1. the market state is Axis Core-owned and valid;
2. the FeeVault is the fee custody bound to that market;
3. the FeeVault is program-controlled;
4. the FeeVault mint equals the configured USDC mint;
5. the FeeVault is not any reserve vault or other forbidden custody account;
6. the selected role is creator or protocol;
7. the selected role's required recipient authority signed;
8. the signer matches the on-chain authority for that role;
9. the destination is a valid USDC token account for the authorized recipient;
10. `max_claim_amount` is positive;
11. the role counter is non-zero;
12. all counter and liability arithmetic is checked;
13. FeeVault covers total creator and protocol liabilities;
14. the resulting `claim_amount` is within the role counter and caller bound; and
15. the transfer and counter update complete atomically.

Creator authorization never permits a protocol claim. Protocol authorization never permits a creator claim. Administrative authority, market creation authority, pause authority, backend signer, and fee payer are not substitutes for the required recipient signer.

Whether fee claims remain available while the protocol or market is paused must be decided explicitly. Pausing must never implicitly redirect or erase an accrued entitlement.

### 10.1 Logical claim transition

Without prescribing handler order, serialization, or ABI, a claim has the following semantic stages:

1. validate market, FeeVault, USDC mint, role, signer, and recipient bindings;
2. read the selected on-chain counter and positive caller bound;
3. reconcile FeeVault against total creator and protocol liabilities;
4. derive the bounded claim amount;
5. atomically transfer that exact amount and reduce only the selected counter; and
6. verify the post-state and emit the successful claim audit fields.

Any failure in any stage rolls back the entire transition. A future implementation may reorder internal operations only if the same preconditions, atomic post-state, and failure behavior are preserved.

## 11. Replay, idempotency, and concurrency

Solana runtime transaction replay protection is necessary but not the accounting defense. The on-chain outstanding counter provides the economic replay defense.

- Every successful claim decreases exactly one counter by exactly the transferred amount.
- A transaction cannot successfully transfer without the matching counter decrease.
- Repeating a full claim after the counter reaches zero fails with no state changes.
- Repeating a bounded partial claim while entitlement remains is a new valid withdrawal of the remaining entitlement, not a double claim of the first amount.
- Two concurrent claims serialize against the same writable market fee state and FeeVault; each observes the state committed before it executes.
- A stale claim can never withdraw more than its signed `max_claim_amount`.
- No separate off-chain nonce, claimed-event set, or indexer cursor is required for correctness.

The final implementation may add an on-chain sequence number for easier audit correlation, but a sequence number does not replace counter-based accounting.

## 12. Failed claim behavior

Claims are all-or-nothing. Any failure, including unauthorized signer, wrong market, wrong vault, wrong mint, wrong recipient, zero entitlement, invalid bound, overflow, custody deficit, token-program failure, or post-transfer mismatch, must leave:

- creator accrued fees unchanged;
- protocol accrued fees unchanged;
- FeeVault unchanged;
- recipient balances unchanged;
- all reserve balances unchanged;
- DTF supply unchanged; and
- NAV inputs unchanged.

The implementation must not decrement a counter permanently before a transfer that can fail. Transaction atomicity must cover both operations regardless of internal handler order.

Claims must not partially pay an amount merely because FeeVault is underfunded. Deficit recovery and reconciliation are separate incident procedures, not normal claim behavior.

## 13. Event and audit fields

Events and logs support audit and indexing, but validated accounts and token balances remain protocol truth.

### 13.1 Fee accrual event

A successful mint fee accrual should expose:

- event or schema version;
- market identifier;
- user or mint initiator;
- configured USDC mint;
- gross user USDC input;
- market `mint_fee_bps`;
- market `creator_share_bps`;
- market `protocol_share_bps`;
- total mint fee;
- creator fee delta;
- protocol fee delta;
- net USDC for composition;
- post-accrual creator counter;
- post-accrual protocol counter;
- pre- and post-FeeVault balances, or the observed FeeVault delta;
- transaction/instruction correlation data available to the runtime; and
- optional market fee-accounting sequence number.

This event must be emitted only if the complete mint succeeds.

### 13.2 Fee claim event

A successful claim should expose:

- event or schema version;
- market identifier;
- fee role: creator or protocol;
- configured USDC mint;
- authorized claimant;
- recipient token account;
- caller-provided claim bound;
- actual claimed amount;
- selected counter before and after the claim;
- other-role counter after the claim;
- FeeVault balance before and after the claim;
- transaction/instruction correlation data available to the runtime; and
- optional market fee-accounting sequence number.

No successful claim event is emitted for a rejected zero claim or any failed transaction. Failure logs may aid diagnosis but are not a durable accounting ledger.

### 13.3 Indexer reconciliation

An indexer may calculate expected identities such as:

```text
lifetime_accruals - lifetime_claims = outstanding counters
FeeVault balance - outstanding counters = unallocated surplus
```

Differences should trigger an alert. The indexer must not repair, override, or replace the on-chain counters.

## 14. Worked accounting cases

Unless noted otherwise, examples use the referenced launch configuration and assume unrelated market validations pass.

### Case 1: zero fee

This case distinguishes a configured zero fee from a non-zero configured fee that rounds down to zero.

#### Case 1A: configured zero fee

This is the semantic behavior if a protocol-approved market fee snapshot has `mint_fee_bps = 0`. It is not the referenced v1 launch value and does not permit creator customization.

Use `G = 1,000 USDC` and `M = 0`:

```text
F = floor(1,000 * 0 / 10_000) = 0 USDC
C_delta = 0 USDC
P_delta = 0 USDC
N = 1,000 - 0 = 1,000 USDC
```

Fee-state reconciliation, assuming zero starting balances:

```text
before: FeeVault = 0, creator accrued = 0, protocol accrued = 0
after:  FeeVault = 0, creator accrued = 0, protocol accrued = 0
check:  gross input 1,000 = fee custody 0 + composition input 1,000
```

#### Case 1B: non-zero configured fee rounds down to zero

Use `G = 99` USDC smallest units with a 1% mint fee:

```text
F = floor(99 * 100 / 10_000) = 0
C_delta = floor(0 * 4000 / 10_000) = 0
P_delta = 0 - 0 = 0
N = 99 - 0 = 99
```

FeeVault and both counters remain unchanged. If a separate minimum-mint or minimum-allocation rule rejects such a small mint, the full transaction fails and the same zero fee-state change results.

Fee-state reconciliation, assuming zero starting balances:

```text
before: FeeVault = 0, creator accrued = 0, protocol accrued = 0
after:  FeeVault = 0, creator accrued = 0, protocol accrued = 0
```

### Case 2: 1,000 USDC mint with 1% fee and 40/60 split

Using six-decimal units:

```text
G = 1,000,000,000 units = 1,000 USDC
F = floor(1,000,000,000 * 100 / 10_000)
  = 10,000,000 units = 10 USDC
C_delta = floor(10,000,000 * 4000 / 10_000)
        = 4,000,000 units = 4 USDC
P_delta = 10,000,000 - 4,000,000
        = 6,000,000 units = 6 USDC
N = 990,000,000 units = 990 USDC
```

After a successful mint, FeeVault increases by 10 USDC, creator accrual increases by 4 USDC, and protocol accrual increases by 6 USDC. Only 990 USDC enters reserve composition. DTF issuance uses actual reserve value added, not 1,000 or 990 USDC directly.

Fee-state reconciliation, assuming zero starting balances:

```text
before: FeeVault = 0, creator accrued = 0, protocol accrued = 0
after:  FeeVault = 10, creator accrued = 4, protocol accrued = 6
check:  gross input 1,000 = fee custody 10 + composition input 990
check:  FeeVault 10 = creator liability 4 + protocol liability 6
```

### Case 3: rounding boundary

Use `G = 199` smallest units:

```text
F = floor(199 * 100 / 10_000) = 1 unit
C_delta = floor(1 * 4000 / 10_000) = 0 units
P_delta = 1 - 0 = 1 unit
N = 199 - 1 = 198 units
```

The user is charged one smallest unit, never two. The indivisible split remainder goes to the protocol, and `C_delta + P_delta = F` remains exact.

Fee-state reconciliation, assuming zero starting balances:

```text
before: FeeVault = 0, creator accrued = 0, protocol accrued = 0
after:  FeeVault = 1, creator accrued = 0, protocol accrued = 1
check:  gross input 199 = fee custody 1 + composition input 198
```

### Case 4: repeated creator claim

Initial state:

```text
FeeVault = 10 USDC
creator accrued = 4 USDC
protocol accrued = 6 USDC
creator max claim = 4 USDC
```

The first authorized claim transfers 4 USDC and sets creator accrued to zero. FeeVault becomes 6 USDC. Repeating the claim finds zero creator accrual and fails without transfer or state change. Protocol accrued remains 6 USDC throughout.

Reconciliation:

```text
before first claim: FeeVault = 10, creator accrued = 4, protocol accrued = 6
after first claim:  FeeVault = 6, creator accrued = 0, protocol accrued = 6,
                    creator recipient increase = 4
after repeat:       unchanged from after first claim
check:              remaining custody 6 + paid 4 = original custody 10
```

### Case 5: unauthorized creator claimant

With creator accrued at 4 USDC, a signer other than the market's creator fee recipient authority requests a creator claim.

The authorization check fails. FeeVault remains 10 USDC, creator accrued remains 4 USDC, protocol accrued remains 6 USDC, and reserves are unchanged.

```text
before = after: FeeVault = 10, creator accrued = 4, protocol accrued = 6
unauthorized recipient increase = 0
reserve balance changes = 0
```

### Case 6: unauthorized protocol claimant

With protocol accrued at 6 USDC, the market creator, a generic protocol admin, or any signer other than the protocol treasury recipient authority requests a protocol claim.

The authorization check fails. FeeVault and both counters remain unchanged. Holding another administrative role does not confer treasury claim authority.

```text
before = after: FeeVault = 10, creator accrued = 4, protocol accrued = 6
unauthorized recipient increase = 0
reserve balance changes = 0
```

### Case 7: attempted reserve-vault claim

An otherwise authorized claimant supplies a reserve vault as the token source instead of the market FeeVault.

Market/custody binding and non-alias validation fail before transfer. No reserve token moves, no fee counter changes, and DTF backing remains unchanged.

```text
before = after: FeeVault = 10, creator accrued = 4, protocol accrued = 6
reserve balance changes = 0
recipient increase = 0
```

### Case 8: FeeVault balance mismatch or insufficient custody

State reports:

```text
FeeVault = 9 USDC
creator accrued = 4 USDC
protocol accrued = 6 USDC
total liabilities = 10 USDC
```

Even though the vault could individually cover a 4 USDC creator claim, `FeeVault < total liabilities`. The claim fails without paying either role or changing either counter. The deficit is surfaced for incident handling rather than allocated by claimant race order.

If FeeVault instead holds 11 USDC, claims remain capped by counters. The extra 1 USDC is unallocated surplus and is not claimable through either role.

Deficit-path reconciliation:

```text
before = after: FeeVault = 9, creator accrued = 4, protocol accrued = 6
recipient increase = 0
reserve balance changes = 0
```

Surplus-path liability reconciliation before any valid claim:

```text
FeeVault 11 = creator liability 4 + protocol liability 6 + surplus 1
claimable total = 10, not 11
```

### Case 9: failed mint does not accrue fees

A 1,000 USDC mint calculates a 10 USDC fee, but a later route, minimum-output, pricing, reserve-delta, or DTF-mint check fails.

Transaction rollback leaves the user transfer, FeeVault, creator counter, protocol counter, reserves, and DTF supply at their pre-transaction values. No successful accrual event exists.

```text
before = after:
user USDC, FeeVault, creator accrued, protocol accrued,
all reserve balances, DTF supply, and user DTF balance
```

### Case 10: redeem accrues no explicit fee

A redeem unwinds reserves and observes 250 USDC of actual output:

```text
redeem_fee_bps = 0
redeem_fee_usdc = 0
user_usdc_out = 250 USDC
```

The user receives the actual 250 USDC subject to the separate `min_usdc_out` check. FeeVault and both accrued counters are unchanged. Execution spread is not recorded as a creator or protocol fee.

```text
fee state before = fee state after:
FeeVault, creator accrued, and protocol accrued are unchanged
fee custody increase = 0
user USDC increase = actual redeem output of 250 USDC
reserve/unwind-side decrease funds that output under the redeem specification
```

## 15. Requirement mapping

The IDs below are canonical requirement IDs from `Axis-pizza/Axis_docs`. This proposal does not create new canonical IDs.

| Canonical ID | Canonical requirement | Proposal coverage |
| --- | --- | --- |
| FEE-001 | Creator fee is a required protocol concept | Sections 4–14 treat creator ownership, accrual, custody, claim, audit, and failure behavior as first-class protocol behavior. |
| FEE-002 | Each market stores a creator address | Section 4.2 requires an immutable market creator identity bound at creation. |
| FEE-003 | Each market stores a creator fee destination | Sections 4.2 and 10 define the immutable logical recipient, signer, and validated USDC destination requirements while deferring final representation. |
| FEE-004 | Creators cannot customize fee bps | Sections 2 and 4.2 preserve protocol-derived market fee configuration and reject recipient choice as a fee-policy control. |
| FEE-005 | Market fee configuration is immutable after creation | Section 2 requires an immutable market snapshot derived from protocol configuration. |
| FEE-006 | Mint fee is charged on gross USDC input | Sections 6 and 7.1 derive `mint_fee_usdc` from gross USDC input. |
| FEE-007 | Mint fee is deducted before reserve composition | Sections 6 and 7.1 define `G = F + N` and permit only `N` to enter reserve composition. |
| FEE-008 | Minted DTF uses net actual reserve value | Sections 7.1 and 8 exclude the fee and base issuance on actual reserve value added, not gross input or quotes. |
| FEE-009 | Redeem has no explicit Axis exit fee in v1 | Sections 2 and 7.3 plus Case 10 fix `redeem_fee_bps = 0` and accrue no redeem fee. |
| FEE-010 | Redeem still enforces execution protection | Section 7.3 and Case 10 preserve actual-USDC-output and `min_usdc_out` semantics while separating execution spread from fees. |
| FEE-011 | Creator and protocol shares sum to 10,000 bps | Section 6 requires the share-sum invariant and uses the protocol share as the exact residual. |
| FEE-012 | Fee bps are bounded by protocol caps | Sections 2 and 6 reference the canonical caps and require market values to remain within them. |
| FEE-013 | Fees use claim-based accrual | Sections 5.2, 7, and 9 define outstanding counters, successful-mint-only accrual, and explicit later claims. |
| FEE-014 | Creator fee claim is explicit | Sections 4.2 and 9–12 define creator authorization, bounded withdrawal, counter reduction, replay behavior, and atomic failure; Cases 4 and 5 exercise it. |
| FEE-015 | Protocol fee claim is explicit | Sections 4.3 and 9–12 define treasury authorization, bounded withdrawal, counter reduction, replay behavior, and atomic failure; Case 6 exercises it. |
| FEE-016 | Fee custody is separate from reserve custody | Sections 5.1 and 8 require distinct custody and prevent reserve debit; Case 7 covers attempted reserve-vault claim. |
| FEE-017 | Fee accounting cannot break reserve or NAV accounting | Sections 5.3, 7, and 8 exclude fees, liabilities, and surplus from reserve value, issuance, and NAV; Case 8 covers custody mismatch. |
| FEE-018 | Fee behavior is test-covered | Section 14 provides the proposal-level worked cases required to drive later unit and integration tests; executable tests remain outside this docs-only PR. |
| MINT-003 | Mint deducts fee before allocation | Sections 6 and 7.1 plus Cases 1–3 define gross-input fee calculation, floor rounding, and net composition input. |
| MINT-017 | Mint accrues creator and protocol fees | Sections 5.2, 7.1, 7.2, and 13.1 define successful-mint-only counter accrual and audit fields; Case 9 covers rollback. |
| MINT-018 | Mint preserves fee and reserve separation | Sections 5.1 and 8 define custody and accounting separation; Case 7 proves claims cannot source reserves. |
| PRICE-001 | NAV is based on actual reserve balances | Sections 5.3 and 8 exclude FeeVault balances, accrued liabilities, and fee surplus from reserve value and NAV. |
| PRICE-012 | Minted DTF uses actual added value | Sections 7.1 and 8 require actual reserve balance-delta value and exclude gross input, quotes, and mint fees; Case 2 demonstrates the boundary. |
| PRICE-013 | Redeem output uses actual USDC received | Section 7.3 and Case 10 preserve `user_usdc_out = actual_usdc_received` with zero explicit redeem fee. |
| ADMIN-008 | Fee administration respects protocol rules | Sections 2, 4, 5, 6, and 8 preserve protocol control, caps, share totals, immutable market snapshots, and fee/reserve/NAV separation. |

## 16. Boundaries and non-goals

This proposal does not implement or finalize:

- Rust handlers or validation code;
- account serialization, discriminators, field order, reserved bytes, or account sizes;
- instruction names, discriminants, ABI, account-meta order, or client SDK;
- token transfers or token-program CPI;
- mint, redeem, or claim handler ordering;
- final PDA seeds, bumps, or derivation rules;
- final integer field widths or arithmetic helpers;
- Token versus Token-2022 adoption or extension policy;
- associated-token-account requirements;
- protocol treasury rotation or creator recipient recovery;
- fee-vault deficit or surplus recovery;
- shared custody or cross-market sweeping;
- backend, indexer, app, database, or UI implementation; or
- changes to canonical fee rates, caps, or commercial policy.

## 17. Dependencies

Before implementation approval, this proposal must be reconciled with:

- P0-SPEC-04 / PR #57 for final account ownership and state boundaries;
- P0-SPEC-05 / PR #58 for final instruction semantics and account roles;
- P0-ROUTE-06 / PR #59 for mint execution validation boundaries;
- canonical mint, redeem, pricing/NAV, admin/safety, and fee requirements in `Axis-pizza/Axis_docs`; and
- later implementation issues for mint fee accrual and explicit creator/protocol claims.

No unresolved choice in those PRs is adopted merely because a candidate name or shape appears in this document.

## 18. Unresolved implementation questions

1. Are fee counters inline in market state or held in a separate market fee state account?
2. What are the final account layouts, serialization, discriminators, versioning, integer widths, reserved bytes, and sizes?
3. What are the final FeeVault, market, and fee-state PDA seeds and validation rules?
4. Are creator and protocol claims separate instructions or one role-parameterized instruction?
5. How is `max_claim_amount` encoded, and should a client offer an explicit claim-all convenience?
6. Does the creator model store a recipient authority, an exact USDC destination, or both?
7. Is the protocol recipient snapshotted per market or resolved from current protocol configuration?
8. How do protocol treasury rotation and already-accrued protocol fees interact?
9. Is creator recipient recovery ever allowed, and if so, under what governance, timelock, or dual-authorization rules?
10. Are claims allowed while the protocol or market is paused or deprecated?
11. Which USDC token program and token-account extensions are accepted or rejected?
12. Must destinations be associated token accounts, or may any correctly owned USDC token account be used?
13. What checked/widened arithmetic implementation prevents overflow before division?
14. Should lifetime accrued, lifetime claimed, or a market fee sequence be stored on-chain for auditability?
15. What exact event/log schema and versioning are used?
16. What incident process handles FeeVault deficit without favoring one fee owner?
17. What governed process, if any, handles unsolicited FeeVault surplus?
18. Can fee custody ever be closed after market deprecation, and what zero-liability checks are required?
19. How should claim account locking and compute behavior scale if protocol fees are later swept across markets?
20. Which error classes distinguish no entitlement, invalid authority, custody mismatch, deficit, overflow, and token transfer failure?
