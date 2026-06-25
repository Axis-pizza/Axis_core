# [P0-IMPL-11A] Define AssetConfig / AssetRegistry state / AssetConfig・AssetRegistry状態を定義

## Parent Issue / 親Issue

P0-IMPL-11

## Summary / 概要

Define on-chain state for supported assets and per-asset execution policy.

## Requirement Areas / 関連Requirement Area

- ASSET
- POLICY-002 to POLICY-005
- DTF-007

## Source Documents / 参照ドキュメント

- `requirements/07-execution-policy-risk-controls.md`
- `requirements/08-asset-universe-requirements.md`
- `requirements/02-dtf-market-requirements.md`

## Scope / 対象範囲

- Define `AssetConfig` or equivalent per-asset state.
- Include asset mint, token program expectation, decimals if needed, policy flags, limits, pricing requirement, and route requirement.
- Define account/PDA shape for lookup by asset mint.

## Out of Scope / 対象外

- Asset discovery scripts.
- Launch-ready asset approval workflow.
- Production route or pricing integrations.

## Acceptance Criteria / 完了条件

- Asset state can represent creation/mint/redeem/rebalance flags.
- Asset state can represent hard minimum allocation, max trade, max weight, price impact, and pricing deviation policy.
- Auto-discovered assets are not implicitly enabled.

## Test Requirements / 必要なテスト

- Serialization tests are covered by P0-IMPL-11D.

## Dependencies / 依存関係

- P0-IMPL-10A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `asset-registry`
- `account-layout`

## Suggested Owner / 推奨Owner

ADP design review, Toby implementation.

## Notes / 補足

The supported asset v0 list is a review input, not automatic on-chain enablement.
