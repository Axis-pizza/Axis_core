# [P0-IMPL-12C] Implement DTFMarket status and creator fields / DTFMarketステータスとクリエイター項目を実装

## Parent Issue / 親Issue

P0-IMPL-12

## Summary / 概要

Implement market status and creator-related fields needed by create_market, mint, redeem, and fee accounting.

## Requirement Areas / 関連Requirement Area

- DTF-012 to DTF-017
- FEE-001 to FEE-005
- MINT-002
- REDEEM-002

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/13-fee-model-requirements.md`

## Scope / 対象範囲

- Implement status values: Created, Active, Paused, Deprecated.
- Store creator and creator fee destination.
- Store immutable market-level fee config copied from ProtocolConfig.
- Expose validation helpers for status and creator fields.

## Out of Scope / 対象外

- Pause instruction implementation.
- Fee claim instruction implementation.
- Secondary-market display metadata.

## Acceptance Criteria / 完了条件

- Market status can block mint when paused/deprecated.
- Creator and fee destination are stored and immutable after creation.
- Creator cannot customize fee bps per market.
- Market fee config is readable for mint accounting.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-12D and P0-IMPL-15 tests.

## Dependencies / 依存関係

- P0-IMPL-12A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `dtf-market`
- `fees`
- `status`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Creator fields support fee accounting and partner/demo context, not investment advice or off-chain accounting truth.
