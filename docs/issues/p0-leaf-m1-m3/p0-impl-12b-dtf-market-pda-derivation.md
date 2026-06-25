# [P0-IMPL-12B] Implement DTFMarket PDA derivation / DTFMarket PDA導出を実装

## Parent Issue / 親Issue

P0-IMPL-12

## Summary / 概要

Implement deterministic PDA derivation and validation for `DTFMarket`.

## Requirement Areas / 関連Requirement Area

- DTF-011
- APPIF-001
- RBA-035
- NFR-SEC-001

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/16-route-builder-backend-api-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Define PDA seeds for markets.
- Implement PDA re-derivation helper.
- Validate supplied market account matches expected PDA and owner.
- Provide helper for later create_market/mint tests.

## Out of Scope / 対象外

- Market initialization.
- DTF mint PDA policy unless tied to the accepted account model.
- Off-chain market URL or metadata.

## Acceptance Criteria / 完了条件

- Expected market PDA can be derived deterministically.
- Wrong market account fails validation.
- Helper does not trust client-provided market identity without PDA check.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-12D.

## Dependencies / 依存関係

- P0-IMPL-12A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `dtf-market`
- `pda`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Use PDA re-derivation as a safety pattern from legacy analysis without porting legacy product logic.
