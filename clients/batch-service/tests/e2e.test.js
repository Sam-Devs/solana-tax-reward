const request = require('supertest');

describe('Batch Service E2E', () => {
  const api = request('http://localhost:4000');

  beforeAll(async () => {
    // Optionally start the service if not already running
  });

  it('responds to health check', async () => {
    const res = await api.get('/health');
    expect(res.statusCode).toBe(200);
    expect(res.body).toHaveProperty('status', 'ok');
  });

  it('performs a batch swap operation', async () => {
    const payload = { operations: [{ user: 'test-user', amount: 1 }] };
    const res = await api.post('/batch/swap').send(payload);
    expect(res.statusCode).toBe(200);
    expect(res.body).toHaveProperty('results');
    expect(Array.isArray(res.body.results)).toBe(true);
  });
});