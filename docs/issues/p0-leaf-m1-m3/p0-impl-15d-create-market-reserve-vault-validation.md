# [P0-IMPL-15D] Wire reserve vault validation into create_market / reserve vault検証をcreate_marketへ接続

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Require create_market to validate reserve vaults for every market asset.

## Requirement Areas / 関連Requirement Area

- DTF-010
- NFR-SEC-004
- PMV-002

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/10-non-functional-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Call reserve vault validation helper for each asset.
- Enforce vault mint, owner/authority, and PDA namespace.
- Store or confirm reserve vault references in DTFMarket state.

## Out of Scope / 対象外

- Mint-time balance delta accounting.
- Fee custody validation.
- Production venue vault validation.

## Acceptance Criteria / 完了条件

- Correct reserve vaults pass create_market.
- Wrong vault fails.
- Wrong vault mint fails.
- Wrong owner/authority fails.
- Missing reserve vault fails.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-15F and P0-IMPL-15G.

## Dependencies / 依存関係

- P0-IMPL-13B.
- P0-IMPL-15A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `create-market`
- `reserves`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Do not accept fee vaults or external pool accounts as reserve vaults.
