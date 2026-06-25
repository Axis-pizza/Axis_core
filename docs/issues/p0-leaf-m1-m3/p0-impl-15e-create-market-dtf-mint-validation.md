# [P0-IMPL-15E] Wire Token-2022 DTF mint validation into create_market / Token-2022 DTFミント検証をcreate_marketへ接続

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Require create_market to validate the market's Token-2022 DTF mint and authority policy.

## Requirement Areas / 関連Requirement Area

- DTF-011
- NFR-SEC-005
- PMV-002

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/10-non-functional-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Call DTF mint validation helper from create_market.
- Enforce Token-2022 DTF mint policy.
- Enforce Axis-controlled mint authority.
- Store DTF mint in market state.

## Out of Scope / 対象外

- DTF mint amount calculation.
- DTF burn path.
- Secondary-market pool setup.

## Acceptance Criteria / 完了条件

- Correct DTF mint passes create_market.
- Wrong mint authority fails.
- Unsupported DTF mint policy fails where implemented.
- Market stores exactly one DTF mint.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-15F and P0-IMPL-15G.

## Dependencies / 依存関係

- P0-IMPL-14B.
- P0-IMPL-15A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `create-market`
- `token-2022`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

This does not create secondary-market liquidity or prove production venue readiness.
