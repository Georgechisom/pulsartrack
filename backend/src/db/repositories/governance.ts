import prisma from '../prisma';
import { Prisma } from '@prisma/client';

export async function findMany(filter?: { status?: string }, limit = 20) {
  return prisma.governanceProposal.findMany({
    where: filter?.status ? { status: filter.status } : undefined,
    orderBy: { createdAt: 'desc' },
    take: limit,
  });
}

export async function findByProposalId(proposalId: bigint) {
  return prisma.governanceProposal.findUnique({ where: { proposalId } });
}

export async function create(data: Prisma.GovernanceProposalCreateInput) {
  return prisma.governanceProposal.create({ data });
}

export async function recordVote(proposalId: bigint, vote: 'for' | 'against' | 'abstain') {
  const field =
    vote === 'for' ? 'votesFor' : vote === 'against' ? 'votesAgainst' : 'votesAbstain';
  return prisma.governanceProposal.update({
    where: { proposalId },
    data: { [field]: { increment: 1 } },
  });
}
