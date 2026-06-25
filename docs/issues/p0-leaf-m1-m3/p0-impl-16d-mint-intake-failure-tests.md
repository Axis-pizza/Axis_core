# [P0-IMPL-16D] Add mint intake failure tests / mint入力失敗テストを追加

## Parent Issue / 親Issue

P0-IMPL-16

## Summary / 概要

Add deterministic tests for mint account validation, USDC input validation, status validation, and fee/net split failures.

## Requirement Areas / 関連Requirement Area

- MINT-001 to MINT-007
- FEE-006 to FEE-008
- PMV-008

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Test non-USDC input failure.
- Test wrong authority/source/destination account failure.
- Test paused/deprecated market mint failure.
- Test disabled mint asset failure.
- Test min allocation failure.
- Test max trade failure.
- Test failed intake does not accrue fees or mint DTF.

## Out of Scope / 対象外

- Controlled adapter execution tests.
- Balance delta tests.
- Production venue tests.

## Acceptance Criteria / 完了条件

- All mint intake failure tests pass deterministically.
- Failure leaves no DTF mint, reserve accounting, or fee accrual side effect.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-16A through P0-IMPL-16C.

## Dependencies / 依存関係

- P0-IMPL-16A.
- P0-IMPL-16B.
- P0-IMPL-16C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `mint`
- `negative-tests`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

These tests should run before adding controlled execution to keep intake failures isolated.
