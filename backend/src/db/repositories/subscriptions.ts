import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findBySubscriber(subscriber: string) {
  return prisma.subscription.findMany({
    where: { subscriber },
    orderBy: { startedAt: 'desc' },
  });
}

export async function findActive(subscriber: string) {
  return prisma.subscription.findFirst({
    where: { subscriber, expiresAt: { gt: new Date() } },
    orderBy: { expiresAt: 'desc' },
  });
}

export async function create(data: Prisma.SubscriptionCreateInput) {
  return prisma.subscription.create({ data });
}
