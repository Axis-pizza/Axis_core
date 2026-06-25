# [P0-IMPL-12A] Define DTFMarket account layout / DTFMarketアカウントレイアウトを定義

## Parent Issue / 親Issue

P0-IMPL-12

## Summary / 概要

Define the `DTFMarket` account layout used as the canonical on-chain market identity.

## Requirement Areas / 関連Requirement Area

- DTF-001 to DTF-017
- FEE-001 to FEE-005
- APPIF-001

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Define fields for creator, creator fee destination, DTF mint, asset list, target weights, reserve references or derivation data, market status, and immutable fee config.
- Define capacity for 2 to 5 assets.
- Define versioning, bump, and reserved bytes if needed.

## Out of Scope / 対象外

- `create_market` validation.
- Reserve vault creation.
- Mint/redeem execution.
- Off-chain metadata registry.

## Acceptance Criteria / 完了条件

- Market state can represent 2 to 5 assets.
- Market state stores DTF mint and creator fields.
- Market fee config can be copied from ProtocolConfig and treated as immutable.
- Layout does not make target weights accounting truth.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-12D.

## Dependencies / 依存関係

- P0-IMPL-10A.
- P0-IMPL-11A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `dtf-market`
- `account-layout`

## Suggested Owner / 推奨Owner

ADP design review, Toby implementation.

## Notes / 補足

`DTFMarket` address is the canonical market ID. Strategy registry data is display/indexing only.
