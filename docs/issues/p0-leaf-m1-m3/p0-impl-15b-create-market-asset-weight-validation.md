# [P0-IMPL-15B] Validate create_market asset list and weights / create_marketの資産リストとウェイトを検証

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Implement asset list and target weight validation for `create_market`.

## Requirement Areas / 関連Requirement Area

- DTF-001 to DTF-008
- POLICY-007
- ASSET

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/07-execution-policy-risk-controls.md`
- `requirements/08-asset-universe-requirements.md`

## Scope / 対象範囲

- Enforce 2 to 5 assets.
- Reject duplicate asset mints.
- Enforce total weights equal 10000 bps.
- Enforce each weight at least 100 bps.
- Enforce each weight within asset `max_weight_bps`.
- Require each asset exists and `creation_enabled=true`.

## Out of Scope / 対象外

- Reserve vault validation.
- DTF mint validation.
- Mint/redeem allocation logic.

## Acceptance Criteria / 完了条件

- 1 asset fails.
- 2 and 5 assets pass when all checks pass.
- 6 assets fail.
- Duplicate assets fail.
- Invalid weight sum fails.
- Weight below 100 bps fails.
- Weight above asset max fails.
- Unknown or creation-disabled asset fails.

## Test Requirements / 必要なテスト

- Failure tests are added in P0-IMPL-15G.

## Dependencies / 依存関係

- P0-IMPL-11C.
- P0-IMPL-15A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `create-market`
- `validation`
- `asset-registry`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Target weights define intended composition. They are not actual reserve balances or NAV truth.
