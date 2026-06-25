# [P0-IMPL-15G] Add create_market failure tests / create_market失敗系テストを追加

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Add deterministic failure tests for invalid create_market inputs and accounts.

## Requirement Areas / 関連Requirement Area

- DTF-001 to DTF-011
- FEE-004, FEE-005
- PMV-008
- NFR-SEC-001

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/10-non-functional-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Test invalid asset count.
- Test duplicate assets.
- Test invalid weight sum and weight below minimum.
- Test asset max weight failure.
- Test unknown and creation-disabled asset.
- Test wrong reserve vault, wrong mint, wrong owner.
- Test wrong DTF mint or authority.
- Test creator custom fee bps cannot override ProtocolConfig.

## Out of Scope / 対象外

- Mint intake failure tests.
- Route failure tests.
- Production venue invalid account tests.

## Acceptance Criteria / 完了条件

- All specified create_market failure cases fail deterministically.
- Failed create_market does not initialize a valid market account.
- Tests do not rely on backend/app state as protocol truth.

## Test Requirements / 必要なテスト

- This issue is the failure test suite for M2.

## Dependencies / 依存関係

- P0-IMPL-15A.
- P0-IMPL-15B.
- P0-IMPL-15C.
- P0-IMPL-15D.
- P0-IMPL-15E.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `create-market`
- `negative-tests`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

These tests should lock down the market boundary before mint lifecycle work depends on it.
