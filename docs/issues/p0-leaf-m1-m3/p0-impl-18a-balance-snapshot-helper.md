# [P0-IMPL-18A] Implement pre/post token balance snapshot helper / 実行前後トークン残高スナップショットヘルパーを実装

## Parent Issue / 親Issue

P0-IMPL-18

## Summary / 概要

Implement reusable helpers to snapshot token balances before and after controlled execution.

## Requirement Areas / 関連Requirement Area

- MINT-010
- EXEC-008
- PMV-003
- NFR-REL-004

## Source Documents / 参照ドキュメント

- `requirements/03-mint-requirements.md`
- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Read token account balances before execution.
- Read token account balances after execution.
- Return deterministic deltas.
- Support reserve vault outputs and USDC source checks needed by mint.

## Out of Scope / 対象外

- Pricing/NAV calculation.
- Production venue balance handling.
- DTF mint amount calculation.

## Acceptance Criteria / 完了条件

- Helper reads balances from validated token accounts.
- Helper reports positive, zero, and negative movement clearly.
- Helper does not use quotes or target weights.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-18D.

## Dependencies / 依存関係

- P0-IMPL-13B.
- P0-IMPL-17B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `balance-delta`
- `accounting`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

This helper is the base for actual reserve value accounting.
