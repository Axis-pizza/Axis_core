# [P0-IMPL-17C] Bind controlled adapter to ApprovedRoute placeholder validation / 制御アダプターをApprovedRouteプレースホルダー検証へ接続

## Parent Issue / 親Issue

P0-IMPL-17

## Summary / 概要

Bind the controlled adapter path to the P0 ApprovedRoute placeholder so mint execution still validates route identity and direction.

## Requirement Areas / 関連Requirement Area

- EXEC-001 to EXEC-012
- APPIF-003, APPIF-004
- RBA-008

## Source Documents / 参照ドキュメント

- `requirements/05-swap-cpi-execution-requirements.md`
- `requirements/15-app-contract-interface-requirements.md`
- `requirements/16-route-builder-backend-api-requirements.md`

## Scope / 対象範囲

- Validate route exists/enabled in the P0 route placeholder.
- Validate route direction, input mint, output mint, and controlled adapter identity.
- Reject unsupported split/multi-hop route payloads.
- Prepare code boundary for future production ApprovedRoute implementation.

## Out of Scope / 対象外

- Full production ApprovedRoute account model.
- Dynamic backend route approval.
- Jupiter route authorization.

## Acceptance Criteria / 完了条件

- Enabled matching route passes.
- Missing, disabled, wrong direction, wrong input mint, and wrong output mint routes fail.
- Route plan data cannot bypass on-chain validation.

## Test Requirements / 必要なテスト

- Failure tests are covered by P0-IMPL-18D and later negative suites.

## Dependencies / 依存関係

- Existing ApprovedRoute proposal.
- P0-IMPL-17A.
- P0-IMPL-17B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `approved-route`
- `controlled-adapter`
- `validation`

## Suggested Owner / 推奨Owner

Toby, ADP review.

## Notes / 補足

Do not authorize routes from opaque Jupiter bytes or backend quotes.
