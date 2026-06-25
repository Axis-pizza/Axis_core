# [P0-IMPL-14B] Implement DTF mint authority validation / DTFミント権限検証を実装

## Parent Issue / 親Issue

P0-IMPL-14

## Summary / 概要

Implement validation that the market DTF Token-2022 mint is controlled by Axis and matches the expected market.

## Requirement Areas / 関連Requirement Area

- DTF-011
- REDEEM-001
- NFR-SEC-005

## Source Documents / 参照ドキュメント

- `requirements/02-dtf-market-requirements.md`
- `requirements/04-redeem-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Validate DTF mint token program and mint address.
- Validate Axis-controlled mint authority.
- Validate supply can be read for initial NAV/redeem-share logic.
- Reject wrong DTF mint.

## Out of Scope / 対象外

- Mint amount calculation.
- Burn instruction implementation.
- Secondary-market DTF/USDC pool creation.

## Acceptance Criteria / 完了条件

- Correct DTF mint passes validation.
- Wrong DTF mint fails.
- Wrong mint authority fails.
- Mint supply is available to downstream accounting.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-14C.

## Dependencies / 依存関係

- P0-IMPL-14A.
- P0-IMPL-12A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `token-2022`
- `validation`
- `security`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Do not rely on an internal total supply mirror when Token-2022 mint supply can be validated.
