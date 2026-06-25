# [P0-IMPL-15A] Add create_market instruction skeleton / create_market命令スケルトンを追加

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Add the `create_market` instruction entrypoint, account surface, and basic dispatch wiring.

## Requirement Areas / 関連Requirement Area

- DTF
- APPIF-005
- NFR-DOC-001

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Add create_market instruction discriminator and handler skeleton.
- Define required accounts for ProtocolConfig, creator, DTFMarket, asset configs, reserve vaults, DTF mint, and token/system programs as needed.
- Validate basic account count/shape before deeper validation.

## Out of Scope / 対象外

- Full asset/weight validation.
- Market state initialization.
- Mint/redeem behavior.

## Acceptance Criteria / 完了条件

- Instruction compiles and dispatches.
- Account surface is documented enough for client/test construction.
- Handler returns explicit errors for obviously missing or malformed accounts.

## Test Requirements / 必要なテスト

- Happy path/failure coverage is added in P0-IMPL-15F and P0-IMPL-15G.

## Dependencies / 依存関係

- P0-IMPL-10B.
- P0-IMPL-12A.
- Existing instruction surface proposal.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `create-market`
- `instruction`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Keep this as wiring only; avoid hiding validation scope inside the skeleton issue.
