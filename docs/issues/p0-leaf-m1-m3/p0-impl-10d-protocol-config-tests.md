# [P0-IMPL-10D] Add ProtocolConfig serialization and initialization tests / ProtocolConfigのシリアライズと初期化テストを追加

## Parent Issue / 親Issue

P0-IMPL-10

## Summary / 概要

Add focused tests for ProtocolConfig layout, initialization, fee validation, PDA derivation, and authority validation.

## Requirement Areas / 関連Requirement Area

- ADMIN-001
- ADMIN-008
- FEE-011, FEE-012
- NFR-DOC-002

## Source Documents / 参照ドキュメント

- `requirements/09-admin-safety-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Add serialization/deserialization or account unpack tests.
- Add valid initialize test.
- Add invalid fee config tests.
- Add reinitialize failure test.
- Add wrong PDA and unauthorized signer tests.

## Out of Scope / 対象外

- Asset registry tests.
- Market creation tests.
- End-to-end mint tests.

## Acceptance Criteria / 完了条件

- ProtocolConfig tests pass deterministically.
- Invalid fee share and fee cap configs fail.
- Reinitialize fails.
- Wrong PDA and unauthorized authority fail.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-10A through P0-IMPL-10C.

## Dependencies / 依存関係

- P0-IMPL-10A.
- P0-IMPL-10B.
- P0-IMPL-10C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `protocol-config`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Use the repo's existing LiteSVM/unit-test pattern once the scaffold is available.
