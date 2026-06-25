# [P0-IMPL-18C] Validate USDC source drain and min_out constraints / USDCソース減少とmin_out制約を検証

## Parent Issue / 親Issue

P0-IMPL-18

## Summary / 概要

Validate that mint execution drains only the intended USDC amounts and satisfies min_out constraints.

## Requirement Areas / 関連Requirement Area

- MINT-009, MINT-010, MINT-014
- EXEC-006 to EXEC-009
- NFR-REL-001 to NFR-REL-004

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Snapshot user or execution source USDC balance before and after controlled execution.
- Enforce source drain does not exceed intended net allocation plus approved movement.
- Enforce each mint leg has nonzero/minimum output protection.
- Ensure failed min_out reverts the parent mint.

## Out of Scope / 対象外

- Redeem `min_usdc_out`.
- Production venue source-drain validation.
- Fee claim custody.

## Acceptance Criteria / 完了条件

- Source USDC drain is bounded.
- Missing min_out fails unless explicitly test-only behavior is approved.
- Output below min_out fails full mint.
- Failed mint does not accrue fees or mint DTF.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-18D.

## Dependencies / 依存関係

- P0-IMPL-16C.
- P0-IMPL-18A.
- P0-IMPL-18B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `min-out`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Post-CPI source-drain bounds are a reusable safety pattern, not legacy product-model reuse.
