import { Injectable, inject } from '@angular/core';
import { Observable, from, map } from 'rxjs';
import { WALLET_BACKEND } from './wallet-backend.interface';
import {
  CastVoteRequest,
  CastVoteResponse,
  DelegateRequest,
  DelegateResponse,
  GetDelegationsResponse,
  GetProposalResponse,
  GetProposalsRequest,
  GetProposalsResponse,
  GetProposalVotesResponse,
  GetVotingPowerResponse,
  ProposalDetail,
  ProposalSummary,
  VoteInfo,
  VotingPowerInfo,
  DelegationInfo
} from '../types/wallet.types';

@Injectable({
  providedIn: 'root'
})
export class GovernanceService {
  private walletBackend = inject(WALLET_BACKEND);

  getProposals(request?: GetProposalsRequest): Observable<ProposalSummary[]> {
    return from(this.walletBackend.getProposals(request)).pipe(
      map((response: unknown) => (response as GetProposalsResponse).proposals)
    );
  }

  getProposal(proposalId: number): Observable<ProposalDetail> {
    return from(this.walletBackend.getProposal(proposalId)).pipe(
      map((response: unknown) => (response as GetProposalResponse).proposal)
    );
  }

  getProposalVotes(proposalId: number, limit?: number, offset?: number): Observable<VoteInfo[]> {
    return from(this.walletBackend.getProposalVotes(proposalId, limit, offset)).pipe(
      map((response: unknown) => (response as GetProposalVotesResponse).votes)
    );
  }

  getVotingPower(address: string): Observable<VotingPowerInfo> {
    return from(this.walletBackend.getVotingPower(address)).pipe(
      map((response: unknown) => (response as GetVotingPowerResponse).voting_power)
    );
  }

  getDelegations(address: string): Observable<DelegationInfo[]> {
    return from(this.walletBackend.getDelegations(address)).pipe(
      map((response: unknown) => {
        const payload = (response as GetDelegationsResponse).delegations;
        return payload.map((delegation) => {
          const timestamp = typeof delegation.delegated_at === 'string'
            ? Math.floor(new Date(delegation.delegated_at).getTime() / 1000)
            : delegation.delegated_at;

          return {
            ...delegation,
            delegated_at: timestamp,
            shares: delegation.shares ?? delegation.amount,
            rewards_claimed: delegation.rewards_claimed ?? 0
          };
        });
      })
    );
  }

  castVote(request: CastVoteRequest): Observable<VoteInfo> {
    return from(this.walletBackend.castVote(request)).pipe(
      map((response: unknown) => {
        const result = response as CastVoteResponse;
        return {
          proposal_id: Number(result.proposal_id),
          voter: result.voter,
          support: result.approve ? 1 : 0,
          weight: result.vote_weight,
          reason: undefined,
          voted_at: Math.floor(Date.now() / 1000)
        } as VoteInfo;
      })
    );
  }

  delegate(request: DelegateRequest): Observable<DelegationInfo> {
    return from(this.walletBackend.delegate(request)).pipe(
      map((response: unknown) => (response as DelegateResponse).delegation)
    );
  }

  // Helper methods
  getProposalStateText(state: string): string {
    switch (state.toLowerCase()) {
      case 'pending': return 'Pending';
      case 'active': return 'Active';
      case 'succeeded': return 'Succeeded';
      case 'defeated': return 'Defeated';
      case 'queued': return 'Queued';
      case 'executed': return 'Executed';
      case 'cancelled': return 'Cancelled';
      case 'vetoed': return 'Vetoed';
      case 'expired': return 'Expired';
      default: return state;
    }
  }

  getVoteTypeText(support: number): string {
    switch (support) {
      case 0: return 'Against';
      case 1: return 'For';
      case 2: return 'Abstain';
      default: return 'Unknown';
    }
  }

  formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  isProposalActive(voteStart: number, voteEnd: number): boolean {
    const now = Math.floor(Date.now() / 1000);
    return now >= voteStart && now <= voteEnd;
  }

  getTimeUntilVoting(voteStart: number): string {
    const now = Math.floor(Date.now() / 1000);
    const diff = voteStart - now;

    if (diff <= 0) return 'Voting started';

    const days = Math.floor(diff / 86400);
    const hours = Math.floor((diff % 86400) / 3600);
    const minutes = Math.floor((diff % 3600) / 60);

    if (days > 0) return `${days}d ${hours}h`;
    if (hours > 0) return `${hours}h ${minutes}m`;
    return `${minutes}m`;
  }
}