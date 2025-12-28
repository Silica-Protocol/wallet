import { Component, Input, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { GovernanceService } from '../../core/services/governance.service';
import { ProposalDetail, VoteInfo } from '../../core/types/wallet.types';

@Component({
  selector: 'app-proposal-detail',
  standalone: true,
  imports: [CommonModule, ReactiveFormsModule],
  templateUrl: './proposal-detail.html',
  styleUrl: './proposal-detail.scss'
})
export class ProposalDetailComponent implements OnInit {
  @Input() proposalId!: number;

  private governanceService = inject(GovernanceService);
  private formBuilder = inject(FormBuilder);

  // Data
  proposal: ProposalDetail | null = null;
  proposalVotes: VoteInfo[] = [];
  userVotingPower = 0;

  // UI State
  isLoading = false;
  isVoting = false;
  selectedTab: 'overview' | 'votes' = 'overview';

  // Forms
  voteForm: FormGroup;

  constructor() {
    this.voteForm = this.formBuilder.group({
      voterAddress: ['', [Validators.required, Validators.pattern(/^chert_[a-zA-Z0-9]+$/)]],
      support: ['', Validators.required],
      reason: ['']
    });
  }

  ngOnInit() {
    this.loadProposal();
    this.voteForm.get('voterAddress')?.valueChanges.subscribe(() => {
      this.loadVotingPower();
    });
  }

  private async loadProposal() {
    if (!this.proposalId) return;

    this.isLoading = true;
    try {
      this.governanceService.getProposal(this.proposalId).subscribe({
        next: (proposal) => {
          this.proposal = proposal;
          this.loadProposalVotes();
          this.isLoading = false;
        },
        error: (error) => {
          console.error('Failed to load proposal:', error);
          this.isLoading = false;
        }
      });
    } catch (error) {
      console.error('Failed to load proposal:', error);
      this.isLoading = false;
    }
  }

  private async loadProposalVotes() {
    if (!this.proposalId) return;

    try {
      this.governanceService.getProposalVotes(this.proposalId).subscribe({
        next: (votes) => {
          this.proposalVotes = votes;
        },
        error: (error) => {
          console.error('Failed to load proposal votes:', error);
        }
      });
    } catch (error) {
      console.error('Failed to load proposal votes:', error);
    }
  }

  private async loadVotingPower() {
    const voterControl = this.voteForm.get('voterAddress');
    const userAddress = voterControl?.value?.toString().trim();
    if (!userAddress || voterControl?.invalid) {
      this.userVotingPower = 0;
      return;
    }

    try {
      this.governanceService.getVotingPower(userAddress).subscribe({
        next: (power) => {
          this.userVotingPower = power.total_power;
        },
        error: (error) => {
          console.error('Failed to load voting power:', error);
          alert('Failed to load voting power. Governance features may not be available yet.');
        }
      });
    } catch (error) {
      console.error('Failed to load voting power:', error);
      alert('Failed to load voting power. Please try again later.');
    }
  }

  selectTab(tab: 'overview' | 'votes') {
    this.selectedTab = tab;
  }

  async castVote() {
    if (this.voteForm.invalid || !this.proposal) return;

    const { support, reason } = this.voteForm.value;
    const voterAddress = this.voteForm.get('voterAddress')?.value;

    this.isVoting = true;
    try {
      await this.governanceService.castVote({
        proposal_id: this.proposal.proposal_id,
        voter: voterAddress,
        support,
        reason: reason || undefined
      }).toPromise();

      // Reload proposal and votes after voting
      await this.loadProposal();
      this.voteForm.reset();
    } catch (error) {
      console.error('Failed to cast vote:', error);
    } finally {
      this.isVoting = false;
    }
  }

  canVote(): boolean {
    if (!this.proposal || this.userVotingPower === 0) return false;
    if (this.voteForm.get('voterAddress')?.invalid) return false;

    const now = Math.floor(Date.now() / 1000);
    return now >= this.proposal.vote_start &&
           now <= this.proposal.vote_end &&
           this.proposal.state === 'Active' &&
           !this.proposal.has_voted;
  }

  getVoteTypeText(support: number): string {
    return this.governanceService.getVoteTypeText(support);
  }

  getVoteTypeClass(support: number): string {
    switch (support) {
      case 0: return 'against';
      case 1: return 'for';
      case 2: return 'abstain';
      default: return 'unknown';
    }
  }

  getStateBadgeClass(state: string): string {
    switch (state.toLowerCase()) {
      case 'active': return 'badge-active';
      case 'pending': return 'badge-pending';
      case 'succeeded': return 'badge-succeeded';
      case 'defeated': return 'badge-defeated';
      case 'queued': return 'badge-queued';
      case 'executed': return 'badge-executed';
      case 'cancelled': return 'badge-cancelled';
      case 'vetoed': return 'badge-vetoed';
      case 'expired': return 'badge-expired';
      default: return 'badge-default';
    }
  }

  getSupportPercentage(): number {
    if (!this.proposal) return 0;
    const total = this.proposal.votes_for + this.proposal.votes_against + this.proposal.votes_abstain;
    if (total === 0) return 0;
    return (this.proposal.votes_for / total) * 100;
  }

  getQuorumProgress(): number {
    if (!this.proposal) return 0;
    const totalVotes = this.proposal.votes_for + this.proposal.votes_against + this.proposal.votes_abstain;
    // Assuming 4% quorum for demonstration
    const quorumThreshold = 4000000; // 4% of 100M supply
    return Math.min((totalVotes / quorumThreshold) * 100, 100);
  }

  getTimeRemaining(): string {
    if (!this.proposal) return '';

    const now = Math.floor(Date.now() / 1000);
    if (now < this.proposal.vote_start) {
      return `Voting starts in ${this.governanceService.getTimeUntilVoting(this.proposal.vote_start)}`;
    } else if (now <= this.proposal.vote_end) {
      const remaining = this.proposal.vote_end - now;
      const days = Math.floor(remaining / 86400);
      const hours = Math.floor((remaining % 86400) / 3600);
      if (days > 0) return `${days}d ${hours}h left`;
      return `${hours}h left`;
    } else {
      return 'Voting ended';
    }
  }

  formatTimestamp(timestamp: number): string {
    return this.governanceService.formatTimestamp(timestamp);
  }

  truncateAddress(address: string): string {
    if (address.length <= 12) return address;
    return address.substring(0, 6) + '...' + address.substring(address.length - 4);
  }

  formatCalldata(calldata: string): string {
    // Basic formatting for contract call data
    if (calldata.length > 66) {
      return calldata.substring(0, 10) + '...' + calldata.substring(calldata.length - 8);
    }
    return calldata;
  }
}