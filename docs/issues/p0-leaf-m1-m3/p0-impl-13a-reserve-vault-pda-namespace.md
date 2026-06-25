# [P0-IMPL-13A] Define reserve vault PDA namespace / リザーブVault PDA名前空間を定義

## Parent Issue / 親Issue

P0-IMPL-13

## Summary / 概要

Define deterministic PDA namespace and seed rules for market reserve vaults.

## Requirement Areas / 関連Requirement Area

- DTF-010
- NFR-SEC-004
- PRICE-001

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/06-pricing-nav-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Define reserve vault PDA seeds per market and asset.
- Define expected token account authority model.
- Document how reserve vaults are distinguished from fee custody and venue accounts.

## Out of Scope / 対象外

- Token account validation implementation.
- Fee vault namespace.
- Production venue vault validation.

## Acceptance Criteria / 完了条件

- Reserve vault PDA is deterministic for market + asset.
- Namespace cannot collide with fee custody by design.
- PDA authority model is clear enough for validation and tests.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-13C.

## Dependencies / 依存関係

- P0-IMPL-12B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `reserves`
- `pda`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

Reserve vaults are DTF backing. Fee custody and external pools are not reserve backing.
