# [P0-IMPL-12D] Add DTFMarket serialization and PDA tests / DTFMarketのシリアライズとPDAテストを追加

## Parent Issue / 親Issue

P0-IMPL-12

## Summary / 概要

Add focused tests for DTFMarket layout, PDA derivation, status fields, creator fields, and immutable market fee config.

## Requirement Areas / 関連Requirement Area

- DTF-010 to DTF-017
- FEE-001 to FEE-005
- NFR-DOC-002

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Test market serialization/deserialization.
- Test PDA derivation and wrong PDA failure.
- Test status field representation.
- Test creator and fee destination field persistence.
- Test immutable market fee config storage.

## Out of Scope / 対象外

- Full create_market instruction tests.
- Mint/redeem tests.
- Fee claim tests.

## Acceptance Criteria / 完了条件

- DTFMarket tests pass deterministically.
- Wrong market PDA fails.
- Market can represent required fields for M2 and M3.
- Fee config fields are stored without making fees reserves.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-12A through P0-IMPL-12C.

## Dependencies / 依存関係

- P0-IMPL-12A.
- P0-IMPL-12B.
- P0-IMPL-12C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `dtf-market`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Keep the market account indexer-friendly, but on-chain state remains protocol truth.
