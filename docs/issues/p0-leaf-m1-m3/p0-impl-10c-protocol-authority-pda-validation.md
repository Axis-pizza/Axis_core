# [P0-IMPL-10C] Validate protocol authority and PDA derivation / プロトコル権限とPDA導出を検証

## Parent Issue / 親Issue

P0-IMPL-10

## Summary / 概要

Implement reusable validation for the ProtocolConfig PDA and authority roles.

## Requirement Areas / 関連Requirement Area

- ADMIN-001
- ADMIN-002 to ADMIN-008
- NFR-SEC-001

## Source Documents / 参照ドキュメント

- `requirements/09-admin-safety-requirements.md`
- `requirements/10-non-functional-requirements.md`

## Scope / 対象範囲

- Re-derive the expected ProtocolConfig PDA.
- Validate supplied ProtocolConfig account matches expected PDA and owner.
- Validate authority signer roles for protocol, pause, asset, route, and pricing operations.
- Provide helpers usable by later M1-M3 instructions.

## Out of Scope / 対象外

- Implementing every admin update instruction.
- Market pause implementation.
- Route registry implementation.

## Acceptance Criteria / 完了条件

- Wrong ProtocolConfig account fails validation.
- Missing or wrong authority signer fails.
- Correct authority signer passes for its role.
- Helpers do not trust client-provided authority labels without checking account state.

## Test Requirements / 必要なテスト

- Covered by P0-IMPL-10D and later instruction-specific tests.

## Dependencies / 依存関係

- P0-IMPL-10A.
- P0-IMPL-10B.

## Suggested Labels / 推奨ラベル

- `p0`
- `leaf`
- `protocol-config`
- `security`
- `pda`

## Suggested Owner / 推奨Owner

Toby.

## Notes / 補足

Use PDA re-derivation as a safety pattern; do not copy legacy product behavior.
