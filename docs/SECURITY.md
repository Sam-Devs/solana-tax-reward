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
- Enforce account owner and PDA seeds validation using Anchor macros to prevent unauthorized access
- Request increased compute budget for complex CPIs and split large operations to prevent transaction failures
- Add detailed logging (`msg!`) before and after all CPI calls for auditability

## Audits
- Internal code review
- Third-party security audit: external audit performed quarterly (Jan/Apr/Jul/Oct), tracked via GitHub issues and action items logged in project board
- Automated CI enforcement: CI `security` job fails on critical or high-severity findings, blocks merges until resolved, and opens remediation issues automatically
- Automated security scans: mantaray, oxygen