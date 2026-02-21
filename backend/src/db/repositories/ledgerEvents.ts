import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findByContract(contractId: string, limit = 50) {
  return prisma.ledgerEvent.findMany({
    where: { contractId },
    orderBy: { ledgerSequence: 'desc' },
    take: limit,
  });
}

export async function findByType(eventType: string, limit = 50) {
  return prisma.ledgerEvent.findMany({
    where: { eventType },
    orderBy: { indexedAt: 'desc' },
    take: limit,
  });
}

export async function create(data: Prisma.LedgerEventCreateInput) {
  return prisma.ledgerEvent.create({ data });
}

export async function getLatestSequence() {
  const event = await prisma.ledgerEvent.findFirst({
    orderBy: { ledgerSequence: 'desc' },
    select: { ledgerSequence: true },
  });
  return event?.ledgerSequence ?? BigInt(0);
}
