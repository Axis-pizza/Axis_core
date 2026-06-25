# [P0-IMPL-18D] Add balance delta negative tests / 残高差分のネガティブテストを追加

## Parent Issue / 親Issue

P0-IMPL-18

## Summary / 概要

Add negative tests proving balance delta checks reject unsafe mint execution outcomes.

## Requirement Areas / 関連Requirement Area

- MINT-009, MINT-010, MINT-014
- EXEC-007, EXEC-008, EXEC-020
- PMV-003, PMV-008, PMV-009

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Test output below min_out fails.
- Test zero/negative reserve delta fails where output is required.
- Test wrong reserve account fails.
- Test excessive USDC source drain fails.
- Test failed delta validation rolls back fees and DTF minting.

## Out of Scope / 対象外

- Production venue invalid account tests.
- Redeem balance delta tests, which remain under M4 parent expansion.

## Acceptance Criteria / 完了条件

- Balance delta negative tests pass deterministically.
- Failed mint does not mint DTF.
- Failed mint does not accrue fees.
- Failed mint does not accept quote as accounting truth.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-18A through P0-IMPL-18C.

## Dependencies / 依存関係

- P0-IMPL-18A.
- P0-IMPL-18B.
- P0-IMPL-18C.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `balance-delta`
- `negative-tests`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

These tests are the main guard against quote-based over-minting.
