# [P0-IMPL-11B] Implement asset enable/disable policy flags / 資産の有効化・無効化ポリシーフラグを実装

## Parent Issue / 親Issue

P0-IMPL-11

## Summary / 概要

Implement independent asset policy flags for creation, mint, redeem, and rebalance behavior.

## Requirement Areas / 関連Requirement Area

- POLICY-010, POLICY-011
- ADMIN-002, ADMIN-003
- MINT-007
- REDEEM-007

## Source Documents / 参照ドキュメント

- `requirements/07-execution-policy-risk-controls.md`
- `requirements/09-admin-safety-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/04-redeem-requirements.md`

## Scope / 対象範囲

- Implement storage/update behavior for `creation_enabled`, `mint_enabled`, `redeem_enabled`, and `rebalance_enabled`.
- Validate asset registry authority for updates.
- Support exit-only configuration where creation and mint are disabled but redeem may remain enabled.

## Out of Scope / 対象外

- Market pause.
- Route disable controls.
- Full governance.

## Acceptance Criteria / 完了条件

- Authorized flag update succeeds.
- Unauthorized flag update fails.
- Flags are independent.
- Exit-only flag set can be represented.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-11D and later create_market/mint tests.

## Dependencies / 依存関係

- P0-IMPL-10C.
- P0-IMPL-11A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `asset-registry`
- `risk-controls`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Preserve exits where safe; disabling mint must not automatically disable redeem.
