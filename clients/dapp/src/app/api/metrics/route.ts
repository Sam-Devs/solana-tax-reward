import { NextResponse } from 'next/server';
import * as promClient from 'prom-client';

// Initialize default metrics once
promClient.collectDefaultMetrics({ prefix: 'dapp_' });

export async function GET() {
  const metrics = await promClient.register.metrics();
  return new NextResponse(metrics, {
    status: 200,
    headers: { 'Content-Type': promClient.register.contentType },
  });
}