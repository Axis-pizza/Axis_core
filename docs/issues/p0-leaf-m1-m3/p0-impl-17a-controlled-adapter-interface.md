# [P0-IMPL-17A] Define controlled adapter interface for P0 tests / P0テスト用の制御アダプターインターフェースを定義

## Parent Issue / 親Issue

P0-IMPL-17

## Summary / 概要

Define the minimal controlled adapter interface used by P0 deterministic tests.

## Requirement Areas / 関連Requirement Area

- EXEC-013, EXEC-014, EXEC-019
- VENUE-005, VENUE-006

## Source Documents / 参照ドキュメント

- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/14-production-venue-integration-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Define operation fields: direction, input mint, output mint, amount in, min out, route/account payload, and actual output path.
- Define mint and redeem directions for P0.
- Define how the adapter is identified as controlled/test-only.

## Out of Scope / 対象外

- Orca Whirlpool CPI.
- Raydium CPMM CPI.
- Jupiter route bytes.
- Split or multi-hop routing.

## Acceptance Criteria / 完了条件

- Interface supports USDC -> asset for mint.
- Interface supports asset -> USDC for redeem.
- Interface requires min_out/min_usdc_out.
- Documentation states this is not production venue readiness.

## Test Requirements / 必要なテスト

- Execution fixture tests are covered by P0-IMPL-17B and P0-IMPL-17C.

## Dependencies / 依存関係

- Existing controlled adapter spike/proposal.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `controlled-adapter`
- `cpi`
- `design`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

Controlled adapter equals local accounting proof only, not Orca/Raydium readiness.
