# [P0-IMPL-10A] Define ProtocolConfig account layout / ProtocolConfigアカウントレイアウトを定義

## Parent Issue / 親Issue

P0-IMPL-10

## Summary / 概要

Define the concrete `ProtocolConfig` account fields and layout needed by P0 Axis Core.

## Requirement Areas / 関連Requirement Area

- AXIS-CORE
- ADMIN-001
- ADMIN-008
- FEE-011, FEE-012
- NFR-DOC-001

## Source Documents / 参照ドキュメント

- `requirements/09-admin-safety-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Define fields for authority, pause authority, asset registry authority, route registry authority, pricing registry authority, protocol treasury, configured USDC mint, and protocol fee config.
- Define versioning, bump, reserved bytes, and size constraints if required by the account model.
- Document which fields are immutable after initialization for P0.

## Out of Scope / 対象外

- Initialize instruction implementation.
- Asset, market, route, mint, or fee claim state.
- Mainnet deployment configuration.

## Acceptance Criteria / 完了条件

- Account layout is documented in code or implementation notes.
- Required authority and fee fields are represented.
- Fee caps support v1 values: `max_mint_fee_bps = 300`, `max_redeem_fee_bps = 0`.
- Layout does not depend on backend/app state.

## Test Requirements / 必要なテスト

- Layout size or serialization test may be added in P0-IMPL-10D.

## Dependencies / 依存関係

- Existing account model proposal.
- Existing instruction surface proposal.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `protocol-config`
- `account-layout`

## Suggested Owner / 推奨Owner

ADP design review, Toby implementation.

## Notes / 補足

Keep this account focused on protocol configuration. Do not encode launch or deployment-process assumptions here.
