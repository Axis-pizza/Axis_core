# [P0-IMPL-11C] Implement supported asset validation helpers / サポート資産検証ヘルパーを実装

## Parent Issue / 親Issue

P0-IMPL-11

## Summary / 概要

Implement reusable helpers for validating that a market asset exists and is enabled for the requested operation.

## Requirement Areas / 関連Requirement Area

- DTF-006 to DTF-008
- MINT-005 to MINT-007
- REDEEM-007
- POLICY-006, POLICY-007

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/07-execution-policy-risk-controls.md`

## Scope / 対象範囲

- Validate asset account matches expected asset mint/PDA.
- Validate creation-enabled, mint-enabled, and redeem-enabled checks.
- Provide max weight, hard min allocation, max trade, and price impact policy accessors.
- Ensure callers cannot pass arbitrary unregistered assets.

## Out of Scope / 対象外

- Pricing source validation.
- ApprovedRoute validation.
- Full mint/redeem instruction implementation.

## Acceptance Criteria / 完了条件

- Unknown asset fails.
- Wrong asset config account fails.
- Disabled operation flag fails for that operation.
- Max weight and max trade policy can be read by market/mint code.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-11D.

## Dependencies / 依存関係

- P0-IMPL-11A.
- P0-IMPL-11B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `asset-registry`
- `validation`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

These helpers are reused by M2 and M3. Keep them deterministic and easy to test.
