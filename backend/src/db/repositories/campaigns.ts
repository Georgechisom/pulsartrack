import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findMany(filter?: { status?: string }, limit = 20) {
  return prisma.campaign.findMany({
    where: filter?.status ? { status: filter.status } : undefined,
    orderBy: { createdAt: 'desc' },
    take: limit,
  });
}

export async function findByCampaignId(campaignId: bigint) {
  return prisma.campaign.findUnique({ where: { campaignId } });
}

export async function getStats() {
  const [total, active, agg] = await Promise.all([
    prisma.campaign.count(),
    prisma.campaign.count({ where: { status: 'Active' } }),
    prisma.campaign.aggregate({
      _sum: { impressions: true, clicks: true, spentStroops: true },
    }),
  ]);

  return {
    totalCampaigns: total,
    activeCampaigns: active,
    totalImpressions: Number(agg._sum.impressions ?? 0),
    totalClicks: Number(agg._sum.clicks ?? 0),
    totalSpentStroops: Number(agg._sum.spentStroops ?? 0),
  };
}

export async function create(data: Prisma.CampaignCreateInput) {
  return prisma.campaign.create({ data });
}

export async function updateStatus(campaignId: bigint, status: string) {
  return prisma.campaign.update({
    where: { campaignId },
    data: { status },
  });
}
