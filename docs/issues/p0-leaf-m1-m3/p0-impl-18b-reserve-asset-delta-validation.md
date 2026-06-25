# [P0-IMPL-18B] Validate reserve asset deltas after controlled execution / 制御実行後のリザーブ資産差分を検証

## Parent Issue / 親Issue

P0-IMPL-18

## Summary / 概要

Validate actual reserve asset deltas after controlled mint execution.

## Requirement Areas / 関連Requirement Area

- MINT-009, MINT-010, MINT-011
- EXEC-007, EXEC-008
- PRICE-012

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/06-pricing-nav-requirements.md`

## Scope / 対象範囲

- Compute `actual_received_i = post_reserve_balance_i - pre_reserve_balance_i`.
- Require positive expected reserve deltas for mint legs.
- Enforce per-asset `min_out_i`.
- Return actual deltas to mint value calculation.

## Out of Scope / 対象外

- Pricing conversion to USDC value.
- Redeem USDC output deltas.
- Production venue CPI.

## Acceptance Criteria / 完了条件

- Actual reserve delta is accounting truth.
- Quote output cannot satisfy `min_out`.
- Actual received below `min_out` fails.
- Successful deltas are passed to P0-IMPL-19C.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-18D.

## Dependencies / 依存関係

- P0-IMPL-18A.
- P0-IMPL-17C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `balance-delta`
- `accounting`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Reserve deltas, not route estimates, drive minted DTF value.
