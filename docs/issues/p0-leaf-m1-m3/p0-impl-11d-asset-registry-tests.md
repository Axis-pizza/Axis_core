# [P0-IMPL-11D] Add AssetRegistry validation tests / AssetRegistry検証テストを追加

## Parent Issue / 親Issue

P0-IMPL-11

## Summary / 概要

Add focused tests for asset config layout, flag updates, and supported asset validation helpers.

## Requirement Areas / 関連Requirement Area

- ASSET
- POLICY-002, POLICY-006, POLICY-010, POLICY-011
- ADMIN-002, ADMIN-003
- DTF-007, DTF-008

## Source Documents / 参照ドキュメント

- `requirements/07-execution-policy-risk-controls.md`
- `requirements/08-asset-universe-requirements.md`
- `requirements/09-admin-safety-requirements.md`

## Scope / 対象範囲

- Test valid asset config initialization/update.
- Test unauthorized update failure.
- Test unknown asset failure.
- Test disabled creation/mint/redeem flag behavior.
- Test policy values are read correctly.

## Out of Scope / 対象外

- Create_market E2E tests.
- Mint E2E tests.
- Production asset launch approval.

## Acceptance Criteria / 完了条件

- AssetRegistry tests pass deterministically.
- Unknown and disabled assets fail in helper tests.
- Unauthorized updates fail.
- Exit-only flag behavior is represented.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-11A through P0-IMPL-11C.

## Dependencies / 依存関係

- P0-IMPL-11A.
- P0-IMPL-11B.
- P0-IMPL-11C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `asset-registry`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Do not treat liquidity discovery output as equivalent to enabled asset state.
