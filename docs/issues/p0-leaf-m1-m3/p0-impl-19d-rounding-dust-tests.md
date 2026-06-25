# [P0-IMPL-19D] Add rounding and dust tests / 丸めとdustテストを追加

## Parent Issue / 親Issue

P0-IMPL-19

## Summary / 概要

Add deterministic tests for decimals normalization, fixed-point math, mint amount calculation, rounding, and dust behavior.

## Requirement Areas / 関連Requirement Area

- MINT-011 to MINT-013
- PRICE-001, PRICE-002, PRICE-012
- FEE-017
- PMV-004

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/06-pricing-nav-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Test decimals normalization across mixed decimals.
- Test initial NAV calculation.
- Test subsequent pre-trade NAV calculation.
- Test actual delta value conversion.
- Test rounding boundaries and dust handling.
- Test fee exclusion from NAV and actual added value.

## Out of Scope / 対象外

- Production oracle tests.
- Full create_market + mint E2E, which remains under parent P0-IMPL-25.
- Redeem math tests.

## Acceptance Criteria / 完了条件

- Math tests pass deterministically.
- Rounding behavior is explicit and reproducible.
- Fee amounts are excluded from NAV.
- Quote-only estimates cannot affect minted DTF amount.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-19A through P0-IMPL-19C.

## Dependencies / 依存関係

- P0-IMPL-19A.
- P0-IMPL-19B.
- P0-IMPL-19C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `math`
- `mint`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

These tests should be reviewed before expanding M4 redeem math leaves.
