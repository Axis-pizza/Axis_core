# [P0-IMPL-13C] Add wrong vault / wrong mint / wrong owner tests / 不正Vault・不正Mint・不正Ownerテストを追加

## Parent Issue / 親Issue

P0-IMPL-13

## Summary / 概要

Add focused tests for reserve vault PDA derivation and token-account validation failures.

## Requirement Areas / 関連Requirement Area

- DTF-010
- REDEEM-006
- NFR-SEC-001, NFR-SEC-004
- PMV-008

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/10-non-functional-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Test correct reserve vault validation.
- Test wrong vault rejection.
- Test wrong mint rejection.
- Test wrong owner/authority rejection.
- Test fee vault/user account cannot be substituted as reserve.

## Out of Scope / 対象外

- Production venue account spoofing tests.
- Full mint/redeem rollback tests.

## Acceptance Criteria / 完了条件

- Reserve vault validation tests pass deterministically.
- All specified wrong-account cases fail.
- Tests document that reserve vault balances, not target weights, are accounting inputs.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-13A and P0-IMPL-13B.

## Dependencies / 依存関係

- P0-IMPL-13A.
- P0-IMPL-13B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `reserves`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

These tests should catch accidental acceptance of non-reserve accounts before mint/redeem work depends on them.
