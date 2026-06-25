# [P0-IMPL-16C] Implement gross USDC to fee/net split interface / gross USDCからfee・net分離インターフェースを実装

## Parent Issue / 親Issue

P0-IMPL-16

## Summary / 概要

Implement the interface that splits gross USDC input into mint fee and net USDC for reserve composition.

## Requirement Areas / 関連Requirement Area

- MINT-003 to MINT-006
- MINT-017, MINT-018
- FEE-006 to FEE-008
- POLICY-006

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/07-execution-policy-risk-controls.md`
- `requirements/13-fee-model-requirements.md`

## Scope / 対象範囲

- Calculate `mint_fee_usdc`.
- Calculate creator/protocol fee split from immutable market fee config.
- Calculate `net_usdc_for_composition`.
- Calculate per-asset net USDC allocation from target weights.
- Enforce hard minimum allocation and max trade policy.
- Expose result to later controlled execution code.

## Out of Scope / 対象外

- Actual fee custody transfer/accrual finalization.
- Reserve CPI execution.
- Final DTF mint amount calculation.

## Acceptance Criteria / 完了条件

- 1000 USDC with 1% fee yields 10 fee and 990 net composition.
- 40/60 creator/protocol split yields 4 and 6 USDC from a 10 USDC fee.
- Asset allocation uses net USDC, not gross input.
- Below-min allocation fails.
- Above max trade allocation fails.
- Fees are not treated as reserves.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-16D and later mint path tests.

## Dependencies / 依存関係

- P0-IMPL-11C.
- P0-IMPL-12C.
- P0-IMPL-16B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `fees`
- `accounting`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

This interface prepares values for execution; it does not make target weights protocol reserve truth.
