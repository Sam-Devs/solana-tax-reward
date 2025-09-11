# Security

## Threat Model
- CPI reentrancy
- Overflow/underflow
- Unauthorized config changes
- Seed/PDAs collision

## Controls
- Anchor account constraint macros
- Safe-math and overflow checks
- Multisig admin via Realms with timelock
- Pause flag for emergency halt
- Close stale UserInfo accounts to limit attack surface

## Audits
- Internal code review
- Third-party security audit (TBD)
- Automated security scans: mantaray, oxygen