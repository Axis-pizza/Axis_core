# [P0-IMPL-13B] Implement reserve vault token account validation / リザーブVaultトークンアカウント検証を実装

## Parent Issue / 親Issue

P0-IMPL-13

## Summary / 概要

Implement validation that supplied reserve token accounts belong to the expected market and asset.

## Requirement Areas / 関連Requirement Area

- DTF-010
- REDEEM-006
- NFR-SEC-001, NFR-SEC-004

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Validate reserve vault PDA derivation.
- Validate reserve vault mint matches expected asset mint.
- Validate owner/authority is Axis-controlled.
- Validate token program expectations where needed.
- Expose helper for create_market, mint, redeem, and tests.

## Out of Scope / 対象外

- CPI venue account validation.
- Fee custody validation.
- NAV calculation.

## Acceptance Criteria / 完了条件

- Correct reserve vault passes.
- Wrong vault fails.
- Wrong mint fails.
- Wrong owner/authority fails.
- Fee vault or user token account is not accepted as reserve vault.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-13C.

## Dependencies / 依存関係

- P0-IMPL-13A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `reserves`
- `validation`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

This helper is a core custody check and should be used consistently by M2 and M3.
