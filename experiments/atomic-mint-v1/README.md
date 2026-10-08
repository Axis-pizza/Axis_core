# HYP-MINT-011: atomic Mint v1 experiment

> EXPERIMENT — not Axis Core production code and not a frozen ABI.

This directory tests the transaction-envelope part of HYP-MINT-011 without
putting speculative Jupiter integration into `programs/axis-core`.

It answers two separate questions:

1. Can the proposed delivery-driven Mint/Redeem account shape fit Solana's
   64-account-lock and 1232-byte signed-transaction limits?
2. What does the already implemented Axis_AMM split fallback prove, and which
   properties does it give up?

It also builds two experiment-only SBF programs: a deterministic direct-route
adapter and a minimal HYP-MINT-011 handler. LiteSVM runs their real CPI path and
reports compute units while checking that failed post-CPI validation rolls all
state back.

## Reused Axis_AMM evidence

By default the runner reads the sibling `Axis_AMM` checkout and replays its
recorded Jupiter V6 SOL-to-USDC route:

- `test/fixtures/jupiter/sol-usdc-100m.json`
- `test/fixtures/jupiter/accounts/<ALT>.json`

The runner uses the captured Jupiter instruction data, account flags, and real
ALT contents. It replaces the fixture's placeholder user with a deterministic
signer, compiles legacy and v0 messages, signs them, and measures the serialized
wire bytes.

It measures four envelopes:

- Axis_AMM split tx0: Jupiter swap only.
- HYP-MINT-011 top-level atomic: Jupiter swap followed by Core verification and
  minting in the same transaction.
- HYP-MINT-011 Core CPI atomic: the captured route accounts and route bytes are
  passed through the Core instruction.
- Axis_AMM split tx1: Core verification and minting only.

The recorded route is one real route, not a representative N=3 sample. It is a
ground-truth packet fixture, not proof that every Jupiter route or every
three-asset basket fits.

This matches the corrected direction of Axis_docs PR #10: maximum three assets
is a product ceiling, not an execution guarantee. Its corrected N=3 sampling
reported 61–67 runtime locks with 4 of 11 clean any-route samples over the
64-lock cap. That distribution governs support decisions; the one captured
Axis_AMM route here is a reproducible fixture for transaction construction.

## Controlled N=2/N=3 projection

The second table retains synthetic `route-accounts` and `route-data-bytes`
controls. The same controls are applied to both candidates, so the comparison
isolates the Core account model:

- `delivery-inline`: inline constituent state; no `ProtocolConfig`,
  `MarketAssetWeight`, `PricingSource`, NAV, or oracle account on Mint/Redeem.
- `nav-external`: adds one `ProtocolConfig`, one `MarketAssetWeight` per asset,
  and one `PricingSource` per asset.
- `redeem-in-kind-sponsored`: no venue or pricing accounts; includes distinct
  holder and fee-payer signers plus the ATA/System programs and per-asset
  destination, mint, and reserve vault.

Synthetic FIT results are capacity projections only. They are not Jupiter
production-route evidence.

### Provisional Core base-account manifest

The controlled projection derives the delivery-inline Mint/Redeem base as
`11 + N` resolved locks before variable route accounts:

| Role | Count |
| --- | ---: |
| user/payer signer | 1 |
| source USDC token account | 1 |
| destination DTF token account | 1 |
| `DTFMarket` | 1 |
| DTF mint | 1 |
| fee vault | 1 |
| reserve vaults | N |
| Axis Core program ID | 1 |
| legacy SPL Token program ID | 1 |
| Token-2022 program ID | 1 |
| route/aggregator program ID | 1 |
| Compute Budget program ID | 1 |
| **Total before variable route accounts** | **11 + N** |

`delivery_inline_core_base_is_eleven_plus_n` constructs the signed-transaction
instruction set for N=2 and N=3 and verifies that count after pubkey
deduplication. `nav_external_reversal_costs_one_plus_two_n_accounts` separately
pins the cost of reversing the three cuts: one `ProtocolConfig` plus one
`MarketAssetWeight` and one `PricingSource` per constituent.

This manifest is a provisional interface target, not a production ABI. The
production Mint/Redeem handlers and their identity and authorization checks do
not exist yet; those handlers must reconcile their actual account list against
this manifest before the interface is certified.

## Run

From the Axis_core repository root:

```bash
./experiments/atomic-mint-v1/run.sh
```

That command builds the experiment-only SBF artifacts before running the
measurements. Set `AXIS_SKIP_SBF_BUILD=1` only when reusing already-built
artifacts locally.

Default unit tests cover account-model arithmetic and signed transaction
construction without requiring an SBF artifact:

```bash
cargo test --manifest-path experiments/atomic-mint-v1/Cargo.toml
```

After `build-programs.sh`, enable the SBF-only rollback test explicitly:

```bash
cargo test --manifest-path experiments/atomic-mint-v1/Cargo.toml \
  --features sbf-integration
```

For a non-sibling Axis_AMM checkout:

```bash
./experiments/atomic-mint-v1/run.sh \
  --axis-amm /path/to/Axis_AMM
```

To change only the controlled N=2/N=3 projection:

```bash
./experiments/atomic-mint-v1/run.sh \
  --route-accounts 40 \
  --route-data-bytes 256
```

## Current result and boundary

The checked-in Axis_AMM route fits as a single v0 transaction both when the
Jupiter instruction is top-level and when its route is carried through the
experimental Core CPI envelope. The split halves also fit independently.

The controlled SBF path verifies the delivery-driven state transition itself:

- exact required delivery commits reserve, USDC, supply, and user-DTF changes;
- one unit of under-delivery fails after CPI and rolls every account back;
- spending one unit above `max_usdc_in` also fails after CPI and rolls every
  account back.

The adapter uses eight locks and roughly 2,000 CU. That CU number measures the
Axis validation skeleton plus the tiny controlled adapter; it must not be used
as a Jupiter CU estimate.

The experiment intentionally omits production authorization and identity
binding (approved adapter ID, market/reserve binding, Token-2022 mint authority,
and fee custody). Those belong to the frozen Core interface after the hypothesis
is accepted; this adapter must never be deployed or copied as a production
handler.

This supports keeping Axis_AMM's policy as a client-side fallback:

1. attempt the atomic transaction;
2. if it exceeds the packet limit, offer two recoverable transactions;
3. label the second path non-atomic and keep intermediate basket assets in the
   user's wallet.

It does **not** establish that split mode satisfies an Axis Core requirement
that Mint be all-or-nothing. Split mode deliberately changes the failure state:
after tx0 succeeds and tx1 is aborted or fails, the user owns basket assets but
has not received DTF tokens.

The Axis_AMM repository also contains a Jupiter V6 program binary and mainnet
account dumps, but its fork runbook says the real Jupiter CPI happy-path test
still needs to be wired; the existing fork test used attestation mode at that
point. Therefore this experiment does not claim CPI execution or CU
certification. Closing that evidence chain requires a real fork replay (or
equivalent production-venue environment), not another packet-size estimate.
