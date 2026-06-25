# [P0-IMPL-17B] Implement controlled adapter execution fixture / 制御アダプター実行フィクスチャを実装

## Parent Issue / 親Issue

P0-IMPL-17

## Summary / 概要

Implement the local test fixture that moves real test tokens through the controlled adapter path.

## Requirement Areas / 関連Requirement Area

- EXEC-013, EXEC-020
- VENUE-005
- PMV-004

## Source Documents / 参照ドキュメント

- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/14-production-venue-integration-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`

## Scope / 対象範囲

- Implement controlled token movement for P0 tests.
- Support deterministic output amounts.
- Support failure injection for min_out and rollback tests.
- Use real SPL/Token-2022 test token accounts where applicable.

## Out of Scope / 対象外

- Production venue adapters.
- Mainnet-fork account cloning.
- Route discovery.

## Acceptance Criteria / 完了条件

- Fixture can move test tokens in mint direction.
- Fixture can move test tokens in redeem direction or is ready for redeem parent work.
- Fixture can intentionally return insufficient output.
- Fixture failure reverts parent instruction in tests.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-18D and later E2E tests.

## Dependencies / 依存関係

- P0-IMPL-17A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `controlled-adapter`
- `tests`
- `fixture`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

The fixture must be visibly test-only to avoid accidental production claims.
