# [P0-IMPL-15F] Add create_market happy path LiteSVM test / create_market正常系LiteSVMテストを追加

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Add a deterministic LiteSVM happy path proving a valid DTF market can be created.

## Requirement Areas / 関連Requirement Area

- DTF-001 to DTF-017
- PMV-002
- APPIF-005

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`

## Scope / 対象範囲

- Build fixture for ProtocolConfig, two enabled assets, reserve vaults, and Token-2022 DTF mint.
- Execute create_market successfully.
- Assert DTFMarket state fields, asset list, weights, reserve vault references, DTF mint, creator, status, and fee config.

## Out of Scope / 対象外

- Mint execution.
- Redeem execution.
- Production venue CPI.

## Acceptance Criteria / 完了条件

- 2-asset create_market succeeds.
- State matches expected inputs and derived config.
- Test is deterministic and suitable for CI.

## Test Requirements / 必要なテスト

- This issue is the happy path test for P0-IMPL-15A through P0-IMPL-15E.

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
- `litesvm`
- `create-market`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

This test proves market creation only; mint path coverage starts in M3.
