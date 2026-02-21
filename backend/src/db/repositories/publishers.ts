import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findMany(filter?: { status?: string; tier?: string }, limit = 20) {
  return prisma.publisher.findMany({
    where: {
      ...(filter?.status ? { status: filter.status } : {}),
      ...(filter?.tier ? { tier: filter.tier } : {}),
    },
    orderBy: { earningsStroops: 'desc' },
    take: limit,
  });
}

export async function findByAddress(address: string) {
  return prisma.publisher.findUnique({ where: { address } });
}

export async function leaderboard(limit = 20) {
  return prisma.publisher.findMany({
    where: { status: 'Verified' },
    orderBy: [{ earningsStroops: 'desc' }, { reputationScore: 'desc' }],
    take: limit,
  });
}

export async function create(data: Prisma.PublisherCreateInput) {
  return prisma.publisher.create({ data });
}

export async function updateReputation(address: string, score: number) {
  return prisma.publisher.update({
    where: { address },
    data: { reputationScore: score, lastActivity: new Date() },
  });
}
