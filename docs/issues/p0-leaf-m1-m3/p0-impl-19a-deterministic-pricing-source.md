# [P0-IMPL-19A] Define deterministic P0 pricing source for tests / テスト用の決定的P0価格ソースを定義

## Parent Issue / 親Issue

P0-IMPL-19

## Summary / 概要

Define a deterministic P0 pricing source for local tests without claiming production oracle readiness.

## Requirement Areas / 関連Requirement Area

- PRICE-001 to PRICE-004
- PRICE-010, PRICE-012
- MINT-015

## Source Documents / 参照ドキュメント

- `requirements/06-pricing-nav-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Define ManualFixedPrice or equivalent deterministic test pricing path.
- Define required asset price fields and staleness/enable flags if used.
- Ensure pricing source is used only for protocol accounting tests in P0.

## Out of Scope / 対象外

- Production oracle integration.
- DEX spot/TWAP production pricing.
- StockToken pricing.
- Secondary-market pool prices.

## Acceptance Criteria / 完了条件

- P0 test pricing source is deterministic.
- Missing/disabled price can fail mint accounting.
- UI/backend prices are not protocol accounting truth.
- Documentation states this is not production pricing readiness.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-19D and later mint E2E tests.

## Dependencies / 依存関係

- P0-IMPL-11A.
- P0-IMPL-18B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `pricing`
- `tests`
- `nav`

## Suggested Owner / 推奨Owner

ADP design review, Toby implementation.

## Notes / 補足

This is a controlled P0 pricing path. It is not production Orca/Raydium or oracle readiness.
