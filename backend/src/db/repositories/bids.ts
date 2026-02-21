import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findByAuction(auctionId: bigint) {
  return prisma.bid.findMany({
    where: { auctionId },
    orderBy: { amountStroops: 'desc' },
  });
}

export async function findByBidder(bidder: string, limit = 20) {
  return prisma.bid.findMany({
    where: { bidder },
    orderBy: { timestamp: 'desc' },
    take: limit,
  });
}

export async function create(data: Prisma.BidCreateInput) {
  return prisma.bid.create({ data });
}
