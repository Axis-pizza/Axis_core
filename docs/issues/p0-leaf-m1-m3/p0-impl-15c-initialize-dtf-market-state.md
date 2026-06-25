# [P0-IMPL-15C] Initialize DTFMarket state / DTFMarket状態を初期化

## Parent Issue / 親Issue

P0-IMPL-15

## Summary / 概要

Initialize the `DTFMarket` account after create_market validation succeeds.

## Requirement Areas / 関連Requirement Area

- DTF-010 to DTF-017
- FEE-001 to FEE-005
- APPIF-005

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`

## Scope / 対象範囲

- Write creator and creator fee destination.
- Write DTF mint and asset/weight list.
- Write reserve vault references or derivation inputs.
- Copy immutable market fee config from ProtocolConfig.
- Set initial market status according to accepted spec.

## Out of Scope / 対象外

- Mint instruction.
- Fee accrual.
- Off-chain metadata or strategy registry.

## Acceptance Criteria / 完了条件

- Market state is initialized only after validation succeeds.
- Creator and fee destination are immutable after creation.
- Creators cannot customize fee bps.
- Market fee config matches ProtocolConfig at creation time.
- No market-level TVL cap is required.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-15F and P0-IMPL-15G.

## Dependencies / 依存関係

- P0-IMPL-12C.
- P0-IMPL-15B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `create-market`
- `dtf-market`
- `fees`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Market state is protocol truth; backend/app metadata is display and indexing only.
