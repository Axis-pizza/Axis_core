# [P0-IMPL-19C] Implement DTF mint amount calculation from observed deltas / 観測差分からDTFミント量を計算

## Parent Issue / 親Issue

P0-IMPL-19

## Summary / 概要

Calculate minted DTF amount from actual reserve deltas and pre-trade NAV.

## Requirement Areas / 関連Requirement Area

- MINT-011 to MINT-013
- PRICE-001, PRICE-002, PRICE-012
- FEE-008, FEE-017
- APPIF-007

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/06-pricing-nav-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`

## Scope / 対象範囲

- Snapshot pre-trade NAV before reserve mutation.
- Use initial NAV = 1 USDC when supply is zero.
- Convert observed reserve deltas to `actual_added_value_usdc` using P0 pricing source.
- Calculate minted DTF amount from actual added value and pre-trade NAV.
- Exclude fees from actual added value and NAV.
- Mint DTF only after all mint validations pass.

## Out of Scope / 対象外

- Production oracle readiness.
- Redeem output formula.
- Fee claim behavior.

## Acceptance Criteria / 完了条件

- Initial mint uses NAV = 1 USDC.
- Subsequent mint uses pre-trade NAV.
- Optimistic quote cannot over-mint.
- Gross/net USDC estimates are not final minted DTF truth.
- Fee custody is excluded from NAV and actual added value.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-19D and later mint E2E.

## Dependencies / 依存関係

- P0-IMPL-18B.
- P0-IMPL-19A.
- P0-IMPL-19B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `nav`
- `accounting`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

Actual observed reserve value is the minting basis; target weights are not reserve truth.
