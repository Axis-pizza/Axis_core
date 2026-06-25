# [P0-IMPL-10B] Implement initialize_protocol_config instruction / initialize_protocol_config命令を実装

## Parent Issue / 親Issue

P0-IMPL-10

## Summary / 概要

Implement the instruction that initializes `ProtocolConfig` at the expected PDA with validated v1 settings.

## Requirement Areas / 関連Requirement Area

- AXIS-CORE
- ADMIN-001
- ADMIN-008
- FEE-011, FEE-012

## Source Documents / 参照ドキュメント

- `requirements/09-admin-safety-requirements.md`
- `requirements/13-fee-model-requirements.md`
- `requirements/19-axis-core-implementation-rfc.md`

## Scope / 対象範囲

- Add instruction discriminator/handler for protocol initialization.
- Initialize the ProtocolConfig PDA.
- Store required authorities, USDC mint, protocol treasury, and fee config.
- Reject invalid fee shares and fee caps.
- Prevent reinitialization.

## Out of Scope / 対象外

- Updating ProtocolConfig after initialization.
- Asset registry or market creation.
- Governance/multisig implementation.

## Acceptance Criteria / 完了条件

- Valid initialization succeeds.
- Reinitialization fails.
- `creator_share_bps + protocol_share_bps != 10000` fails.
- `mint_fee_bps > max_mint_fee_bps` fails.
- Nonzero `redeem_fee_bps` fails for v1 unless a later approved spec changes it.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-10D.

## Dependencies / 依存関係

- P0-IMPL-10A.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `protocol-config`
- `instruction`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

This establishes the config root used by M1-M3 work.
