# [P0-IMPL-16A] Add mint instruction account surface / mint命令のアカウントサーフェスを追加

## Parent Issue / 親Issue

P0-IMPL-16

## Summary / 概要

Define and wire the P0 mint instruction account surface before implementing detailed mint accounting.

## Requirement Areas / 関連Requirement Area

- MINT-001, MINT-002
- APPIF-006
- NFR-SEC-001

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Add mint instruction discriminator/handler skeleton.
- Define accounts for ProtocolConfig, DTFMarket, user authority, user USDC source, user DTF destination, reserve vaults, fee custody, asset configs, route placeholders, controlled adapter accounts, DTF mint, and token programs as needed.
- Validate basic market status access and account shape.

## Out of Scope / 対象外

- USDC balance movement.
- Fee/net split.
- Controlled adapter implementation.
- DTF mint amount calculation.

## Acceptance Criteria / 完了条件

- Mint instruction compiles and dispatches.
- Account surface is documented for LiteSVM fixture construction.
- Obvious missing or malformed account sets fail explicitly.

## Test Requirements / 必要なテスト

- Failure coverage begins in P0-IMPL-16D.

## Dependencies / 依存関係

- P0-IMPL-15C.
- Existing instruction surface proposal.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `mint`
- `instruction`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Do not add production venue account assumptions in this issue.
