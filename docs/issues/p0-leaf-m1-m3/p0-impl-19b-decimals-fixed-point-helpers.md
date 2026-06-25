# [P0-IMPL-19B] Implement decimals normalization and fixed-point helpers / 小数桁正規化と固定小数点ヘルパーを実装

## Parent Issue / 親Issue

P0-IMPL-19

## Summary / 概要

Implement deterministic helpers for decimals normalization, fixed-point arithmetic, and safe value calculations.

## Requirement Areas / 関連Requirement Area

- PRICE-001, PRICE-012
- MINT-011 to MINT-013
- NFR-REL-004

## Source Documents / 参照ドキュメント

- `requirements/06-pricing-nav-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Normalize token amounts across asset decimals.
- Implement fixed-point multiplication/division helpers for amount, price, NAV, and DTF quantity calculations.
- Define overflow, rounding, and checked math behavior for P0.

## Out of Scope / 対象外

- Final mainnet dust policy if not accepted yet.
- Production pricing source implementation.
- Redeem formula finalization.

## Acceptance Criteria / 完了条件

- Helpers use checked math.
- Decimals normalization is deterministic.
- Rounding direction is documented.
- Helpers are usable by mint amount calculation.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-19D.

## Dependencies / 依存関係

- P0-IMPL-19A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `math`
- `fixed-point`
- `pricing`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

Rounding and dust handling should be explicit rather than accidental.
