# Axis Core P0 Instruction Surface Proposal (P0-SPEC-05)

> Status: proposal for review. This document defines instruction semantics, validation
> boundaries, and accounting invariants. It does not define a Rust handler, serialized
> instruction enum, ABI, account-meta ordering, client SDK, or venue-specific encoding.

## 1. Decision requested

Approve the semantic P0 instruction surface in this document as the contract between
later account-model, ABI, handler, adapter, and test work.

Approval of this proposal does **not** approve unresolved account layouts from PR #57,
an `ApprovedRoute` granularity, a P0 `PricingSource` subset, Token-2022 extensions,
fixed-point or rounding rules, or production venue account encodings.

## 2. Scope

This proposal covers:

- protocol initialization, bounded configuration updates, and protocol pause;
- asset configuration, independent execution flags, and execution policy limits;
- pricing-source administration as a semantic boundary only;
- approved-route administration as a semantic boundary only;
- market creation, activation, pause/unpause, and terminal deprecation;
- USDC-in DTF mint and DTF-in USDC redeem;
- separate creator and protocol fee claims; and
- emergency exit-only operation.

The proposed names are semantic names. Final Rust names, discriminants, payload
serialization, account order, optional-account representation, and remaining-account
encoding require a later ABI proposal.

## 3. Non-goals

This proposal does not:

- implement Rust handlers, an instruction enum, an ABI, or a client SDK;
- implement CPI execution, mint/redeem behavior, fee claims, or route validation;
- define production Orca, Raydium, or other venue instruction/account encodings;
- define a general-purpose route graph or aggregator;
- finalize `ApprovedRoute` granularity or PDA derivation;
- select the P0 `PricingSource` variants or source-specific accounts;
- finalize Token-2022 adoption or an extension policy;
- redefine the fee model or its accounting layout;
- add market close, rebalance execution, Auction Program, ClearCorrection, or
  Axis-controlled JIT liquidity.

## 4. Canonical references and dependencies

Canonical requirements:

- `Axis-pizza/Axis_docs/requirements/03-mint-requirements.md`
- `Axis-pizza/Axis_docs/requirements/04-redeem-requirements.md`
- `Axis-pizza/Axis_docs/requirements/05-swap-cpi-execution-requirements.md`
- `Axis-pizza/Axis_docs/requirements/07-execution-policy-risk-controls.md`
- `Axis-pizza/Axis_docs/requirements/09-admin-safety-requirements.md`
- `Axis-pizza/Axis_docs/requirements/13-fee-model-requirements.md`
- `Axis-pizza/Axis_docs/requirements/19-axis-core-implementation-rfc.md`

Repository dependencies:

- `docs/context/axis-core-implementation-brief.md` is listed as an input dependency.
  If it is unavailable in this branch, no decision is inferred from it.
- The account model is an explicit dependency on
  [PR #57](https://github.com/Axis-pizza/axis_core/pull/57). The proposal document from
  that PR was reviewed, but its unresolved decisions remain unresolved here.
- Fee claims depend on the fee-accounting proposal tracked as P0-FEE-07. This proposal
  states claim boundaries only. P0-FEE-07 owns the final custody layout, counter
  layout, partial-versus-all claim behavior, and per-market-versus-shared fee custody
  decision.

If this document conflicts with a canonical Axis_docs requirement, the canonical
requirement wins until the conflict is resolved by review.

## 5. Truth and authority boundaries

### 5.1 Truth model

| Domain | Status | Rule |
|---|---|---|
| Route-builder plan, quote, expected output, target weights, backend/UI/indexer data | Advisory | May be passed as instruction data, but is never protocol accounting truth. |
| On-chain configuration | Enforced policy | Axis Core validates current protocol, market, asset, pricing, route, mint, token-program, and authority state. |
| Actual program-observed balance deltas | Execution truth | Pre/post balances on validated token accounts determine actual input and output. |
| Reserve accounting | Protocol accounting truth | Actual balances of program-controlled reserve vaults are backing truth; approved prices may value those balances. |
| Fee accounting | Separate accounting domain | Fee custody and accrued claims are excluded from reserve value and NAV. Fee claims cannot mutate reserve custody. |

The route builder or backend may provide route identifiers, account assembly,
venue-specific data, quotes, expected price impact, and minimum outputs. Axis Core must
independently validate the loaded on-chain route and policy, invoke only an approved
adapter/program, observe actual token movement, and derive accounting from those
observations. Backend output is never protocol accounting truth.

### 5.2 Authority model

| Authority | Scope | Must not imply |
|---|---|---|
| Admin/config authority (`authority`) | Initialize and update allowed protocol configuration; bounded future-market fee config | Route, asset, pause, or fee-recipient authority unless the configured keys are intentionally equal |
| Pause authority | Protocol and market emergency state | General config mutation |
| Asset policy authority (`asset_registry_authority`) | Asset config, independent flags, execution limits | Route approval or market fee changes |
| Route authority (`route_registry_authority`) | Create/update/disable approved-route boundaries | Accounting truth or custody control |
| Pricing authority (`pricing_registry_authority`) | Create/update/disable supported pricing boundaries | Selection of an unapproved P0 pricing implementation |
| Market creator | Create a market and nominate the immutable creator fee destination | Custom market fee bps or protocol administration |
| Fee recipient authority | Claim only its accrued fee entitlement | Reserve access, market mutation, or another recipient's claim |
| User | Authorize USDC spend or DTF burn and choose minimum output | Bypass policy, route, pricing, or balance-delta validation |

### 5.3 Fee model imported from canonical requirements

This proposal does not redefine fee accounting. It imports these launch constraints
from `13-fee-model-requirements.md` and leaves custody/counter mechanics to P0-FEE-07:

| Parameter | Launch value |
|---|---:|
| `mint_fee_bps` | 100 |
| `redeem_fee_bps` | 0 |
| `creator_share_bps` | 4,000 |
| `protocol_share_bps` | 6,000 |
| `max_mint_fee_bps` | 300 |
| `max_redeem_fee_bps` | 0 |

Each market snapshots the applicable valid protocol fee configuration at creation.
That market snapshot is immutable in P0. Protocol fee-template updates affect future
markets only.

## 6. Common validation and rejection model

Every instruction must reject unexpected writable or executable program accounts and
validate account ownership, PDA relationship, stored key relationships, mint
relationships, token-account authority, and token-program ownership before mutation.
Caller-supplied program IDs are never trusted merely because they are executable.

Error classes used below:

| Code | Error class / rejection point |
|---|---|
| `AUTH` | Missing signer or signer does not match the authority stored in validated state |
| `OWNER` | Wrong account owner, executable program, PDA relationship, or account role |
| `STATE` | Invalid lifecycle transition, paused/disabled state, immutable-field update, or already initialized state |
| `POLICY` | Asset, pricing, route-complexity, trade-size, weight, price-impact, or emergency-policy rejection |
| `ROUTE` | Missing, disabled, stale, mismatched, or unapproved route/venue/pool/mint/direction |
| `TOKEN` | Wrong mint, token program, token authority, token-account role, or unsupported extension |
| `FEE` | Fee cap/share/config/custody/accrual/claim invariant failure |
| `SLIPPAGE` | Required minimum absent/zero where prohibited, or observed output below minimum |
| `DELTA` | Impossible, wrong-direction, aliased, or inconsistent observed balance delta |
| `ARITH` | Overflow, underflow, division by zero, invalid precision, or unresolved rounding cannot be applied safely |

A failure at any validation point must occur before irreversible state is accepted.
Solana transaction atomicity must make mint, redeem, and claim operations all-or-nothing.

## 7. Instruction summary

| Group | Proposed instruction | Signer | CPI permission | Primary state transition |
|---|---|---|---|---|
| Protocol | `initialize_protocol_config` | Initial admin/config authority | System account creation only | Uninitialized → initialized |
| Protocol | `update_protocol_config` | Current admin/config authority | None | Allowed config fields updated |
| Protocol | `update_protocol_fee_config` | Current admin/config authority | None | Future-market fee template updated within caps |
| Safety | `set_protocol_paused` | Pause authority | None | Active ⇄ paused |
| Asset | `upsert_asset_config` | Asset policy authority | System account creation only when new | Missing/existing → configured |
| Asset | `set_asset_execution_flags` | Asset policy authority | None | Independent execution flags updated |
| Pricing | `upsert_pricing_source` | Pricing authority | System account creation only when new | Boundary missing/existing → configured |
| Pricing | `set_pricing_source_enabled` | Pricing authority | None | Enabled ⇄ disabled |
| Route | `upsert_approved_route` | Route authority | System account creation only when new | Boundary missing/existing → configured |
| Route | `set_approved_route_enabled` | Route authority | None | Enabled ⇄ disabled |
| Market | `create_market` | Market creator | System/token account creation; no venue CPI | Missing → Created |
| Market | `activate_market` | Admin/config authority | None | Created → Active; paused reactivation unresolved |
| Market | `set_market_paused` | Pause authority | None | Active ⇄ Paused |
| Market | `deprecate_market` | Admin/config authority | None | Created/Active/Paused → Deprecated |
| Mint | `mint_dtf_with_usdc` | User/payer | Approved token and venue CPI only | Reserves/supply/fee accrual increase atomically |
| Redeem | `redeem_dtf_for_usdc` | User/owner | Approved token and venue CPI only | Supply/reserves decrease; user USDC increases atomically |
| Fee | `claim_creator_fee` | Creator fee recipient authority | Configured USDC token program only | Creator accrual and fee-vault balance decrease |
| Fee | `claim_protocol_fee` | Protocol treasury authority | Configured USDC token program only | Protocol accrual and fee-vault balance decrease |

`upsert_pricing_source` and `set_pricing_source_enabled` approve only the administrative
boundary. They cannot be encoded or implemented until review selects the P0 source
subset and validation contract.

## 8. Detailed instruction specifications

### 8.1 `initialize_protocol_config`

| Field | Proposal |
|---|---|
| Intent | Create the singleton protocol configuration and establish separated authorities, configured USDC, pause state, and bounded launch fee template. |
| Inputs | Authority keys; configured USDC mint and token program; fee values/caps and protocol treasury; initial paused state if allowed. Semantic only, not ABI fields. |
| Required signer(s) | Initial admin/config authority and payer if distinct. |
| Authority checks | Initialization path must be bound to deployment/governance policy; no existing config may exist. |
| Account roles | Protocol config PDA; payer; System Program; configured USDC mint; optional rent/sysvar roles required by the eventual runtime. |
| Writable accounts | Protocol config and payer only. |
| Expected token programs | No token CPI. Stored USDC token program must own the USDC mint and be supported by the approved token policy. |
| Preconditions | Canonical config PDA is uninitialized; all authorities and treasury are valid non-default keys; fee shares total 10,000 bps; fee values do not exceed caps; launch redeem fee/cap are zero. |
| Validation rules | Reject duplicate initialization, invalid PDA/owner, unsupported USDC mint/program pairing, invalid caps/shares, and authority aliasing that violates an approved separation policy. |
| Allowed CPI | System account creation/assignment only. |
| State changes | Persist protocol authorities, configured USDC boundary, fee template/caps, and protocol pause state. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable; no token balance may change. |
| Postconditions | Exactly one valid protocol config exists and all later instructions can derive authority and USDC policy from it. |
| Failure conditions | Existing config; missing signer; wrong PDA/owner; invalid authority/treasury; fee bound/share failure; unsupported USDC program. |
| Expected error/rejection | `AUTH` → `OWNER` → `TOKEN`/`FEE` before account initialization is committed. |
| Unresolved questions | Deployment initializer policy, authority aliasing policy, final PDA/layout, and whether initialization must start paused. |

### 8.2 `update_protocol_config`

| Field | Proposal |
|---|---|
| Intent | Rotate explicitly mutable protocol authority/config fields without changing existing market fee snapshots or custody relationships. |
| Inputs | Field-mask or equivalent plus replacement values; exact encoding deferred. |
| Required signer(s) | Current admin/config authority. |
| Authority checks | Signer must equal current stored authority; a new authority does not authorize the transaction that installs it. |
| Account roles | Protocol config; any referenced new authority/treasury keys as read-only identity inputs. |
| Writable accounts | Protocol config only. |
| Expected token programs | None. Changing configured USDC mint/program after initialization is prohibited unless a separate migration specification approves it. |
| Preconditions | Protocol initialized; update targets only review-approved mutable fields. |
| Validation rules | Immutable keys and fee template are rejected here; zero/default authorities rejected; existing market state cannot be rewritten. |
| Allowed CPI | None. |
| State changes | Only selected mutable protocol fields change. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable; token balances must remain unchanged. |
| Postconditions | New authorities are effective for subsequent transactions; market snapshots and vault bindings are unchanged. |
| Failure conditions | Missing/wrong authority; immutable-field mutation; invalid new key; wrong config owner/PDA. |
| Expected error/rejection | `AUTH` or `OWNER`, then `STATE`, before write. |
| Unresolved questions | Which authority rotations require multisig/timelock and whether registry authorities may be intentionally aliased. |

### 8.3 `update_protocol_fee_config`

| Field | Proposal |
|---|---|
| Intent | Update the protocol fee template for markets created later, within immutable protocol caps. |
| Inputs | Mint/redeem fee bps and creator/protocol share bps; treasury changes belong to the approved config path, not implicitly here. |
| Required signer(s) | Current admin/config authority. |
| Authority checks | Stored config authority must sign. Creators and fee recipients have no fee-rate authority. |
| Account roles | Protocol config containing the protocol fee template/caps. |
| Writable accounts | Protocol config only. |
| Expected token programs | None. |
| Preconditions | Config initialized; fee updates supported by final account model. |
| Validation rules | `mint_fee_bps <= max_mint_fee_bps`; redeem fee/cap remain zero for launch; shares sum to 10,000; no existing market account is writable. |
| Allowed CPI | None. |
| State changes | Future-market fee template changes; existing market fee snapshots remain byte-for-byte unchanged. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | New markets snapshot the new valid template; old markets retain their original immutable fee state. |
| Failure conditions | Unauthorized update; cap/share violation; attempt to update market fee state; arithmetic range failure. |
| Expected error/rejection | `AUTH` → `FEE`/`STATE` before write. |
| Unresolved questions | Whether caps themselves are immutable constants or governable fields, and whether fee-template updates require timelock. |

### 8.4 `set_protocol_paused`

| Field | Proposal |
|---|---|
| Intent | Quickly stop unsafe protocol actions and restore them after review. |
| Inputs | `paused`; optional emergency exit policy only if the account model later supports it. |
| Required signer(s) | Pause authority. |
| Authority checks | Signer must equal stored pause authority. |
| Account roles | Protocol config/safety state. |
| Writable accounts | Protocol config/safety state only. |
| Expected token programs | None. |
| Preconditions | Protocol initialized. |
| Validation rules | Pause blocks create, activate, mint, and non-safety mutations selected by final policy. Redeem may remain available only under an explicit safe exit-only policy; no implicit bypass is allowed. |
| Allowed CPI | None. |
| State changes | Protocol safety state changes Active ⇄ Paused. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Every affected instruction observes the new state before any CPI or mutation. |
| Failure conditions | Missing/wrong pause signer; invalid transition/policy; wrong config owner/PDA. |
| Expected error/rejection | `AUTH`/`OWNER`, then `STATE`. |
| Unresolved questions | Exact paused-redeem semantics, whether unpause requires admin concurrence/timelock, and whether safety state is a field or separate account. |

### 8.5 `upsert_asset_config`

| Field | Proposal |
|---|---|
| Intent | Create an asset config or update its review-approved policy limits without automatically approving a route or pricing implementation. |
| Inputs | Asset mint; immutable identity data when created; max trade, max weight, max price impact, pricing/deviation requirements, manual-review and approved-route requirements; optional initial flags. |
| Required signer(s) | Asset policy authority; payer when creating. |
| Authority checks | Stored `asset_registry_authority` must sign. |
| Account roles | Protocol config; asset config PDA; asset mint; payer/System Program when new. |
| Writable accounts | Asset config and payer when new. |
| Expected token programs | No token CPI. Asset mint owner/token program and extensions must satisfy the currently approved asset policy; Token-2022 extension policy remains unresolved. |
| Preconditions | Protocol not paused for non-safety administration; mint is valid; hard minimum is exactly 1 USDC; limits are representable and internally consistent. |
| Validation rules | Preserve immutable mint identity; reject automatic enablement from discovery; reject unsafe limits; flags remain independent; route/pricing references are not treated as proof of readiness. |
| Allowed CPI | System account creation only when new. |
| State changes | Asset policy state is created or selected mutable limits are updated. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Runtime create/mint/redeem checks can load one authoritative asset policy; existing reserves are untouched. |
| Failure conditions | Unauthorized signer; wrong mint/program/PDA; invalid limits; immutable mint change; paused non-safety update. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN` → `STATE`/`POLICY`. |
| Unresolved questions | Combined versus split registry/policy accounts, support-status representation, preset encoding, and Token-2022 extension policy. |

### 8.6 `set_asset_execution_flags`

| Field | Proposal |
|---|---|
| Intent | Independently enable/disable creation, mint, redeem, and rebalance, including rapid transition to exit-only mode. |
| Inputs | Four explicit booleans or an equivalent complete flag set; partial-update encoding deferred. |
| Required signer(s) | Asset policy authority. |
| Authority checks | Stored `asset_registry_authority` must sign. |
| Account roles | Protocol config; asset config. |
| Writable accounts | Asset config only. |
| Expected token programs | None. |
| Preconditions | Asset config exists and matches its mint. Safety-off updates must be permitted while protocol is paused; re-enablement may require protocol active/manual review. |
| Validation rules | Flags are independent. Exit-only is `creation=false, mint=false, redeem=true, rebalance=false`. Disabling redeem must be explicit and should occur only when redeem itself is unsafe. |
| Allowed CPI | None. |
| State changes | Only asset execution flags change. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Subsequent create/mint/redeem/rebalance checks use the new flags; existing reserve custody is unchanged. |
| Failure conditions | Unauthorized signer; wrong asset account; prohibited re-enable while paused; malformed flag update. |
| Expected error/rejection | `AUTH`/`OWNER`, then `STATE`/`POLICY`. |
| Unresolved questions | Re-enable review/timelock rules and whether pause authority also receives disable-only capability. |

### 8.7 `upsert_pricing_source`

| Field | Proposal |
|---|---|
| Intent | Establish or update the on-chain pricing-validation boundary for an asset without selecting unsupported source types. |
| Inputs | Asset binding and source-specific policy/reference data defined only after the P0 pricing subset is approved. |
| Required signer(s) | Pricing authority; payer when creating. |
| Authority checks | Stored `pricing_registry_authority` must sign. |
| Account roles | Protocol config; asset config; pricing boundary; payer/System Program when new; source accounts read-only as required by the future subset. |
| Writable accounts | Pricing boundary and payer when new. |
| Expected token programs | None. |
| Preconditions | Asset exists; source type belongs to the separately approved P0 subset. |
| Validation rules | Preserve asset binding; validate source ownership/reference/freshness policy; reject an implementation not yet approved for P0. |
| Allowed CPI | System account creation only when new. No oracle/venue CPI is approved here. |
| State changes | Pricing boundary is created or mutable policy/reference fields change. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Consumers can validate a current enabled source under a separately approved pricing contract. |
| Failure conditions | Unauthorized signer; wrong owner/reference; unsupported source type; invalid freshness/deviation policy. |
| Expected error/rejection | `AUTH`/`OWNER`, then `POLICY`/`STATE`. |
| Unresolved questions | Entire P0 source subset, account shape, source-specific references, multi-source rules, fixed-point math, freshness units, and update semantics. |

### 8.8 `set_pricing_source_enabled`

| Field | Proposal |
|---|---|
| Intent | Disable a stale/unsafe source quickly or re-enable it after approved review. |
| Inputs | `enabled`. |
| Required signer(s) | Pricing authority. |
| Authority checks | Stored `pricing_registry_authority` must sign. |
| Account roles | Protocol config; pricing boundary. |
| Writable accounts | Pricing boundary only. |
| Expected token programs | None. |
| Preconditions | Pricing boundary exists and is owned/bound correctly. |
| Validation rules | Disable must remain available during emergencies; re-enable must pass the source-specific validation contract. |
| Allowed CPI | None. |
| State changes | Pricing enabled state changes. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Instructions requiring pricing reject a disabled source before execution. |
| Failure conditions | Unauthorized signer; wrong owner/binding; unsafe re-enable. |
| Expected error/rejection | `AUTH`/`OWNER`, then `POLICY`. |
| Unresolved questions | Whether enablement is stored on one source or a registry entry and what evidence is required for re-enable. |

### 8.9 `upsert_approved_route`

| Field | Proposal |
|---|---|
| Intent | Create or update an approved execution boundary for a venue program, venue/pool account set, input/output mints, and direction. |
| Inputs | Route identity/binding, expected venue program, approved venue/pool references, input/output mints, direction, bounded complexity, and freshness/revision semantics. Exact payload and granularity deferred. |
| Required signer(s) | Route authority; payer when creating. |
| Authority checks | Stored `route_registry_authority` must sign. |
| Account roles | Protocol config; affected asset configs; approved-route boundary; venue program/account identities read-only; payer/System Program when new. |
| Writable accounts | Approved-route boundary and payer when new. |
| Expected token programs | No token CPI. Any recorded token-program boundary must match the mint owners and approved token policy. |
| Preconditions | Assets exist; venue program is explicitly approved; direct route is preferred; no split/arbitrary graph/SOL intermediate/unsupported multi-hop. |
| Validation rules | Bind venue program, venue/pool accounts, mints, direction, and enabled revision. A quote or `route_available` response cannot create approval. Material identity changes should create a new boundary rather than silently retargeting an existing approval. |
| Allowed CPI | System account creation only when new; no venue CPI. |
| State changes | Route approval boundary is created or review-approved mutable metadata/state changes. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Runtime execution can compare caller data/accounts against current enabled on-chain approval. |
| Failure conditions | Unauthorized signer; unapproved venue; wrong mint/program/account binding; unsupported complexity; stale update; immutable identity retarget. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN` → `ROUTE`/`POLICY`. |
| Unresolved questions | Route/asset-pair/venue/pool granularity, PDA seeds, revision/expiry model, controlled-adapter representation, and production venue account encodings. |

### 8.10 `set_approved_route_enabled`

| Field | Proposal |
|---|---|
| Intent | Pause/disable an unsafe route immediately or re-enable it after validation. |
| Inputs | `enabled`; expected current revision or equivalent stale-write protection. |
| Required signer(s) | Route authority. |
| Authority checks | Stored `route_registry_authority` must sign. |
| Account roles | Protocol config; approved-route boundary. |
| Writable accounts | Approved-route boundary only. |
| Expected token programs | None. |
| Preconditions | Route boundary exists and matches expected identity/revision. |
| Validation rules | Stale administrative writes fail. Disable must remain available during emergencies; re-enable must validate the current venue/program/account boundary. |
| Allowed CPI | None. |
| State changes | Route enabled state and, if approved, revision metadata change. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | New mint/redeem execution cannot use a disabled or stale route. |
| Failure conditions | Unauthorized signer; missing/wrong route; stale revision; unsafe re-enable. |
| Expected error/rejection | `AUTH`/`OWNER`, then `ROUTE`/`STATE`. |
| Unresolved questions | Exact freshness/revision mechanism and whether disable-only authority is delegated to pause authority. |

### 8.11 `create_market`

| Field | Proposal |
|---|---|
| Intent | Create a DTF market in `Created` state, bind its DTF mint/reserve/fee custody, and snapshot the current protocol fee template. |
| Inputs | Market identity/nonce candidate; creator and creator fee destination; 2–5 asset mints and target weights; DTF mint parameters allowed by the future token policy. No creator-selected fee rates. |
| Required signer(s) | Market creator and payer; additional mint authority only if the final creation model requires it. |
| Authority checks | Creator signs; protocol-derived authority controls DTF mint and vaults; creator cannot act as protocol/route/asset authority merely by creating a market. |
| Account roles | Protocol config; new market; creator/payer; DTF mint; configured USDC mint; per-asset configs and pricing boundaries; reserve vaults; fee vault; System Program; approved token programs and associated-token program only if the final account model uses it. |
| Writable accounts | New market, payer, DTF mint when created, reserve vaults, fee vault; no existing asset/route/pricing config writable. |
| Expected token programs | Configured USDC token program plus the approved DTF/reserve token programs. Token-2022 for DTF is a candidate, not finalized; extensions must pass the future policy. |
| Preconditions | Protocol active; creation enabled for every asset; 2–5 unique assets; weights sum to 10,000 bps and respect each max weight; required pricing/route readiness exists; custody accounts are distinct and initially safe. |
| Validation rules | Validate every mint/program/authority/PDA relationship; snapshot protocol fee config; reject custom fee input; reject fee-vault/reserve alias; target weights are policy/advisory composition targets, not reserve truth. |
| Allowed CPI | System and approved token-program account/mint initialization only. No venue swap CPI. |
| State changes | Market becomes `Created`; immutable creator, fee snapshot, asset set/weights, DTF mint, and custody bindings are recorded per the approved account model. |
| Token-balance changes | New custody token accounts must begin at zero unless a separately specified atomic initial-mint path is approved; no DTF supply or fee accrual is created here. |
| Actual-balance-delta checks | Confirm no unexpected reserve/fee/supply balance change and no pre-funded aliased custody is accepted. |
| Postconditions | Market is not mintable until activated; fee and reserve custody are separated; no market close path is created. |
| Failure conditions | Paused protocol; disabled/unregistered asset; invalid weights/count/duplicates; missing/stale pricing or route readiness where required; wrong mint/program; custody alias; invalid creator destination; custom fee attempt. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN` → `STATE`/`POLICY`/`ROUTE`/`FEE`/`DELTA`. |
| Unresolved questions | Market/PDA derivation, DTF mint creation versus validation, exact activation gate, reserve-vault derivation, pricing subset, Token-2022 policy, and whether route readiness is required at creation or activation. |

### 8.12 `activate_market`

| Field | Proposal |
|---|---|
| Intent | Move a fully validated market into `Active` state. |
| Inputs | Expected market revision/version if the final state model supports stale-write protection. |
| Required signer(s) | Admin/config authority. |
| Authority checks | Stored protocol authority must sign. |
| Account roles | Protocol config; market; all asset configs; required pricing boundaries; required approved-route boundaries; DTF mint and custody accounts read-only for validation. |
| Writable accounts | Market only. |
| Expected token programs | No CPI. Validate DTF/reserve/USDC mint owners against approved token programs. |
| Preconditions | Protocol active; market `Created` or approved reactivation state; every asset enabled for creation/mint; custody, pricing, route, fee, and token policies valid. |
| Validation rules | Revalidate current on-chain state rather than creation-time/backend claims; reject stale/disabled route or pricing state and custody aliases. |
| Allowed CPI | None. |
| State changes | Market status becomes `Active`. |
| Token-balance changes | None. |
| Actual-balance-delta checks | No token balance or supply may change. |
| Postconditions | Mint is permitted only while all runtime checks continue to pass; activation does not freeze mutable safety controls. |
| Failure conditions | Unauthorized signer; paused protocol; invalid lifecycle; disabled asset; missing/stale route/pricing; wrong mint/program; custody alias. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN` → `STATE`/`POLICY`/`ROUTE`/`FEE`. |
| Unresolved questions | Whether creator can request/self-activate, whether Paused→Active uses this or `set_market_paused(false)`, and exact activation evidence. |

### 8.13 `set_market_paused`

| Field | Proposal |
|---|---|
| Intent | Pause or unpause one market without changing assets, fee snapshot, custody, or supply. |
| Inputs | `paused`. |
| Required signer(s) | Pause authority. |
| Authority checks | Stored pause authority must sign. |
| Account roles | Protocol config; market. |
| Writable accounts | Market only. |
| Expected token programs | None. |
| Preconditions | Market exists and is not deprecated. Unpause requires protocol active and may require full readiness revalidation through `activate_market`. |
| Validation rules | Paused market always blocks mint. Redeem behavior follows an explicit safe exit-only policy and all asset/route/pricing checks; pause alone never bypasses them. |
| Allowed CPI | None. |
| State changes | `Active ⇄ Paused`; no transition out of `Deprecated`. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Runtime instructions observe the new market state before CPI. |
| Failure conditions | Unauthorized signer; deprecated market; invalid transition; unsafe unpause. |
| Expected error/rejection | `AUTH`/`OWNER`, then `STATE`/`POLICY`. |
| Unresolved questions | Exact paused-market redeem rule and whether unpause belongs exclusively to admin/config authority. |

### 8.14 `deprecate_market`

| Field | Proposal |
|---|---|
| Intent | Enter the terminal v1 lifecycle state without closing the market or custody accounts. |
| Inputs | Optional reason/event data that has no accounting effect. |
| Required signer(s) | Admin/config authority. |
| Authority checks | Stored protocol authority must sign. |
| Account roles | Protocol config; market. |
| Writable accounts | Market only. |
| Expected token programs | None. |
| Preconditions | Market exists and is not already deprecated. |
| Validation rules | Deprecation blocks create/activate/mint/rebalance. Redeem may remain available only under explicit exit-only policy and current safety checks. No close, sweep, or reserve transfer is implied. |
| Allowed CPI | None. |
| State changes | Market status becomes terminal `Deprecated`. |
| Token-balance changes | None. |
| Actual-balance-delta checks | Not applicable. |
| Postconditions | Market cannot return to Active under P0 and all custody remains accountable. |
| Failure conditions | Unauthorized signer; wrong market; prohibited transition. |
| Expected error/rejection | `AUTH`/`OWNER`, then `STATE`. |
| Unresolved questions | Final deprecated-market redeem rule and any future zero-balance close policy, which is out of P0. |

### 8.15 `mint_dtf_with_usdc`

| Field | Proposal |
|---|---|
| Intent | Accept gross USDC, separate/accrue the mint fee, compose reserves with net USDC through approved execution, and mint DTF from actual added reserve value. |
| Inputs | `gross_usdc_in > 0`; required per-asset nonzero `min_out`; route identifiers/current revision bindings; bounded adapter data; advisory quotes/weights/expected price impact. Exact ABI deferred. |
| Required signer(s) | User/owner authorizing the USDC source; payer if distinct. |
| Authority checks | User controls/delegates the source account and DTF destination as allowed by token policy. Protocol PDA authority alone controls fee/reserve custody and DTF mint authority. |
| Account roles | Protocol config; active market; market DTF mint; user USDC source; user DTF destination; fee vault; market fee state/counters; each asset config and required pricing source; each current approved route; net-USDC execution source if separate; each reserve vault; approved token programs; approved venue programs and strictly validated venue accounts. |
| Writable accounts | User USDC source; user DTF destination; DTF mint; fee vault; fee counters/market; execution source; reserve vaults; only venue accounts required writable by the approved adapter. Config, pricing, and route accounts remain read-only. |
| Expected token programs | Configured USDC program, approved reserve-asset programs, and approved DTF mint program. Program IDs must match mint owners and policy; Token-2022 extension behavior is not assumed. |
| Preconditions | Protocol and market allow mint; every asset `mint_enabled`; route/pricing current and enabled; gross input and all minimums valid; net allocations are at least 1 USDC and at most `max_trade_usdc`; price impact within policy; fee config valid; no custody alias. |
| Validation rules | Compute fee from gross USDC; move/retain fee in separate fee custody; compose only `net_usdc_for_composition`; validate route venue/pool/mints/direction/complexity on-chain; snapshot pre-trade reserve balances and supply; use approved prices only for actual received reserves; target weights and quotes remain advisory. Any NAV input or supply conversion belongs to the deferred mint-accounting formula. |
| Allowed CPI | Configured token-program transfers/mint plus an approved controlled/production venue adapter for one bounded route per asset. No arbitrary program, split route, unsupported multi-hop, or SOL intermediary. |
| State changes | On success only: creator/protocol fee accrual increases by the canonical split; reserve accounting reflects actual balances; DTF supply increases by the amount derived from actual added reserve value. |
| Token-balance changes | User USDC decreases by gross input; fee vault increases by mint fee; only net USDC is available to execution; reserve vaults increase by observed outputs; user DTF and total supply increase by the same minted amount. |
| Actual-balance-delta checks | Snapshot before CPI. For each validated reserve vault, `actual_received_i = post - pre > 0` and `>= min_out_i`; input moves only in expected direction; fee-vault increase equals accrued fee; fee vault is not a reserve; DTF destination/supply increase equals computed mint. Quote mismatch is acceptable only when all observed-delta, value, policy, and minimum checks pass. |
| Postconditions | Minted DTF is based on actual added reserve value, never a quote or gross USDC input. Fees are excluded from reserves/NAV, and no partial swap, accrual, or mint survives failure. Fixed-point representation, rounding, initial NAV handling, and the final NAV/supply formula are deferred. |
| Failure conditions | Missing signer; wrong owner/program/mint; paused protocol/market; disabled asset; stale/unapproved route or pricing; policy breach; fee/reserve alias; missing/zero minimum; actual output below per-asset minimum; delta mismatch; arithmetic/pricing/rounding failure. |
| Expected error/rejection | Static `AUTH`/`OWNER`/`TOKEN`/`STATE`/`POLICY`/`ROUTE`/`FEE` before CPI; `SLIPPAGE`/`DELTA`/`ARITH` after observation and before commit. |
| Unresolved questions | Fixed-point representation, rounding/dust, initial NAV handling, final NAV/supply formula, account ordering, execution source design, route freshness encoding, pricing subset, Token-2022 policy, and adapter ABI. |

### 8.16 `redeem_dtf_for_usdc`

| Field | Proposal |
|---|---|
| Intent | Burn user DTF, unwind only the user's pro-rata reserves through approved execution, and pay the actual observed USDC output subject to `min_usdc_out`. |
| Inputs | `dtf_amount_in > 0`; required nonzero `min_usdc_out`; per-asset route identifiers/current revision bindings; bounded adapter data and advisory quotes. Exact ABI deferred. |
| Required signer(s) | User/owner authorizing the DTF source. |
| Authority checks | User controls/delegates the DTF source; protocol PDA alone controls reserve vaults and burn/unwind authority as applicable. |
| Account roles | Protocol config; market; DTF mint; user DTF source; user USDC destination; temporary/program-controlled USDC settlement account if required; fee vault read-only; each reserve vault; each asset config and required pricing source; current approved unwind routes; approved token and venue programs/accounts. |
| Writable accounts | User DTF source; DTF mint; user USDC destination; settlement account; reserve vaults; only approved venue accounts required writable. Fee vault/counters must be read-only; no redeem fee accrues. |
| Expected token programs | Configured USDC program, approved reserve-asset programs, and DTF mint program, each matched to mint ownership/policy. |
| Preconditions | Redeem is permitted by protocol emergency policy and market status; every asset `redeem_enabled`; valid pre-redeem supply; routes/pricing current as required; custody is non-aliased; required minimum present. |
| Validation rules | Snapshot pre-redeem supply/reserves and USDC settlement balance; compute pro-rata reserve amounts using pre-burn supply; never unwind more than those amounts; validate routes/venue/pool/mints/direction on-chain; redeem fee is zero and execution spread is not an Axis fee. |
| Allowed CPI | DTF burn, approved token transfers, and one bounded approved unwind route per asset. No arbitrary/split/unsupported multi-hop execution. |
| State changes | On success only: DTF supply decreases by input; reserve balances decrease according to actual unwind within pro-rata bounds. Creator/protocol accrual remains unchanged. |
| Token-balance changes | User DTF and total supply decrease by input; reserve vaults decrease no more than computed amounts; settlement USDC increases by observed venue outputs; user USDC increases by actual payable USDC; fee vault is unchanged. |
| Actual-balance-delta checks | Snapshot before execution. Each reserve input delta must be nonpositive, expected, and no larger than its computed pro-rata amount; settlement `actual_usdc_received = post - pre > 0`; user USDC increase equals actual payout; `actual_usdc_received >= min_usdc_out`; fee-vault and fee-counter deltas are zero. Quote mismatch is harmless only if observed checks and minimums pass. |
| Postconditions | `user_usdc_out = actual_usdc_received`; no explicit creator/protocol redeem fee; no partial burn, reserve transfer, or payout survives failure. |
| Failure conditions | Wrong DTF mint/program/owner; zero/excess input; unsafe paused state; disabled asset; stale/unapproved route; wrong venue/pool/mints; pro-rata or price-impact failure; minimum failure; delta mismatch; fee-vault alias; arithmetic/rounding failure. |
| Expected error/rejection | Static `AUTH`/`OWNER`/`TOKEN`/`STATE`/`POLICY`/`ROUTE`/`FEE` before CPI; `SLIPPAGE`/`DELTA`/`ARITH` after observation and before commit. |
| Unresolved questions | Deterministic rounding/dust, burn timing, settlement-account design, paused/deprecated exit rules, pricing required for redeem, route freshness, token policy, and adapter ABI. |

Fee claim is boundary-level only in this proposal. The following sections preserve the
required authorization, reserve-separation, and observed-transfer invariants without
selecting a fee account implementation. P0-FEE-07 owns the final custody layout,
counter layout, partial-versus-all claim behavior, and per-market-versus-shared fee
custody decision.

### 8.17 `claim_creator_fee`

| Field | Proposal |
|---|---|
| Intent | Transfer a claimable creator fee from separate USDC fee custody to the market's configured creator fee destination. |
| Inputs | Claim request defined by P0-FEE-07. This proposal does not choose partial-versus-all behavior or its encoding. |
| Required signer(s) | Authority defined for the immutable creator fee destination by P0-FEE-07. |
| Authority checks | Signer and destination must match the market's stored claim authorization; creator identity alone is insufficient if the configured destination authority differs. |
| Account roles | Protocol config; market fee boundary; fee custody/accounting roles defined by P0-FEE-07; configured creator destination token account; configured USDC mint/program; reserve vaults read-only only if needed to prove non-aliasing. |
| Writable accounts | Only the fee accounting/custody roles selected by P0-FEE-07 and the creator destination. Reserve vaults must never be writable. |
| Expected token programs | Configured USDC token program only. |
| Preconditions | A positive creator entitlement is claimable under P0-FEE-07; selected custody is sufficient; destination is a valid USDC account; fee custody is not any reserve vault. |
| Validation rules | Fulfilled claim cannot exceed the creator entitlement or selected custody; protocol entitlement is unchanged; no destination substitution; replay/double claim is prevented under the P0-FEE-07 counter model. |
| Allowed CPI | Configured USDC token transfer as specified by P0-FEE-07. No venue CPI. |
| State changes | P0-FEE-07 accounting records reduce the creator entitlement by the exact fulfilled claim. Final counter layout is not defined here. |
| Token-balance changes | Selected fee custody decreases and creator destination increases by the same fulfilled amount; reserve balances and DTF supply are unchanged. |
| Actual-balance-delta checks | Observe equal selected-custody decrease and destination increase; account for only token-program behavior permitted by policy; assert zero reserve and supply delta. |
| Postconditions | Creator entitlement is reduced exactly once under P0-FEE-07; protocol entitlement and reserves are unchanged. |
| Failure conditions | Missing/wrong signer; wrong destination/mint/program; zero/excess claim; insufficient custody; fee/reserve alias; transfer or observed-delta mismatch. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN`/`FEE` before CPI; `DELTA` after transfer observation. |
| Unresolved questions | P0-FEE-07 owns the final custody layout, counter layout, partial-versus-all behavior, per-market-versus-shared custody decision, instruction name, destination authority model, and rounding rules. |

### 8.18 `claim_protocol_fee`

| Field | Proposal |
|---|---|
| Intent | Transfer claimable protocol fees from separate USDC fee custody to the configured protocol treasury. |
| Inputs | Claim request defined by P0-FEE-07. This proposal does not choose partial-versus-all behavior, a per-market claim, or a multi-market sweep. |
| Required signer(s) | Protocol treasury authority defined by P0-FEE-07/protocol config. |
| Authority checks | Signer and destination must match current approved treasury authorization; admin authority is not automatically the treasury authority. |
| Account roles | Protocol config; market fee boundary; fee custody/accounting roles defined by P0-FEE-07; configured protocol treasury USDC account; configured USDC mint/program; reserve vaults read-only only if needed for non-alias proof. |
| Writable accounts | Only the fee accounting/custody roles selected by P0-FEE-07 and the treasury destination. Reserve vaults must never be writable. |
| Expected token programs | Configured USDC token program only. |
| Preconditions | A positive protocol entitlement is claimable under P0-FEE-07; selected custody is sufficient; treasury is a valid USDC destination; no reserve alias exists. |
| Validation rules | Fulfilled claim cannot exceed the protocol entitlement or selected custody; creator entitlement is unchanged; destination cannot be caller-substituted; replay/double claim is prevented under the P0-FEE-07 counter model. |
| Allowed CPI | Configured USDC token transfer as specified by P0-FEE-07. No venue CPI. |
| State changes | P0-FEE-07 accounting records reduce the protocol entitlement by the exact fulfilled claim. Final counter layout is not defined here. |
| Token-balance changes | Selected fee custody decreases and treasury increases by the same fulfilled amount; reserves and DTF supply are unchanged. |
| Actual-balance-delta checks | Observe equal selected-custody decrease and treasury increase; assert zero reserve/supply delta. |
| Postconditions | Protocol entitlement is reduced exactly once under P0-FEE-07; creator entitlement and reserve accounting are unchanged. |
| Failure conditions | Missing/wrong treasury signer; wrong destination/mint/program; zero/excess claim; insufficient custody; alias; delta mismatch. |
| Expected error/rejection | `AUTH`/`OWNER`/`TOKEN`/`FEE` before CPI; `DELTA` after transfer observation. |
| Unresolved questions | P0-FEE-07 owns the final custody layout, counter layout, partial-versus-all behavior, per-market-versus-shared custody decision, per-market-versus-multi-market claim behavior, instruction name, treasury authority model, and rounding rules. |

## 9. Account role matrix

Legend: `R` read-only, `W` writable, `S` signer, `C` created/initialized, `V` optional
venue-specific role after approval, and `—` absent. Account order is intentionally not
defined.

| Instruction family | Protocol config | Market | Asset/policy | Pricing | Route | User source/dest | DTF mint | Reserve vault | Fee vault/state | Token programs | Venue program/accounts |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Initialize/update protocol | W | — | — | — | — | payer `S/W` on init | — | — | embedded/boundary | R when validating USDC | — |
| Protocol pause | W | — | — | — | — | — | — | — | — | — | — |
| Asset upsert/flags | R | — | W/C | — | — | payer `S/W` if create | — | — | — | mint owner R | — |
| Pricing upsert/enable | R | — | R | W/C | — | payer `S/W` if create | — | — | — | — | source refs R |
| Route upsert/enable | R | — | R | — | W/C | payer `S/W` if create | — | — | — | mint owners R | venue identities R |
| Create market | R | W/C | R | R | R if activation gate | creator/payer `S/W` | W/C | W/C | W/C | R | — |
| Activate/pause/deprecate market | R | W | R as needed | R as needed | R as needed | — | R | R | R | R as needed | — |
| Mint | R | W | R | R | R | `S/W` | W | W | W | R | program R; adapter accounts `V/W` |
| Redeem | R | W | R | R if required | R | `S/W` | W | W | R and never fee-accrual W | R | program R; adapter accounts `V/W` |
| Fee claim | R | W fee counter only | — | — | — | recipient `S/W` | R | R/never W | W | configured USDC R | — |

## 10. Signer and authority matrix

| Instruction | User | Creator | Admin/config | Pause | Asset policy | Pricing | Route | Creator fee recipient | Protocol treasury |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `initialize_protocol_config` | — | — | S (initial) | — | — | — | — | — | — |
| `update_protocol_config` | — | — | S | — | — | — | — | — | — |
| `update_protocol_fee_config` | — | — | S | — | — | — | — | — | — |
| `set_protocol_paused` | — | — | — | S | — | — | — | — | — |
| `upsert_asset_config` / flags | — | — | — | — | S | — | — | — | — |
| `upsert_pricing_source` / enabled | — | — | — | — | — | S | — | — | — |
| `upsert_approved_route` / enabled | — | — | — | — | — | — | S | — | — |
| `create_market` | — | S | — | — | — | — | — | — | — |
| `activate_market` / `deprecate_market` | — | — | S | — | — | — | — | — | — |
| `set_market_paused` | — | — | — | S | — | — | — | — | — |
| `mint_dtf_with_usdc` / `redeem_dtf_for_usdc` | S | — | — | — | — | — | — | — | — |
| `claim_creator_fee` | — | — | — | — | — | — | — | S | — |
| `claim_protocol_fee` | — | — | — | — | — | — | — | — | S |

Equal public keys across configured roles do not collapse the semantic authority checks.
The program still checks the authority appropriate to the instruction.

## 11. CPI permission matrix

| Instruction family | System Program | Configured token program | Approved venue program | Oracle/pricing CPI | Arbitrary CPI |
|---|---:|---:|---:|---:|---:|
| Initialize/upsert state | Account creation only | No | No | No | No |
| Config, flags, pause, lifecycle | No | No | No | No | No |
| Create market | Account creation | Mint/vault initialization only | No | No | No |
| Mint | No except separately justified ATA creation | Transfer + DTF mint | Yes, current approved route/adapter only | No by this proposal | No |
| Redeem | No except separately justified ATA creation | DTF burn + transfers | Yes, current approved route/adapter only | No by this proposal | No |
| Fee claim | No | One configured USDC transfer | No | No | No |

Production venue CPI remains unimplemented and unencoded. A controlled adapter may prove
P0 accounting, but it does not establish mainnet venue readiness.

## 12. State transition matrix

| State/object | Instruction | From | To | Guard |
|---|---|---|---|---|
| Protocol config | `initialize_protocol_config` | Missing | Initialized/Active or Initialized/Paused | Unique PDA and valid authorities/config |
| Protocol safety | `set_protocol_paused(true)` | Active | Paused | Pause authority |
| Protocol safety | `set_protocol_paused(false)` | Paused | Active | Pause authority plus unresolved recovery policy |
| Asset policy | `upsert_asset_config` | Missing/existing | Configured/updated | Asset authority; immutable identity |
| Asset flags | `set_asset_execution_flags` | Any valid flags | Any independently valid flags | Exit-only supported |
| Pricing boundary | pricing instructions | Missing/enabled/disabled | Configured/enabled/disabled | Approved subset only |
| Route boundary | route instructions | Missing/enabled/disabled | Configured/enabled/disabled | Route identity/current revision |
| Market | `create_market` | Missing | Created | Current policy and custody validation |
| Market | `activate_market` | Created | Active | Full readiness validation |
| Market | `set_market_paused(true)` | Active | Paused | Pause authority |
| Market | activation/unpause path | Paused | Active | Protocol active and readiness revalidated |
| Market | `deprecate_market` | Created/Active/Paused | Deprecated | Terminal; no P0 close |
| Market | any | Deprecated | Active/Paused/Closed | Forbidden in P0 |
| Supply/reserves/fees | Mint | Valid pre-state | Atomic increased reserves/supply/accrual | Observed deltas and all minimums |
| Supply/reserves | Redeem | Valid pre-state | Atomic reduced supply/reserves and USDC payout | Pro-rata bounds and observed deltas |
| Fee accrual | Claim | Positive accrued amount | Reduced by exact claim | Separate custody; reserves unchanged |

## 13. Actual-balance-delta checks

| Instruction | Pre snapshot | Required observed delta | Accounting use | Rejection |
|---|---|---|---|---|
| Config/admin/lifecycle | None | No token/supply balance change | None | Any unexpected writable token mutation is `DELTA`/`OWNER` |
| `create_market` | New mint/vault balances | Zero initial supply/reserve/fee balance unless separately specified | Establishes clean custody baseline | Prefunded/aliased/unexpected balance is rejected |
| Mint: user USDC | User source | Decrease equals authorized gross input | Gross input and fee base | Wrong direction/amount rejects |
| Mint: fee vault | Fee vault | Increase equals canonical mint fee | Separate fee custody/accrual | Alias or mismatch rejects |
| Mint: reserves | Every reserve vault | Positive `post - pre`, each `>= min_out_i` | Actual added reserve value | Quote cannot substitute; mismatch rejects |
| Mint: DTF | Mint supply and user destination | Equal increases matching computed output | Final DTF amount | Supply/destination mismatch rejects |
| Redeem: DTF | Supply and user source | Equal decreases matching `dtf_amount_in` | Redeem share uses pre-burn supply | Mismatch rejects atomically |
| Redeem: reserves | Every reserve vault | Expected decrease, never above computed pro-rata amount | Reserve accounting truth | Excess/wrong-direction delta rejects |
| Redeem: USDC | Settlement and user destination | Settlement increase determines actual received; user increase equals payout | `user_usdc_out`, checked against `min_usdc_out` | Quote cannot substitute; mismatch rejects |
| Redeem: fees | Fee vault and counters | Exactly zero | Confirms launch redeem fee is zero | Any fee delta rejects |
| Fee claim | Fee vault, recipient, reserves, supply | Vault decrease = recipient increase = claim; reserve/supply delta zero | Decrement matching accrued claim | Alias, short transfer, reserve, or supply delta rejects |

## 14. Scenario matrix for tests and specification review

Legend: `✓` required success path; codes are expected rejection classes; `—` is not
applicable. `Q≠A/✓` means a backend quote may differ from actual output and the
instruction still succeeds only if all on-chain policy, observed-delta, and minimum
checks pass.

| Instruction | Success | Missing signer | Wrong owner/program | Stale route | Unapproved route | Disabled asset | Protocol paused | Market paused | Insufficient actual output | Minimum-output failure | Fee vault aliases reserve | Wrong token program | Wrong mint | Backend quote mismatch | Actual delta mismatch |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `initialize_protocol_config` | ✓ | `AUTH` | `OWNER` | — | — | — | — | — | — | — | — | `TOKEN` | `TOKEN` | — | — |
| `update_protocol_config` | ✓ | `AUTH` | `OWNER` | — | — | — | policy-dependent `STATE` | — | — | — | — | — | — | — | — |
| `update_protocol_fee_config` | ✓ | `AUTH` | `OWNER` | — | — | — | policy-dependent `STATE` | — | — | — | — | — | — | — | — |
| `set_protocol_paused` | ✓ | `AUTH` | `OWNER` | — | — | — | ✓ for unpause/pause idempotence policy | — | — | — | — | — | — | — | — |
| `upsert_asset_config` | ✓ | `AUTH` | `OWNER` | — | — | — | disable allowed; enable/update `STATE` | — | — | — | — | `TOKEN` | `TOKEN` | — | — |
| `set_asset_execution_flags` | ✓ | `AUTH` | `OWNER` | — | — | ✓ (including disabling) | disable allowed; re-enable `STATE` | — | — | — | — | — | — | — | — |
| `upsert_pricing_source` | ✓ after subset approval | `AUTH` | `OWNER` | — | — | `POLICY` if binding invalid | non-safety update policy | — | — | — | — | — | wrong bound mint `OWNER` | — | — |
| `set_pricing_source_enabled` | ✓ after subset approval | `AUTH` | `OWNER` | — | — | `POLICY` if binding invalid | disable allowed; re-enable policy | — | — | — | — | — | wrong bound mint `OWNER` | — | — |
| `upsert_approved_route` | ✓ | `AUTH` | `OWNER` | `ROUTE` | `ROUTE` | `POLICY` | disable-only policy | — | — | — | — | `TOKEN` | `TOKEN` | quote cannot approve `ROUTE` | — |
| `set_approved_route_enabled` | ✓ | `AUTH` | `OWNER` | `ROUTE` | `ROUTE` | — | disable allowed; re-enable policy | — | — | — | — | — | — | — | — |
| `create_market` | ✓ | `AUTH` | `OWNER` | `ROUTE` if required | `ROUTE` if required | `POLICY` | `STATE` | — | — | — | `FEE` | `TOKEN` | `TOKEN` | advisory only | `DELTA` |
| `activate_market` | ✓ | `AUTH` | `OWNER` | `ROUTE` | `ROUTE` | `POLICY` | `STATE` | readiness path only | — | — | `FEE` | `TOKEN` | `TOKEN` | advisory only | `DELTA` if balances changed |
| `set_market_paused` | ✓ | `AUTH` | `OWNER` | — | — | — | pause allowed; unpause `STATE` | ✓ | — | — | — | — | — | — | — |
| `deprecate_market` | ✓ | `AUTH` | `OWNER` | — | — | — | policy-defined but safety action allowed | ✓ | — | — | — | — | — | — | — |
| `mint_dtf_with_usdc` | ✓ | `AUTH` | `OWNER` | `ROUTE` | `ROUTE` | `POLICY` | `STATE` | `STATE` | `SLIPPAGE`/`DELTA` | `SLIPPAGE` | `FEE` | `TOKEN` | `TOKEN` | `Q≠A/✓` | `DELTA` |
| `redeem_dtf_for_usdc` | ✓ | `AUTH` | `OWNER` | `ROUTE` | `ROUTE` | `POLICY` | exit-policy `STATE` | exit-policy `STATE` | `SLIPPAGE`/`DELTA` | `SLIPPAGE` | `FEE` | `TOKEN` | `TOKEN` | `Q≠A/✓` | `DELTA` |
| `claim_creator_fee` | ✓ | `AUTH` | `OWNER` | — | — | — | claim policy must be explicit | — | insufficient custody `FEE` | — | `FEE` | `TOKEN` | `TOKEN` | — | `DELTA` |
| `claim_protocol_fee` | ✓ | `AUTH` | `OWNER` | — | — | — | claim policy must be explicit | — | insufficient custody `FEE` | — | `FEE` | `TOKEN` | `TOKEN` | — | `DELTA` |

Additional mandatory all-or-nothing assertions:

- a failed mint leaves user USDC, reserves, DTF supply, fee custody, and accrual exactly
  at their pre-transaction state;
- a failed redeem leaves user DTF, supply, reserves, fee state, and user USDC exactly at
  their pre-transaction state;
- a failed claim leaves fee custody, accrual, recipient, reserves, and supply unchanged;
- an optimistic or pessimistic quote never changes accounting independently of actual
  observed deltas; and
- wrong or duplicated writable accounts, including the same account supplied for fee
  and reserve roles, fail before CPI.

## 15. Review invariants

Approval requires agreement on these invariants:

1. Route-builder/backend output may be instruction data, but it is never protocol
   accounting truth.
2. Runtime policy and route approval are checked on-chain against current state.
3. Mint fee is calculated from gross USDC, separated first, and only net USDC composes
   reserves.
4. Minted DTF derives from actual added reserve value, never a quote or gross input.
5. Redeem output is actual observed USDC and launch redeem fee is zero.
6. Reserve custody, fee custody, and fee claims remain separate accounting domains.
7. Fee claims are separate from mint/redeem reserve transitions and defer to P0-FEE-07.
8. Only mint/redeem may invoke approved venue adapters; no arbitrary CPI exists.
9. Protocol, market, asset, pricing, and route safety controls reject before execution.
10. Exit-only behavior is preferred when redeem remains safe, but it never bypasses
    route, pricing, token, minimum-output, or balance-delta checks.

## 16. Unresolved questions

The following block ABI/implementation work but do not block review of the semantic
surface:

1. What exact account layouts, PDA seeds, serialization, and account-meta ordering will
   be approved from/following PR #57?
2. Is `ApprovedRoute` route-, asset-pair-, venue-, or pool-granular, and how are
   revision/freshness and updates represented?
3. Which `PricingSource` variants and source-specific accounts form the P0 subset?
4. What fixed-point, decimals-normalization, rounding, and dust rules apply to NAV,
   fee math, pro-rata redemption, and actual-value conversion?
5. Is Token-2022 approved for the DTF mint, and which extensions are permitted?
6. What are exact protocol-paused, market-paused, and deprecated-market redeem rules?
7. Does `activate_market` own all reactivation, or may
   `set_market_paused(false)` reactivate after equivalent validation?
8. Must route/pricing readiness be complete at market creation or only at activation?
9. What controlled-adapter ABI proves P0 accounting without becoming a production venue
   encoding?
10. Per P0-FEE-07, is fee custody per-market or shared, are claims partial or “all,” and
    are creator/protocol instructions separate or unified?
11. What authority rotations, re-enables, and unpauses require multisig, timelock, or
    dual authorization?
12. What event/log schema is required for administration, execution observations, fee
    accrual, and claims?

Until resolved, implementations must not infer permissive defaults.
