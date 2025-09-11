describe('Solana Tax & Reward DApp E2E', () => {
  before(() => {
    cy.visit('/');
  });

  it('should load the homepage', () => {
    cy.contains('Solana Tax & Reward');
  });

  it('should connect wallet', () => {
    cy.get('button').contains('Connect Wallet').click();
    // Assuming mock wallet connection in test environment
    cy.window().its('solana').should('exist');
  });

  it('should perform a token swap', () => {
    // Set input amount
    cy.get('input[name="amount"]').clear().type('1');
    // Submit swap form
    cy.get('button').contains('Swap & Distribute').click();
    // Verify transaction status
    cy.contains('Transaction submitted').should('be.visible');
  });

  it('should display claimable rewards', () => {
    cy.get('button').contains('Claim Rewards').click();
    cy.contains('Rewards claimed').should('be.visible');
  });
});