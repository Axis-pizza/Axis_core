# [P0-IMPL-14C] Add Token-2022 mint authority and supply tests / Token-2022ミント権限と供給量テストを追加

## Parent Issue / 親Issue

P0-IMPL-14

## Summary / 概要

Add focused tests for DTF Token-2022 mint policy, authority validation, and supply reads.

## Requirement Areas / 関連Requirement Area

- DTF-011
- MINT-012
- REDEEM-004
- NFR-SEC-005

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/03-mint-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Test valid Token-2022 DTF mint authority.
- Test wrong authority failure.
- Test wrong DTF mint failure.
- Test supply read for zero and nonzero supply cases.
- Test unsupported extension rejection if implemented in P0.

## Out of Scope / 対象外

- Full mint/redeem lifecycle tests.
- Production Token-2022 StockToken support.

## Acceptance Criteria / 完了条件

- Token-2022 tests pass deterministically.
- Wrong mint and wrong authority fail.
- Supply read behavior supports later mint/redeem accounting.

## Test Requirements / 必要なテスト

- This issue is the test requirement for P0-IMPL-14A and P0-IMPL-14B.

## Dependencies / 依存関係

- P0-IMPL-14A.
- P0-IMPL-14B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `tests`
- `token-2022`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Keep these tests local and deterministic; they do not prove secondary-market or production venue readiness.
