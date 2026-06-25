# [P0-IMPL-14A] Define Token-2022 DTF mint extension policy / Token-2022 DTFミント拡張ポリシーを定義

## Parent Issue / 親Issue

P0-IMPL-14

## Summary / 概要

Define the conservative Token-2022 extension policy for DTF mints used in P0.

## Requirement Areas / 関連Requirement Area

- DTF-011
- NFR-SEC-005
- PMV-002

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/10-non-functional-requirements.md`
- `requirements/12-pre-mainnet-validation-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Define which Token-2022 mint extensions are allowed for P0 DTF mints.
- Define which extensions are explicitly disallowed or deferred.
- Document whether Axis Core creates the DTF mint or validates a supplied mint for P0.

## Out of Scope / 対象外

- StockToken/xStocks extension handling.
- Secondary-market pool setup.
- Final metadata strategy if deferred.

## Acceptance Criteria / 完了条件

- P0 extension policy is documented.
- Unsupported extensions fail validation or are out of scope.
- Policy is compatible with mint authority and supply checks.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-14C where practical.

## Dependencies / 依存関係

- Existing Token-2022 spike.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `token-2022`
- `dtf-mint`
- `design`

## Suggested Owner / 推奨Owner

ADP design review, Toby implementation.

## Notes / 補足

Do not pull StockToken/xStocks production readiness into P0 Core.
