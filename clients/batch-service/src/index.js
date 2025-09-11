const tracer = require('dd-trace').init({ analytics: true });
const express = require('express');
const promClient = require('prom-client');
const bodyParser = require('body-parser');

const app = express();
app.use(bodyParser.json());

// Metrics
const collectDefaultMetrics = promClient.collectDefaultMetrics;
collectDefaultMetrics({ prefix: 'batch_service_' });
const opsCounter = new promClient.Counter({
  name: 'batch_service_ops_total',
  help: 'Number of batch operations',
});
const swapDuration = new promClient.Histogram({
  name: 'batch_service_swap_duration_seconds',
  help: 'Duration of batch swap operations',
  buckets: [0.1, 0.5, 1, 2],
});

app.get('/health', (req, res) => {
  res.json({ status: 'ok' });
});

app.get('/metrics', async (req, res) => {
  res.set('Content-Type', promClient.register.contentType);
  res.end(await promClient.register.metrics());
});

app.post('/batch/swap', async (req, res) => {
  const start = Date.now();
  opsCounter.inc(req.body.operations.length);
  // simulate batch swap
  const results = req.body.operations.map((op) => ({
    user: op.user,
    success: true,
  }));
  const duration = (Date.now() - start) / 1000;
  swapDuration.observe(duration);
  res.json({ results });
});

const PORT = process.env.PORT || 4000;
app.listen(PORT, () =>
  console.log(`Batch service listening on port ${PORT}`),
);