import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findMany(filter?: { status?: string }, limit = 20) {
  return prisma.auction.findMany({
    where: filter?.status ? { status: filter.status } : undefined,
    orderBy: { startTime: 'desc' },
    take: limit,
  });
}

export async function findByAuctionId(auctionId: bigint) {
  return prisma.auction.findUnique({ where: { auctionId } });
}

export async function create(data: Prisma.AuctionCreateInput) {
  return prisma.auction.create({ data });
}

export async function incrementBidCount(auctionId: bigint) {
  return prisma.auction.update({
    where: { auctionId },
    data: { bidCount: { increment: 1 } },
  });
}

export async function settle(auctionId: bigint, winner: string, winningBid: bigint) {
  return prisma.auction.update({
    where: { auctionId },
    data: {
      winner,
      winningBidStroops: winningBid,
      status: 'Settled',
      settledAt: new Date(),
    },
  });
}
