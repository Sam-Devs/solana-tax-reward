# Testing Framework Documentation

## Overview

This document describes the comprehensive testing infrastructure implemented for the Solana Tax Reward program. The testing framework includes multiple layers of validation to ensure program correctness, security, and performance.

## Test Structure

### 1. Unit Tests (`tests/unit_tests.rs`)

**Purpose**: Test individual functions and components in isolation

**Coverage**:
- State structure serialization/deserialization
- Tax calculation logic
- Error type validation
- Basic data type properties
- Overflow protection in calculations

**Key Features**:
- Fast execution (no blockchain simulation)
- Focused on pure functions
- Tests mathematical properties and edge cases

### 2. Integration Tests (`tests/integration_tests.rs`) 

**Purpose**: Test program instructions using Anchor's testing framework

**Coverage**:
- Program initialization
- Taxed swap and distribute flows
- Reward claiming mechanics  
- Configuration updates
- Error condition handling

**Key Features**:
- Uses `#[tokio::test]` for async testing
- Simulates realistic program interactions
- Tests PDA derivations and account relationships

### 3. Property-Based Tests (`tests/property_tests.rs`)

**Purpose**: Validate program behavior across wide input ranges using randomized testing

**Coverage**:
- Tax calculation invariants (never exceeds input)
- Reward calculation determinism
- Account size consistency
- PDA derivation consistency  
- Edge cases with maximum/minimum values
- Overflow protection
- Rounding behavior validation

**Key Features**:
- Uses `proptest` crate for random input generation
- Tests mathematical properties that must always hold
- Validates behavior at boundary conditions
- Stress tests with large-scale values

### 4. End-to-End Tests (`tests/e2e_tests.rs`)

**Purpose**: Test complete program flows using Solana Program Test framework

**Coverage**:
- Full initialization sequence
- Real instruction execution
- Token account creation and management
- SOL reward distribution
- Program pausing and error scenarios

**Key Features**:
- Uses `solana-program-test` for realistic blockchain simulation
- Creates real mint accounts and token operations
- Tests complete user journeys
- Validates state transitions

### 5. Anchor Integration Tests (`tests/anchor_tests.rs`)

**Purpose**: Test using Anchor's native testing patterns with real program context

**Coverage**:
- Context validation for all instructions
- Account constraint verification
- Cross-program invocation testing
- Real SPL token integration

**Key Features**:
- Native Anchor test environment
- Real program compilation and execution
- Comprehensive account relationship testing

### 6. Test Utilities (`tests/test_utils.rs`)

**Purpose**: Provide reusable testing infrastructure and helpers

**Components**:
- Environment setup functions
- Mock data generators
- Assertion helpers
- Common test scenarios
- PDA derivation utilities

## Test Execution

### Running All Tests
```bash
# Run all unit and integration tests
cargo test

# Run property-based tests (may take longer)
cargo test -- --ignored

# Run specific test modules
cargo test unit_tests
cargo test integration_tests
cargo test property_tests
```

### Running Anchor Tests
```bash
# Run Anchor-specific tests
anchor test --skip-deploy
```

## Test Categories

### Mathematical Properties
- **Tax Calculation**: Validates that tax never exceeds input amount
- **Proportionality**: Tests that tax scales correctly with rate
- **Reward Distribution**: Ensures rewards are calculated consistently
- **Overflow Protection**: Verifies safe arithmetic operations

### State Management
- **Account Sizes**: Validates serialized sizes match constants
- **PDA Consistency**: Tests deterministic address generation
- **State Transitions**: Validates valid state changes

### Error Conditions
- **Program Paused**: Tests behavior when operations are disabled
- **Invalid Inputs**: Validates error handling for bad parameters
- **Authority Checks**: Tests access control enforcement

### Performance & Scale
- **Large Values**: Tests with maximum safe input values
- **Precision Limits**: Validates behavior at calculation boundaries
- **Stress Testing**: Tests system under high-load scenarios

## Dependencies

The testing framework relies on:
- `proptest = "1.0"` - Property-based testing
- `tokio` - Async test runtime
- `solana-program-test` - Blockchain simulation
- `solana-sdk` - Core Solana types
- `spl-token` - Token program integration
- `anchor-lang` - Anchor framework testing

## Test Data Management

### Mock Data Generation
- Realistic but deterministic test scenarios
- Edge case data for boundary testing
- Invalid data for error condition testing

### Test Isolation
- Each test creates fresh program state
- No shared state between test runs
- Deterministic randomization with seeds

## Continuous Integration

### Pre-commit Checks
```bash
# Format and lint checks
cargo fmt -- --check
cargo clippy --all -- -D warnings

# Complete test suite
cargo test
cargo test -- --ignored
anchor test --skip-deploy
```

### Coverage Goals
- **Unit Tests**: >90% function coverage
- **Integration Tests**: All instruction paths covered
- **Property Tests**: All mathematical invariants verified
- **E2E Tests**: All user flows validated

## Test Maintenance

### Adding New Tests
1. Identify the component/behavior to test
2. Choose appropriate test type (unit/integration/property/e2e)
3. Follow existing patterns and naming conventions
4. Include both positive and negative test cases
5. Document any complex test scenarios

### Test Performance
- Unit tests should complete in <100ms each
- Integration tests should complete in <1s each
- Property tests may take longer but should complete in <30s each
- E2E tests should complete in <5s each

### Debugging Tests
- Use `RUST_LOG=debug cargo test` for detailed logging
- Individual test execution: `cargo test test_name`
- Test output: `cargo test -- --nocapture`

## Security Testing

### Exploit Prevention
- Tests for arithmetic overflow/underflow
- Authority bypass attempts
- Malformed account data handling
- Re-entrancy protection

### Economic Security
- Tax rate boundary testing
- Reward calculation precision
- Token supply manipulation resistance
- Fee calculation accuracy

This comprehensive testing framework ensures the Solana Tax Reward program is robust, secure, and reliable across all operating conditions.
