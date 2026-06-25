# [P0-IMPL-16B] Validate user USDC input and token accounts / ユーザーUSDC入力とトークンアカウントを検証

## Parent Issue / 親Issue

P0-IMPL-16

## Summary / 概要

Validate that mint input comes from the configured USDC mint and expected user token account.

## Requirement Areas / 関連Requirement Area

- MINT-001, MINT-002
- APPIF-006
- NFR-SEC-001

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Validate user source token account mint equals ProtocolConfig USDC mint.
- Validate user authority controls or signs for the source account as required.
- Validate user DTF destination account mint equals market DTF mint.
- Validate market status allows mint.

## Out of Scope / 対象外

- Fee calculation.
- CPI execution.
- DTF token minting.

## Acceptance Criteria / 完了条件

- Valid USDC source passes.
- Non-USDC input fails.
- Wrong user authority fails.
- Wrong DTF destination mint fails.
- Paused/deprecated market blocks mint according to status policy.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-16D.

## Dependencies / 依存関係

- P0-IMPL-10C.
- P0-IMPL-14B.
- P0-IMPL-16A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `usdc`
- `validation`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Mint input is USDC only for P0.
