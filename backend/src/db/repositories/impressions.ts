import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findByCampaign(campaignId: bigint, limit = 50) {
  return prisma.impression.findMany({
    where: { campaignId },
    orderBy: { timestamp: 'desc' },
    take: limit,
  });
}

export async function findByPublisher(publisher: string, limit = 50) {
  return prisma.impression.findMany({
    where: { publisher },
    orderBy: { timestamp: 'desc' },
    take: limit,
  });
}

export async function create(data: Prisma.ImpressionCreateInput) {
  return prisma.impression.create({ data });
}

export async function countByCampaign(campaignId: bigint) {
  return prisma.impression.count({ where: { campaignId } });
}
